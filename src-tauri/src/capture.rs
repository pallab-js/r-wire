use crate::dissector;
use crate::model::PacketSummary;
use crate::state::FlowTable;
use rusqlite::Connection;
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::Manager;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::Instant;

const MAX_PACKET_COUNT: u64 = 5_000_000;
// Thresholds are for *this process* (RSS), matching AGENTS.md.
const MEMORY_WARNING_THRESHOLD: u64 = 512 * 1024 * 1024;
const MEMORY_CRITICAL_THRESHOLD: u64 = 768 * 1024 * 1024;

/// libpcap timestamps are (seconds, **microseconds**); we store nanoseconds.
pub fn ns_from_parts(tv_sec: i64, tv_usec: i64) -> i64 {
    tv_sec * 1_000_000_000 + tv_usec * 1_000
}

/// Packets kept waiting for a database write. Bounds memory while the database
/// is unavailable (disk full, locked, corrupt) instead of queueing forever.
const MAX_DB_QUEUE: usize = 5_000;

/// Writes `packets` to SQLite in one transaction.
///
/// The transaction is rolled back on any failure (the `tx` is dropped without
/// a commit), so the caller can safely retry the whole batch.
fn persist_packets(
    db: &mut Connection,
    packets: &[(PacketSummary, Vec<u8>)],
) -> Result<(), String> {
    if packets.is_empty() {
        return Ok(());
    }

    let tx = db
        .transaction()
        .map_err(|e| format!("begin transaction: {e}"))?;

    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO packets (id, timestamp_ns, source_addr, dest_addr, \
                 protocol, length, info, src_port, dst_port, data) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .map_err(|e| format!("prepare insert: {e}"))?;

        for (summary, data) in packets {
            stmt.execute(rusqlite::params![
                summary.id as i64,
                summary.timestamp,
                summary.source_addr,
                summary.dest_addr,
                summary.protocol,
                summary.length,
                summary.info,
                summary.src_port,
                summary.dst_port,
                data
            ])
            .map_err(|e| format!("insert packet {}: {e}", summary.id))?;
        }
    }

    tx.commit().map_err(|e| format!("commit: {e}"))
}

/// Writes the queued packets to SQLite, clearing the queue **only on success**.
///
/// The previous flow cleared `db_batch` whenever the lock was taken, so a
/// failed insert (database full, locked, corrupt) silently dropped up to
/// `DB_BATCH_SIZE` packets — and a poisoned lock dropped the flush entirely.
/// Failures are now logged and retried on the next tick, and the queue is
/// bounded so a database that stays unavailable cannot exhaust memory.
fn flush_db_batch(
    db_conn: &Mutex<Connection>,
    db_batch: &mut Vec<(PacketSummary, Vec<u8>)>,
    context: &str,
) {
    if db_batch.is_empty() {
        return;
    }

    let outcome = match db_conn.lock() {
        Ok(mut db) => persist_packets(&mut db, db_batch),
        Err(_) => Err("packet database lock is poisoned".to_string()),
    };

    match outcome {
        Ok(()) => db_batch.clear(),
        Err(e) => {
            log::error!(
                "[{context}] {} packets not written ({e}); queued for retry",
                db_batch.len()
            );
            trim_db_queue(db_batch, context);
        }
    }
}

/// Drops the oldest queued packets once the retry queue grows too large.
fn trim_db_queue(db_batch: &mut Vec<(PacketSummary, Vec<u8>)>, context: &str) {
    if db_batch.len() > MAX_DB_QUEUE {
        let overflow = db_batch.len() - MAX_DB_QUEUE;
        db_batch.drain(..overflow);
        log::warn!(
            "[{context}] packet database unavailable; dropped {overflow} oldest \
             queued packets to bound memory"
        );
    }
}

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

    // (packet id, captured bytes, timestamp, length on the wire)
    let (packet_tx, packet_rx) = std_mpsc::channel::<(u64, Vec<u8>, i64, u32)>();

    let cap_handle = std::thread::spawn(move || {
        let mut cap = cap;
        let mut id_counter: u64 = 0;

        loop {
            match cap.next_packet() {
                Ok(packet) => {
                    id_counter += 1;
                    let data = packet.data.to_vec();
                    let timestamp_ns =
                        ns_from_parts(packet.header.ts.tv_sec, packet.header.ts.tv_usec as i64);
                    // `header.len` is the frame's length on the wire; `data` is
                    // cut short at the 1600-byte snaplen.
                    if packet_tx
                        .send((id_counter, data, timestamp_ns, packet.header.len))
                        .is_err()
                    {
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
    // Throttle the memory probe: `total_packets_captured.is_multiple_of(500)`
    // is true at 0 packets, which would otherwise sample every 10ms while idle.
    let mut last_memory_check = Instant::now();
    let mut auto_stop_triggered = false;
    let mut sys = System::new();
    let self_pid = Pid::from(std::process::id() as usize);
    const BATCH_SIZE: usize = 100;
    const DB_BATCH_SIZE: usize = 500;
    const BATCH_TIMEOUT_MS: u64 = 100;

    loop {
        tokio::select! {
            _ = stop_rx.recv() => {
                flush_db_batch(&db_conn, &mut db_batch, "capture stop");
                if !batch.is_empty() {
                    let _ = app_handle.emit_all("new_packet_batch", &batch);
                    batch.clear();
                }
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                loop {
                    match packet_rx.try_recv() {
                        Ok((packet_id, packet_data, timestamp_ns, original_len)) => {
                            total_packets_captured += 1;

                            if total_packets_captured > MAX_PACKET_COUNT || auto_stop_triggered {
                                if !auto_stop_triggered {
                                    log::warn!("Maximum packet count ({}) reached. Stopping capture.", MAX_PACKET_COUNT);
                                    auto_stop_triggered = true;
                                    let _ = app_handle.emit_all(
                                        "capture_auto_stop",
                                        format!("Reached {} packets", MAX_PACKET_COUNT),
                                    );
                                }
                                break;
                            }

                            if let Some(mut summary) = dissector::parse_summary(&packet_data, packet_id, timestamp_ns) {
                                // Report the length on the wire, not just the
                                // bytes that survived the snaplen — a truncated
                                // 9000-byte frame is still 9000 bytes long.
                                summary.length = original_len.max(summary.length);

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
                            flush_db_batch(&db_conn, &mut db_batch, "capture channel closed");
                            if !batch.is_empty() {
                                let _ = app_handle.emit_all("new_packet_batch", &batch);
                            }
                            return Ok(());
                        }
                    }
                }

                if db_batch.len() >= DB_BATCH_SIZE {
                    flush_db_batch(&db_conn, &mut db_batch, "capture batch");
                }

                if last_memory_check.elapsed().as_secs() >= 1 {
                    last_memory_check = Instant::now();
                    sys.refresh_processes(ProcessesToUpdate::Some(&[self_pid]), false);
                    let used_memory = sys.process(self_pid).map(|p| p.memory()).unwrap_or(0);

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
                    flush_db_batch(&db_conn, &mut db_batch, "capture emit");
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

    // The auto-stop path breaks out of the loop without passing through the
    // stop_rx flush, so anything still batched would otherwise be lost.
    flush_db_batch(&db_conn, &mut db_batch, "capture shutdown");
    if !batch.is_empty() {
        let _ = app_handle.emit_all("new_packet_batch", &batch);
        batch.clear();
    }

    // The capture thread only exits when its `send()` fails, i.e. when the
    // receiver is dropped. Dropping it *before* join() lets the thread observe
    // the disconnect and terminate instead of blocking here forever.
    drop(packet_rx);
    let _ = cap_handle.join();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    const SCHEMA: &str = "CREATE TABLE packets (
            id INTEGER PRIMARY KEY,
            timestamp_ns INTEGER NOT NULL,
            source_addr TEXT,
            dest_addr TEXT,
            protocol TEXT,
            length INTEGER,
            info TEXT,
            src_port INTEGER,
            dst_port INTEGER,
            data BLOB NOT NULL
        )";

    fn queued(id: u64) -> (PacketSummary, Vec<u8>) {
        (
            PacketSummary {
                id,
                timestamp: id as i64,
                source_addr: "10.0.0.1".to_string(),
                dest_addr: "10.0.0.2".to_string(),
                protocol: "TCP".to_string(),
                length: 60,
                info: String::new(),
                src_port: Some(1234),
                dst_port: Some(80),
            },
            vec![0u8; 8],
        )
    }

    #[test]
    fn a_successful_write_clears_the_queue() {
        let db = Mutex::new(Connection::open_in_memory().expect("in-memory db"));
        db.lock().unwrap().execute(SCHEMA, []).expect("schema");

        let mut queue = vec![queued(1), queued(2)];
        flush_db_batch(&db, &mut queue, "test");

        assert!(queue.is_empty(), "written packets must leave the queue");
        let count: i64 = db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM packets", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 2);
    }

    #[test]
    fn a_failed_write_keeps_the_queue_for_retry() {
        // No `packets` table, so the insert fails the way a full or corrupt
        // database would. Regression: the batch used to be cleared whenever
        // the lock was taken, silently dropping every queued packet.
        let db = Mutex::new(Connection::open_in_memory().expect("in-memory db"));

        let mut queue = vec![queued(1), queued(2)];
        flush_db_batch(&db, &mut queue, "test");

        assert_eq!(queue.len(), 2, "a failed insert must not drop packets");
    }

    #[test]
    fn a_poisoned_lock_keeps_the_queue_for_retry() {
        let db = Mutex::new(Connection::open_in_memory().expect("in-memory db"));
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _guard = db.lock().expect("lock");
            panic!("poison the packet database lock");
        }));
        assert!(panicked.is_err());
        assert!(db.lock().is_err(), "the lock should now be poisoned");

        let mut queue = vec![queued(1)];
        flush_db_batch(&db, &mut queue, "test");

        assert_eq!(queue.len(), 1, "a poisoned lock must not drop packets");
    }

    #[test]
    fn the_retry_queue_is_bounded() {
        // A database that never comes back must not queue packets forever.
        let db = Mutex::new(Connection::open_in_memory().expect("in-memory db"));
        let mut queue: Vec<_> = (0..=MAX_DB_QUEUE as u64).map(queued).collect();

        flush_db_batch(&db, &mut queue, "test");

        assert_eq!(queue.len(), MAX_DB_QUEUE);
        assert_eq!(queue[0].0.id, 1, "the oldest packets are the ones dropped");
    }
}
