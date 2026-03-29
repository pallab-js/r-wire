use crate::dissector;
use crate::model::PacketSummary;
use std::sync::{Arc, Mutex, mpsc as std_mpsc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::Instant;
use tauri::Manager;
use rusqlite::Connection;

pub async fn run_capture(
    app_handle: tauri::AppHandle,
    interface_name: String,
    mut stop_rx: tokio_mpsc::Receiver<()>,
    db_conn: Arc<Mutex<Connection>>,
) -> Result<(), String> {
    log::info!("Starting packet capture on interface: {}", interface_name);

    // Open the capture device
    let cap = pcap::Capture::from_device(interface_name.as_str())
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("permission") {
                log::error!("Permission denied when opening device: {}", interface_name);
                "PermissionError".to_string()
            } else {
                log::error!("Failed to open device {}: {}", interface_name, e);
                format!("Failed to open device: {}", e)
            }
        })?
        .promisc(true)
        .snaplen(65535)
        .timeout(1000)
        .open()
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("permission") {
                log::error!("Permission denied when activating capture on: {}", interface_name);
                "PermissionError".to_string()
            } else {
                log::error!("Failed to activate capture on {}: {}", interface_name, e);
                format!("Failed to activate capture: {}", e)
            }
        })?;

    log::info!("Successfully opened capture device: {}", interface_name);

    let (packet_tx, packet_rx) = std_mpsc::channel::<(u64, Vec<u8>, i64)>();
    
    // Spawn blocking thread for packet capture
    let cap_handle = std::thread::spawn(move || {
        let mut cap = cap;
        let mut id_counter: u64 = 0;
        
        loop {
            match cap.next_packet() {
                Ok(packet) => {
                    id_counter += 1;
                    let data = packet.data.to_vec();
                    let timestamp_ns = packet.header.ts.tv_sec * 1_000_000_000
                        + (packet.header.ts.tv_usec as i64) * 1_000_000;
                    if packet_tx.send((id_counter, data, timestamp_ns)).is_err() {
                        break;
                    }
                }
                Err(pcap::Error::TimeoutExpired) => continue,
                Err(e) => {
                    eprintln!("Packet capture error: {}", e);
                    break;
                }
            }
        }
    });

    let mut batch: Vec<PacketSummary> = Vec::new();
    let mut db_batch: Vec<(u64, Vec<u8>, i64)> = Vec::new();
    let mut last_emit = Instant::now();
    const BATCH_SIZE: usize = 50;
    const DB_BATCH_SIZE: usize = 200;
    const BATCH_TIMEOUT_MS: u64 = 250;

    let insert_packets = |db: &mut Connection, packets: &Vec<(u64, Vec<u8>, i64)>| {
        if packets.is_empty() { return; }
        if let Ok(tx) = db.transaction() {
            {
                let mut stmt = tx.prepare_cached("INSERT INTO packets (id, timestamp_ns, data) VALUES (?1, ?2, ?3)").unwrap();
                for (id, data, ts) in packets {
                    let id_i64 = *id as i64;
                    let _ = stmt.execute(rusqlite::params![id_i64, ts, data]);
                }
            }
            let _ = tx.commit();
        }
    };

    loop {
        tokio::select! {
            _ = stop_rx.recv() => {
                if !db_batch.is_empty() {
                    if let Ok(mut db) = db_conn.lock() {
                        insert_packets(&mut db, &db_batch);
                    }
                }
                if !batch.is_empty() {
                    let _ = app_handle.emit_all("new_packet_batch", &batch);
                    batch.clear();
                }
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                loop {
                    match packet_rx.try_recv() {
                        Ok((packet_id, packet_data, timestamp_ns)) => {
                            if let Some(summary) = dissector::parse_summary(&packet_data, packet_id, timestamp_ns) {
                                db_batch.push((packet_id, packet_data, timestamp_ns));
                                batch.push(summary);
                            }
                        }
                        Err(std_mpsc::TryRecvError::Empty) => break,
                        Err(std_mpsc::TryRecvError::Disconnected) => {
                            if !db_batch.is_empty() {
                                if let Ok(mut db) = db_conn.lock() {
                                    insert_packets(&mut db, &db_batch);
                                }
                            }
                            if !batch.is_empty() {
                                let _ = app_handle.emit_all("new_packet_batch", &batch);
                            }
                            return Ok(());
                        }
                    }
                }

                if db_batch.len() >= DB_BATCH_SIZE {
                    if let Ok(mut db) = db_conn.lock() {
                        insert_packets(&mut db, &db_batch);
                        db_batch.clear();
                    }
                }

                let should_emit = batch.len() >= BATCH_SIZE || 
                    last_emit.elapsed().as_millis() >= BATCH_TIMEOUT_MS as u128;
                
                if should_emit && !batch.is_empty() {
                    if !db_batch.is_empty() {
                        if let Ok(mut db) = db_conn.lock() {
                            insert_packets(&mut db, &db_batch);
                            db_batch.clear();
                        }
                    }
                    if let Err(e) = app_handle.emit_all("new_packet_batch", &batch) {
                        eprintln!("Failed to emit batch: {}", e);
                    }
                    batch.clear();
                    last_emit = Instant::now();
                }
            }
        }
    }

    let _ = cap_handle.join();
    Ok(())
}
