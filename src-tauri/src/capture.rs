use crate::dissector;
use crate::model::{PacketSummary, CachedPacket};
use std::sync::{Arc, Mutex, mpsc as std_mpsc};
use std::collections::BTreeMap;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::Instant;
use tauri::Manager;

pub async fn run_capture(
    app_handle: tauri::AppHandle,
    interface_name: String,
    mut stop_rx: tokio_mpsc::Receiver<()>,
    packet_cache: Arc<Mutex<BTreeMap<u64, CachedPacket>>>,
) -> Result<(), String> {
    // Open the capture device
    let cap = pcap::Capture::from_device(interface_name.as_str())
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("permission") {
                "PermissionError".to_string()
            } else {
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
                "PermissionError".to_string()
            } else {
                format!("Failed to activate capture: {}", e)
            }
        })?;

    // Create a channel for packets from the blocking thread (use std::sync::mpsc)
    // Send (packet_id, packet_data, timestamp_ns)
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
                    // Extract timestamp from packet header
                    // pcap header has tv_sec (seconds) and tv_usec (microseconds)
                    let timestamp_ns = (packet.header.ts.tv_sec as i64) * 1_000_000_000 
                        + (packet.header.ts.tv_usec as i64) * 1_000;
                    // Send packet with timestamp (blocking, but that's ok in this thread)
                    if packet_tx.send((id_counter, data, timestamp_ns)).is_err() {
                        // Receiver dropped, stop capturing
                        break;
                    }
                }
                Err(pcap::Error::TimeoutExpired) => {
                    // Timeout is normal, continue
                    continue;
                }
                Err(e) => {
                    eprintln!("Packet capture error: {}", e);
                    break;
                }
            }
        }
    });

    let mut batch: Vec<PacketSummary> = Vec::new();
    let mut last_emit = Instant::now();
    const BATCH_SIZE: usize = 50;
    const BATCH_TIMEOUT_MS: u64 = 250;

    loop {
        tokio::select! {
            // Check for stop signal
            _ = stop_rx.recv() => {
                // Emit any remaining packets in the batch before stopping
                if !batch.is_empty() {
                    if let Err(e) = app_handle.emit_all("new_packet_batch", &batch) {
                        eprintln!("Failed to emit final batch: {}", e);
                    }
                    batch.clear();
                }
                break;
            }
            // Receive packets with timeout to allow checking stop signal
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                // Try to receive packets (non-blocking)
                loop {
                    match packet_rx.try_recv() {
                        Ok((packet_id, packet_data, timestamp_ns)) => {
                            // Parse the packet summary with actual timestamp
                            if let Some(summary) = dissector::parse_summary(&packet_data, packet_id, timestamp_ns) {
                                // Store the full raw packet in cache with timestamp
                                {
                                    if let Ok(mut cache) = packet_cache.lock() {
                                        cache.insert(packet_id, CachedPacket {
                                            data: packet_data,
                                            timestamp_ns,
                                        });
                                        
                                        // Limit cache size to prevent unbounded memory growth
                                        // BTreeMap maintains sorted order, so we can efficiently remove oldest
                                        const MAX_CACHE_SIZE: usize = 100_000;
                                        if cache.len() > MAX_CACHE_SIZE {
                                            // Remove oldest packets (lowest IDs) - O(k log n) where k is items to remove
                                            let to_remove = cache.len() - MAX_CACHE_SIZE;
                                            let keys_to_remove: Vec<u64> = cache.keys().take(to_remove).cloned().collect();
                                            for key in keys_to_remove {
                                                cache.remove(&key);
                                            }
                                        }
                                    } else {
                                        eprintln!("Failed to lock packet cache for insertion");
                                    }
                                }
                                
                                // Add to batch
                                batch.push(summary);
                            }
                        }
                        Err(std_mpsc::TryRecvError::Empty) => {
                            // No more packets available right now
                            break;
                        }
                        Err(std_mpsc::TryRecvError::Disconnected) => {
                            // Channel closed, capture thread ended
                            if !batch.is_empty() {
                                if let Err(e) = app_handle.emit_all("new_packet_batch", &batch) {
                                    eprintln!("Failed to emit batch on disconnect: {}", e);
                                }
                            }
                            return Ok(());
                        }
                    }
                }

                // Check if we should emit the batch
                let should_emit = batch.len() >= BATCH_SIZE || 
                    last_emit.elapsed().as_millis() >= BATCH_TIMEOUT_MS as u128;
                
                if should_emit && !batch.is_empty() {
                    if let Err(e) = app_handle.emit_all("new_packet_batch", &batch) {
                        eprintln!("Failed to emit batch: {}", e);
                    }
                    batch.clear();
                    last_emit = Instant::now();
                }
            }
        }
    }

    // Wait for capture thread to finish (it will when we break above)
    let _ = cap_handle.join();

    Ok(())
}
