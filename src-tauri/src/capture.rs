use crate::dissector;
use crate::model::PacketSummary;
use crate::state::FlowTable;
use rusqlite::Connection;
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use sysinfo::System;
use tauri::Manager;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::Instant;

const MAX_PACKET_COUNT: u64 = 5_000_000;
const MEMORY_WARNING_THRESHOLD: u64 = 7 * 1024 * 1024 * 1024;
const MEMORY_CRITICAL_THRESHOLD: u64 = 7_500 * 1024 * 1024;

pub async fn run_capture(
    app_handle: tauri::AppHandle,
    interface_name: String,
    filter: Option<String>,
    mut stop_rx: tokio_mpsc::Receiver<()>,
    db_conn: Arc<Mutex<Connection>>,
    flow_table: Arc<Mutex<FlowTable>>,
) -> Result<(), String> {
    log::info!(
        "Starting packet capture on interface: {} with filter: {:?}",
        interface_name,
        filter
    );

    let mut cap = pcap::Capture::from_device(interface_name.as_str())
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
        .snaplen(1600)
        .timeout(500)
        .open()
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("permission") {
                log::error!(
                    "Permission denied when activating capture on: {}",
                    interface_name
                );
                "PermissionError".to_string()
            } else {
                log::error!("Failed to activate capture on {}: {}", interface_name, e);
                format!("Failed to activate capture: {}", e)
            }
        })?;

    if let Some(f) = filter {
        if let Err(e) = cap.filter(&f, true) {
            log::error!("Failed to apply BPF filter '{}': {}", f, e);
            return Err(format!("Invalid BPF filter: {}", e));
        }
    }

    log::info!("Successfully opened capture device: {}", interface_name);

    let (packet_tx, packet_rx) = std_mpsc::channel::<(u64, Vec<u8>, i64)>();

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
    let mut db_batch: Vec<(PacketSummary, Vec<u8>)> = Vec::new();
    let mut last_emit = Instant::now();
    let mut total_packets_captured: u64 = 0;
    let mut memory_warning_logged = false;
    let mut auto_stop_triggered = false;
    let mut sys = System::new();
    const BATCH_SIZE: usize = 100;
    const DB_BATCH_SIZE: usize = 500;
    const BATCH_TIMEOUT_MS: u64 = 100;

    let insert_packets = |db: &mut Connection, packets: &Vec<(PacketSummary, Vec<u8>)>| {
        if packets.is_empty() {
            return;
        }
        match db.transaction() {
            Ok(tx) => {
                let mut success = true;
                {
                    match tx.prepare_cached("INSERT INTO packets (id, timestamp_ns, source_addr, dest_addr, protocol, length, info, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)") {
                        Ok(mut stmt) => {
                            for (summary, data) in packets {
                                let id_i64 = summary.id as i64;
                                if let Err(e) = stmt.execute(rusqlite::params![
                                    id_i64,
                                    summary.timestamp,
                                    summary.source_addr,
                                    summary.dest_addr,
                                    summary.protocol,
                                    summary.length,
                                    summary.info,
                                    data
                                ]) {
                                    log::error!("Failed to insert packet {}: {}", id_i64, e);
                                    success = false;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to prepare insert statement: {}", e);
                            success = false;
                        }
                    }
                }
                if success {
                    if let Err(e) = tx.commit() {
                        log::error!("Failed to commit transaction: {}", e);
                    }
                }
            }
            Err(e) => log::error!("Failed to start transaction: {}", e),
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
                            total_packets_captured += 1;

                            if total_packets_captured > MAX_PACKET_COUNT || auto_stop_triggered {
                                if !auto_stop_triggered {
                                    log::warn!("Maximum packet count ({}) reached. Stopping capture.", MAX_PACKET_COUNT);
                                    auto_stop_triggered = true;
                                }
                                break;
                            }

                            if let Some(summary) = dissector::parse_summary(&packet_data, packet_id, timestamp_ns) {
                                if let Some(key) = dissector::get_flow_key(&packet_data) {
                                    if let Ok(mut flows) = flow_table.lock() {
                                        flows.update(packet_id, timestamp_ns, summary.length, key);
                                    }
                                }

                                db_batch.push((summary.clone(), packet_data));
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

                if total_packets_captured.is_multiple_of(500) {
                    sys.refresh_memory();
                    let used_memory = sys.used_memory();

                    if used_memory > MEMORY_CRITICAL_THRESHOLD && !auto_stop_triggered {
                        log::warn!(
                            "Critical memory usage ({} bytes). Auto-stopping capture to prevent crash.",
                            used_memory
                        );
                        auto_stop_triggered = true;
                        let _ = app_handle.emit_all("capture_auto_stop", "Memory limit exceeded");
                    } else if used_memory > MEMORY_WARNING_THRESHOLD && !memory_warning_logged {
                        log::warn!(
                            "High memory usage detected ({} bytes / {} MB). Consider stopping capture.",
                            used_memory,
                            used_memory / (1024 * 1024)
                        );
                        memory_warning_logged = true;
                        let _ = app_handle.emit_all("memory_warning", used_memory);
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

        if auto_stop_triggered {
            break;
        }
    }

    let _ = cap_handle.join();
    Ok(())
}
