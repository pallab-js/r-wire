pub mod capture;
pub mod dissector;
pub mod export;
pub mod model;
pub mod state;

use rusqlite::Connection;
use state::FlowTable;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tauri::Manager;
use tokio::sync::mpsc;

type PacketBatchEntry = (
    i64,
    i64,
    String,
    String,
    String,
    i32,
    String,
    Option<u16>,
    Option<u16>,
    Vec<u8>,
);

// Initialize logging
#[cfg(not(debug_assertions))]
use log::LevelFilter;

/// Rate limiter for capture operations to prevent DoS.
pub struct CaptureRateLimiter {
    /// Start of the current one-minute window; `None` once it has expired.
    window_start: Mutex<Option<Instant>>,
    /// Time of the most recent capture attempt (enforces minimum spacing).
    last_capture_time: Mutex<Option<Instant>>,
    capture_count: AtomicUsize,
    max_captures_per_minute: usize,
    min_interval_seconds: u64,
}

impl CaptureRateLimiter {
    fn new() -> Self {
        Self {
            window_start: Mutex::new(None),
            last_capture_time: Mutex::new(None),
            capture_count: AtomicUsize::new(0),
            max_captures_per_minute: 10, // Max 10 captures per minute
            min_interval_seconds: 5,     // Min 5 seconds between captures
        }
    }

    fn check_rate_limit(&self) -> Result<(), String> {
        let now = Instant::now();

        // Roll the window forward when it expires, clearing the count.
        // Without this reset the counter only ever grows, and capture becomes
        // permanently disabled for the lifetime of the process.
        if let Ok(mut start) = self.window_start.lock() {
            if let Some(s) = *start {
                if now.duration_since(s).as_secs() >= 60 {
                    *start = None;
                    self.capture_count.store(0, Ordering::Relaxed);
                }
            }
        }

        // Enforce the minimum spacing between *consecutive* attempts.
        if let Ok(guard) = self.last_capture_time.lock() {
            if let Some(last) = *guard {
                let elapsed = now.duration_since(last).as_secs();
                if elapsed < self.min_interval_seconds {
                    return Err(format!(
                        "Rate limited: Please wait {} seconds before starting another capture",
                        self.min_interval_seconds - elapsed
                    ));
                }
            }
        }

        if self.capture_count.load(Ordering::Relaxed) >= self.max_captures_per_minute {
            return Err(
                "Rate limited: Too many capture attempts. Please wait a minute.".to_string(),
            );
        }

        Ok(())
    }

    fn record_capture(&self) {
        let now = Instant::now();

        if let Ok(mut last) = self.last_capture_time.lock() {
            *last = Some(now);
        }
        if let Ok(mut start) = self.window_start.lock() {
            if start.is_none() {
                *start = Some(now);
            }
        }
        self.capture_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Shared syntax checks for paths supplied by the renderer.
///
/// `Path::components()` is the only reliable way to spot traversal: a raw
/// `contains("..")` check passes `/tmp/a/../etc/x.pcap` because the `..` sits
/// between other characters, so the previous guard only ever caught a bare
/// `..` — which is not a traversal on its own.
fn parse_abs_path(file_path: &str) -> Result<&Path, String> {
    if file_path.contains('\0') {
        return Err("Invalid file path: contains null bytes".to_string());
    }

    let path = Path::new(file_path);

    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Path traversal detected: '..' not allowed".to_string());
    }

    // A relative path would land in the process CWD (undefined for a GUI app
    // launched from Finder), so refuse it rather than guess.
    if !path.is_absolute() {
        return Err("Invalid file path: must be an absolute path".to_string());
    }

    Ok(path)
}

fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|e| allowed.iter().any(|a| *a == e))
}

/// Validates export path for new files (doesn't need to exist yet).
fn validate_export_path_new(file_path: &str) -> Result<PathBuf, String> {
    let path = parse_abs_path(file_path)?;

    // Validate file extension is .pcap
    if !has_extension(path, &["pcap"]) {
        return Err("File must have .pcap extension".to_string());
    }

    // The save dialog resolves the directory up front; a missing parent means
    // the write would otherwise go somewhere unexpected or fail silently.
    match path.parent() {
        Some(parent) if parent.is_dir() => {}
        Some(_) => return Err("Destination directory does not exist".to_string()),
        None => return Err("Invalid file path: missing parent directory".to_string()),
    }

    // Never write through a symlink or on top of a device/fifo.
    if let Ok(meta) = path.symlink_metadata() {
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err("Refusing to overwrite a symlink or non-regular file".to_string());
        }
    }

    Ok(path.to_path_buf())
}

/// Validates network interface name to prevent injection attacks.
fn validate_interface_name(name: &str) -> Result<(), String> {
    // Interface names should only contain alphanumeric chars, hyphens, underscores, and dots
    if name.is_empty() || name.len() > 256 {
        return Err("Interface name must be between 1 and 256 characters".to_string());
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("Interface name contains invalid characters".to_string());
    }

    Ok(())
}

/// Validates BPF filter string to prevent command injection.
/// Validates a BPF filter by compiling it with libpcap before capture starts.
///
/// The previous check was a character blacklist. It rejected perfectly legal
/// filters — `tcp port 80 and (port 8080 or port 8443)`, `ip[0] & 0x0f`,
/// `len > 100` — because `(`, `&` and `>` were on the list, while letting
/// syntactically broken ones through to the capture task, where the error only
/// reached stderr and the UI went on showing a running capture that collected
/// nothing. A BPF expression never reaches a shell (libpcap compiles it in
/// process), so there is nothing here to escape: what matters is whether it
/// compiles.
fn validate_bpf_filter(filter: &str, interface_name: &str) -> Result<(), String> {
    // BPF filters should be reasonable length
    if filter.len() > 1024 {
        return Err("BPF filter too long (max 1024 characters)".to_string());
    }

    // Compile against a throwaway handle on the target interface. Opening a
    // handle does not start capturing and nothing is ever read from it.
    let mut cap = pcap::Capture::from_device(interface_name)
        .map_err(|e| format!("Cannot validate BPF filter: {}", e))?
        .open()
        .map_err(|e| format!("Cannot validate BPF filter: {}", e))?;

    cap.filter(filter, true)
        .map_err(|e| format!("Invalid BPF filter '{}': {}", filter, e))?;

    Ok(())
}

/// Validates packet IDs to prevent excessive resource usage.
fn validate_packet_ids(ids: &[u64]) -> Result<(), String> {
    if ids.is_empty() {
        return Err("No packet IDs provided".to_string());
    }

    if ids.len() > 100000 {
        return Err("Too many packet IDs (max 100,000)".to_string());
    }

    Ok(())
}

/// Validates pagination parameters to prevent DoS.
fn validate_pagination(offset: usize, limit: usize) -> Result<(), String> {
    if limit == 0 || limit > 10000 {
        return Err("Limit must be between 1 and 10,000".to_string());
    }

    if offset > 1000000 {
        return Err("Offset too large (max 1,000,000)".to_string());
    }

    Ok(())
}

/// Validates filter string to prevent SQL injection and excessive length.
fn validate_filter(filter: &str) -> Result<(), String> {
    if filter.len() > 500 {
        return Err("Filter string too long (max 500 characters)".to_string());
    }

    // Reject null bytes
    if filter.contains('\0') {
        return Err("Filter contains invalid characters".to_string());
    }

    Ok(())
}

pub struct AppState {
    // Sender to signal the capture task to stop
    pub stop_tx: Mutex<Option<mpsc::Sender<()>>>,
    // SQLite connection for packet storage
    pub db_conn: Arc<Mutex<Connection>>,
    // Global flow table for connection tracking
    pub flow_table: Arc<Mutex<FlowTable>>,
    // Rate limiter for capture operations
    pub rate_limiter: CaptureRateLimiter,
}

/// Retrieves a paginated list of packets, optionally filtered.
#[tauri::command]
async fn get_packets(
    offset: usize,
    limit: usize,
    filter: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<model::PacketSummary>, String> {
    // Validate pagination parameters
    validate_pagination(offset, limit)?;

    // Validate filter if provided
    if let Some(ref f) = filter {
        validate_filter(f)?;
    }

    let db = state
        .db_conn
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    let (where_clause, params) = if let Some(f) = filter {
        build_filter_clause(&f)
    } else {
        ("".to_string(), vec![])
    };

    let query = format!(
        "SELECT id, timestamp_ns, source_addr, dest_addr, protocol, length, info, src_port, dst_port FROM packets {} ORDER BY id ASC LIMIT ? OFFSET ?",
        where_clause
    );

    let mut stmt = db
        .prepare(&query)
        .map_err(|e| format!("Prepare failed: {}", e))?;

    // Convert Vec<String> to Vec<&dyn ToSql>
    let mut sql_params: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let limit_i64 = limit as i64;
    let offset_i64 = offset as i64;
    sql_params.push(&limit_i64);
    sql_params.push(&offset_i64);

    let packet_rows = stmt
        .query_map(&*sql_params, |row| {
            Ok(model::PacketSummary {
                id: row.get::<_, i64>(0)? as u64,
                timestamp: row.get(1)?,
                source_addr: row.get(2)?,
                dest_addr: row.get(3)?,
                protocol: row.get(4)?,
                length: row.get(5)?,
                info: row.get(6)?,
                src_port: row.get(7)?,
                dst_port: row.get(8)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut packets = Vec::new();
    for packet in packet_rows {
        packets.push(packet.map_err(|e| format!("Row mapping failed: {}", e))?);
    }

    Ok(packets)
}

/// Retrieves the total count of packets matching a filter.
#[tauri::command]
async fn get_packet_count(
    filter: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    // Validate filter if provided
    if let Some(ref f) = filter {
        validate_filter(f)?;
    }

    let db = state
        .db_conn
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    let (where_clause, params) = if let Some(f) = filter {
        build_filter_clause(&f)
    } else {
        ("".to_string(), vec![])
    };

    let query = format!("SELECT COUNT(*) FROM packets {}", where_clause);
    let mut stmt = db
        .prepare(&query)
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let sql_params: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let count: i64 = stmt
        .query_row(&*sql_params, |row| row.get(0))
        .map_err(|e| format!("Count failed: {}", e))?;

    Ok(count as usize)
}

fn build_filter_clause(filter: &str) -> (String, Vec<String>) {
    let filter = filter.to_lowercase();
    let filter = filter.trim();
    if filter.is_empty() {
        return ("".to_string(), vec![]);
    }

    if filter.starts_with("protocol:") {
        let val = filter.replace("protocol:", "").trim().to_string();
        return (
            "WHERE protocol LIKE ?".to_string(),
            vec![format!("%{}%", val)],
        );
    }
    if filter.starts_with("ip:") {
        let val = filter.replace("ip:", "").trim().to_string();
        return (
            "WHERE source_addr LIKE ? OR dest_addr LIKE ?".to_string(),
            vec![format!("%{}%", val), format!("%{}%", val)],
        );
    }
    if filter.starts_with("src:") {
        let val = filter.replace("src:", "").trim().to_string();
        return (
            "WHERE source_addr LIKE ?".to_string(),
            vec![format!("%{}%", val)],
        );
    }
    if filter.starts_with("dst:") {
        let val = filter.replace("dst:", "").trim().to_string();
        return (
            "WHERE dest_addr LIKE ?".to_string(),
            vec![format!("%{}%", val)],
        );
    }
    if filter.starts_with("port:") {
        let val = filter.replace("port:", "").trim().to_string();
        // Match the exact port on either side. Substring-matching `info`
        // made `port:80` also hit 8080, 1080, and `192.168.1.80`.
        return match val.parse::<u16>() {
            Ok(port) => (
                "WHERE src_port = ? OR dst_port = ?".to_string(),
                vec![port.to_string(), port.to_string()],
            ),
            Err(_) => ("WHERE 0 = 1".to_string(), vec![]),
        };
    }

    // General search
    (
        "WHERE protocol LIKE ? OR source_addr LIKE ? OR dest_addr LIKE ? OR info LIKE ? OR CAST(length AS TEXT) LIKE ?".to_string(),
        vec![
            format!("%{}%", filter),
            format!("%{}%", filter),
            format!("%{}%", filter),
            format!("%{}%", filter),
            format!("%{}%", filter),
        ]
    )
}

/// Lists all available network interfaces for packet capture.
#[tauri::command(async)]
fn list_interfaces() -> Result<Vec<String>, String> {
    match pcap::Device::list() {
        Ok(devices) => Ok(devices.iter().map(|d| d.name.clone()).collect()),
        Err(e) => Err(format!("Failed to list interfaces: {}", e)),
    }
}

/// Starts packet capture on the specified network interface.
#[tauri::command]
async fn start_capture(
    interface_name: String,
    filter: Option<String>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Validate interface name
    validate_interface_name(&interface_name)?;

    // Validate BPF filter if provided
    if let Some(ref f) = filter {
        validate_bpf_filter(f, &interface_name)?;
    }

    // Check rate limit
    state.rate_limiter.check_rate_limit()?;

    // Check if already capturing
    let mut stop_tx_guard = state
        .stop_tx
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    if stop_tx_guard.is_some() {
        return Err("Capture already in progress".to_string());
    }

    // Create channel for stop signal
    let (stop_tx, stop_rx) = mpsc::channel(1);
    *stop_tx_guard = Some(stop_tx);
    drop(stop_tx_guard); // Release lock early

    // Clear packet table and flow table so a restart begins from a clean slate
    purge_capture_data(&state)?;

    // Clone the DB Arc for the task
    let db_conn = Arc::clone(&state.db_conn);
    let flow_table = Arc::clone(&state.flow_table);

    // Spawn the capture task
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        let result = capture::run_capture(
            app_handle_clone.clone(),
            interface_name,
            filter,
            stop_rx,
            db_conn,
            flow_table,
        )
        .await;

        // Release the "capture in progress" latch on every exit path. Auto-stop
        // never goes through stop_capture(), so without this the latch stays set
        // and every subsequent start_capture() fails forever.
        let state = app_handle_clone.state::<AppState>();
        if let Ok(mut guard) = state.stop_tx.lock() {
            *guard = None;
        }

        if let Err(e) = result {
            eprintln!("Capture error: {}", e);
            // `start_capture` has already returned Ok() by the time this runs,
            // so without this event a capture that dies while opening the
            // device (permissions, interface removed, filter compile) leaves
            // the UI "capturing" forever with no packets and no explanation.
            let _ = app_handle_clone.emit_all("capture_error", e);
        }
    });

    // Count the attempt only once it has actually been started.
    state.rate_limiter.record_capture();

    Ok(())
}

/// Exports all packets from the database to a PCAP file.
#[tauri::command(async)]
fn export_pcap_all(file_path: String, state: tauri::State<'_, AppState>) -> Result<usize, String> {
    // Validate file path to prevent path traversal
    let path = validate_export_path_new(&file_path)?;

    // Snapshot the high-water mark so a concurrently running capture can add
    // packets without ever growing this export past its starting point.
    let max_id: Option<i64> = {
        let db = state
            .db_conn
            .lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
        db.query_row("SELECT MAX(id) FROM packets", [], |row| row.get(0))
            .map_err(|e| format!("Query failed: {}", e))?
    };

    let max_id = max_id.ok_or_else(|| "No packets found in database".to_string())?;

    let mut writer = export::PcapWriter::create(&path)?;
    let mut cursor: i64 = 0;

    loop {
        // Fetch a bounded page, then drop the lock before touching the disk:
        // holding it across the whole export would stall the capture's
        // insert path for as long as the export takes.
        let page: Vec<(i64, i64, Vec<u8>, i64)> = {
            let db = state
                .db_conn
                .lock()
                .map_err(|e| format!("Failed to lock db: {}", e))?;

            let mut stmt = db
                .prepare(
                    "SELECT id, timestamp_ns, data, length FROM packets
                     WHERE id > ?1 AND id <= ?2
                     ORDER BY id ASC LIMIT 500",
                )
                .map_err(|e| format!("Prepare failed: {}", e))?;

            let rows = stmt
                .query_map(rusqlite::params![cursor, max_id], |row| {
                    let id: i64 = row.get(0)?;
                    let timestamp_ns: i64 = row.get(1)?;
                    let data: Vec<u8> = row.get(2)?;
                    let length: i64 = row.get(3)?;
                    Ok((id, timestamp_ns, data, length))
                })
                .map_err(|e| format!("Query failed: {}", e))?;

            rows.flatten().collect()
        };

        if page.is_empty() {
            break;
        }

        cursor = page.last().map(|(id, _, _, _)| *id).unwrap_or(cursor);
        for (_, timestamp_ns, data, length) in &page {
            writer.write(*timestamp_ns, data, (*length).max(0) as u32)?;
        }
    }

    if writer.packets_written() == 0 {
        return Err("No packets found in database".to_string());
    }

    Ok(writer.packets_written())
}

/// Resolves the flow that owns a stored packet.
///
/// The flow key is derivable from the packet bytes, so this reads the packet by
/// primary key and hashes it — one indexed lookup plus one hash. The previous
/// implementation walked every flow's `packet_ids` with `contains`, which is
/// O(every packet captured) per call: on a large session a single "Follow
/// Stream" click scanned millions of ids across millions of small vectors while
/// holding the flow lock.
fn flow_key_for_packet(state: &AppState, packet_id: u64) -> Result<crate::state::FlowKey, String> {
    let data = {
        let db = state
            .db_conn
            .lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
        db.query_row(
            "SELECT data FROM packets WHERE id = ?1",
            [packet_id as i64],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| format!("Packet {} not found in database.", packet_id))?
    };

    dissector::get_flow_key(&data)
        .ok_or_else(|| "Packet does not belong to a tracked flow.".to_string())
}

/// Retrieves all packet summaries belonging to the same flow as the given packet.
#[tauri::command]
async fn get_flow_packets(
    packet_id: u64,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<model::PacketSummary>, String> {
    // Validate packet_id is not zero
    if packet_id == 0 {
        return Err("Invalid packet ID".to_string());
    }

    let flow_key = flow_key_for_packet(&state, packet_id)?;

    let flows = state
        .flow_table
        .lock()
        .map_err(|e| format!("Failed to lock flow table: {}", e))?;

    let packet_ids = flows
        .flows
        .get(&flow_key)
        .ok_or_else(|| "Packet does not belong to a tracked flow.".to_string())?
        .packet_ids
        .clone();
    drop(flows); // Release lock

    let db = state
        .db_conn
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    let mut packet_list = Vec::new();
    // Fetch summaries for all IDs in this flow
    for chunk in packet_ids.chunks(999) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let query = format!(
                "SELECT id, timestamp_ns, source_addr, dest_addr, protocol, length, info, src_port, dst_port FROM packets WHERE id IN ({}) ORDER BY id ASC", 
                placeholders
            );
        let mut stmt = db
            .prepare(&query)
            .map_err(|e| format!("Prepare failed: {}", e))?;

        let i64_ids: Vec<i64> = chunk.iter().map(|&id| id as i64).collect();
        let params: Vec<&dyn rusqlite::ToSql> = i64_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt
            .query_map(&*params, |row| {
                Ok(model::PacketSummary {
                    id: row.get::<_, i64>(0)? as u64,
                    timestamp: row.get(1)?,
                    source_addr: row.get(2)?,
                    dest_addr: row.get(3)?,
                    protocol: row.get(4)?,
                    length: row.get(5)?,
                    info: row.get(6)?,
                    src_port: row.get(7)?,
                    dst_port: row.get(8)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;

        for row in rows.flatten() {
            packet_list.push(row);
        }
    }
    Ok(packet_list)
}

/// Collects the packets of one flow and turns them into reassembled messages.
///
/// TCP sides are reassembled by sequence number (ordering, retransmissions and
/// gaps are handled by [`dissector::reassemble_tcp`]); UDP has no sequence
/// numbers, so capture order is the only order there is.
struct StreamAssembler {
    is_tcp: bool,
    /// Keyed by (address, **port**): a flow can run between two ports of the
    /// same host (127.0.0.1:54321 -> 127.0.0.1:80), where keying on the address
    /// alone would merge both directions into a single sequence space.
    sides: Vec<((String, u16), Vec<dissector::TcpSegment>)>,
    datagrams: Vec<((String, u16), dissector::StreamBlock)>,
    /// Whoever sent the first packet of the flow — the SYN sender whenever the
    /// capture includes the handshake.
    initiator: Option<(i64, (String, u16))>,
}

impl StreamAssembler {
    fn new(protocol: u8) -> Self {
        Self {
            is_tcp: protocol == 6,
            sides: Vec::new(),
            datagrams: Vec::new(),
            initiator: None,
        }
    }

    fn push(&mut self, timestamp_ns: i64, data: &[u8], source_addr: String) {
        let side = dissector::extract_ports(data).map(|(src_port, _)| (source_addr, src_port));

        if self
            .initiator
            .as_ref()
            .is_none_or(|(earliest, _)| timestamp_ns < *earliest)
        {
            if let Some(side) = side.clone() {
                self.initiator = Some((timestamp_ns, side));
            }
        }

        if self.is_tcp {
            // Sequence-aware: segments are collected per side here and ordered,
            // deduplicated and split at gaps afterwards — arrival order tells
            // us nothing about where a byte belongs.
            if let (Some(segment), Some(side)) =
                (dissector::get_tcp_segment(data, timestamp_ns), side)
            {
                match self.sides.iter().position(|(key, _)| *key == side) {
                    Some(idx) => self.sides[idx].1.push(segment),
                    None => self.sides.push((side, vec![segment])),
                }
            }
            return;
        }

        if let (Some(payload), Some(side)) = (dissector::get_transport_payload(data), side) {
            if payload.is_empty() {
                return;
            }
            self.datagrams.push((
                side,
                dissector::StreamBlock {
                    data: payload,
                    missing_before: 0,
                    timestamp_ns,
                },
            ));
        }
    }

    fn finish(self) -> Vec<model::StreamMessage> {
        let client_side = self.initiator.map(|(_, side)| side);
        let is_client_side = |side: &(String, u16)| Some(side) == client_side.as_ref();

        let mut blocks: Vec<(bool, dissector::StreamBlock)> = Vec::new();
        for (side, segments) in self.sides {
            let is_client = is_client_side(&side);
            blocks.extend(
                dissector::reassemble_tcp(segments)
                    .into_iter()
                    .map(|block| (is_client, block)),
            );
        }
        for (side, block) in self.datagrams {
            blocks.push((is_client_side(&side), block));
        }

        // Interleave the two directions by capture time; each direction's own
        // bytes are already in stream order.
        blocks.sort_by_key(|(is_client, block)| (block.timestamp_ns, !*is_client));

        blocks
            .into_iter()
            .map(|(is_client, block)| model::StreamMessage {
                is_client,
                data: block.data,
                timestamp: block.timestamp_ns,
                missing_before: block.missing_before,
            })
            .collect()
    }
}

/// Reassembles the transport layer stream for the given packet's flow.
#[tauri::command]
async fn get_stream_content(
    packet_id: u64,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<model::StreamMessage>, String> {
    // Validate packet_id is not zero
    if packet_id == 0 {
        return Err("Invalid packet ID".to_string());
    }

    let flow_key = flow_key_for_packet(&state, packet_id)?;

    let flows = state
        .flow_table
        .lock()
        .map_err(|e| format!("Failed to lock flow table: {}", e))?;

    let packet_ids = flows
        .flows
        .get(&flow_key)
        .ok_or_else(|| "Flow not found.".to_string())?
        .packet_ids
        .clone();
    drop(flows);

    let db = state
        .db_conn
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    let mut assembler = StreamAssembler::new(flow_key.protocol);

    // Fetch raw data for all packets in the flow
    for chunk in packet_ids.chunks(999) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let query = format!(
                "SELECT timestamp_ns, data, source_addr FROM packets WHERE id IN ({}) ORDER BY timestamp_ns ASC",
                placeholders
            );
        let mut stmt = db
            .prepare(&query)
            .map_err(|e| format!("Prepare failed: {}", e))?;

        let i64_ids: Vec<i64> = chunk.iter().map(|&id| id as i64).collect();
        let params: Vec<&dyn rusqlite::ToSql> = i64_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt
            .query_map(&*params, |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| format!("Query failed: {}", e))?;

        for row in rows.flatten() {
            let (ts, data, src_addr) = row;
            assembler.push(ts, &data, src_addr);
        }
    }
    drop(db);

    Ok(assembler.finish())
}

/// Stops the currently active packet capture session.
#[tauri::command]
fn stop_capture(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut stop_tx_guard = state
        .stop_tx
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    if let Some(tx) = stop_tx_guard.take() {
        tx.try_send(())
            .map_err(|e| format!("Failed to send stop signal: {}", e))?;
    }
    Ok(())
}

/// Deletes every stored packet and flow so the user can start over without
/// restarting the app. Before this command existed the UI's Clear button only
/// emptied the in-memory list, so the packets reappeared on the next fetch.
#[tauri::command(async)]
fn clear_packets(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    purge_capture_data(&state)
}

/// Shared by `clear_packets` and `start_capture`.
///
/// The locks are taken sequentially rather than nested — `db_conn` is released
/// before `flow_table` is taken — matching how the capture loop acquires them.
fn purge_capture_data(state: &AppState) -> Result<usize, String> {
    let cleared = {
        let db = state
            .db_conn
            .lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;
        db.execute("DELETE FROM packets", [])
            .map_err(|e| format!("Failed to clear packets: {}", e))?
    };

    let mut flows = state
        .flow_table
        .lock()
        .map_err(|e| format!("Failed to lock flow table: {}", e))?;
    flows.clear();

    Ok(cleared)
}

/// Retrieves detailed protocol dissection for a specific packet.
#[tauri::command]
async fn get_packet_detail(
    id: u64,
    state: tauri::State<'_, AppState>,
) -> Result<model::PacketDetail, String> {
    // Validate packet ID
    if id == 0 {
        return Err("Invalid packet ID".to_string());
    }

    let db = state
        .db_conn
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    let id_i64 = id as i64;
    let mut stmt = db
        .prepare("SELECT data, timestamp_ns, length FROM packets WHERE id = ?1")
        .map_err(|e| format!("Prepare failed: {}", e))?;
    let packet: Option<(Vec<u8>, i64, i64)> = stmt
        .query_row([id_i64], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .ok();

    if let Some((data, timestamp_ns, length)) = packet {
        if let Some(mut detail) = dissector::dissect_packet(&data, id, timestamp_ns) {
            // `dissect_packet` only sees the stored bytes, so it can report the
            // captured length; the summary carries the length on the wire.
            detail.summary.length = length as u32;
            Ok(detail)
        } else {
            Err("Failed to dissect packet.".to_string())
        }
    } else {
        Err("Packet not found in database.".to_string())
    }
}

/// Exports selected packets to a PCAP file.
#[tauri::command(async)]
fn export_pcap(
    file_path: String,
    packet_ids: Vec<u64>,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    // Validate packet IDs
    validate_packet_ids(&packet_ids)?;

    // Validate file path to prevent path traversal
    let path = validate_export_path_new(&file_path)?;

    // PCAP records must come out in capture order, so process the ids
    // ascending; sorting up front also makes the chunk queries disjoint.
    let mut sorted_ids = packet_ids;
    sorted_ids.sort_unstable();
    sorted_ids.dedup();

    let mut writer = export::PcapWriter::create(&path)?;

    for chunk in sorted_ids.chunks(999) {
        // SQLite bind limit is typically 999
        let placeholders = vec!["?"; chunk.len()].join(",");
        let query = format!(
            "SELECT timestamp_ns, data, length FROM packets WHERE id IN ({}) ORDER BY id ASC",
            placeholders
        );

        // Re-acquire the lock per chunk so a concurrent capture is only
        // blocked for the duration of one chunk, not the whole export.
        let db = state
            .db_conn
            .lock()
            .map_err(|e| format!("Failed to lock db: {}", e))?;

        let mut stmt = db
            .prepare(&query)
            .map_err(|e| format!("Prepare failed: {}", e))?;

        let i64_ids: Vec<i64> = chunk.iter().map(|&id| id as i64).collect();
        let params: Vec<&dyn rusqlite::ToSql> = i64_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt
            .query_map(&*params, |row| {
                let timestamp_ns: i64 = row.get(0)?;
                let data: Vec<u8> = row.get(1)?;
                let length: i64 = row.get(2)?;
                Ok((timestamp_ns, data, length))
            })
            .map_err(|e| format!("Query failed: {}", e))?;

        for row in rows.flatten() {
            let (timestamp_ns, data, length) = row;
            writer.write(timestamp_ns, &data, length.max(0) as u32)?;
        }
    }

    if writer.packets_written() == 0 {
        return Err("No valid packets found in database".to_string());
    }

    Ok(writer.packets_written())
}

fn init_db(_app_handle: &tauri::AppHandle) -> Result<Connection, Box<dyn std::error::Error>> {
    // Use platform-specific app data directory for security
    let db_path = if let Some(data_dir) = dirs::data_local_dir() {
        let app_dir = data_dir.join("auracap");
        std::fs::create_dir_all(&app_dir)?;
        app_dir.join("capture.db")
    } else {
        // Fallback to current directory if dirs fails
        let mut root_dir = std::env::current_dir()?;
        if root_dir.ends_with("src-tauri") {
            if let Some(parent) = root_dir.parent() {
                root_dir = parent.to_path_buf();
            }
        }
        root_dir.join("capture.db")
    };

    log::info!("Initializing database at: {:?}", db_path);

    let conn = Connection::open(&db_path)?;

    // Set secure file permissions on Unix systems (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if db_path.exists() {
            if let Ok(metadata) = db_path.metadata() {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600); // rw-------
                let _ = std::fs::set_permissions(&db_path, perms);
            }
        }
    }

    // Refresh schema for development to ensure all columns exist
    conn.execute("DROP TABLE IF EXISTS packets", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS packets (
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
        )",
        [],
    )?;

    // Create an index on id for fast pagination
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_packets_id ON packets(id)",
        [],
    )?;

    // Optimizations for write-heavy workload
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
    ",
    )?;

    Ok(conn)
}

/// Validates an existing capture file handed over by the renderer.
fn validate_import_path(file_path: &str) -> Result<PathBuf, String> {
    let path = parse_abs_path(file_path)?;

    if !has_extension(path, &["pcap", "pcapng", "cap"]) {
        return Err("File must have .pcap, .pcapng, or .cap extension".to_string());
    }

    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    if !path.is_file() {
        return Err("Path is not a file".to_string());
    }

    // Resolve symlinks (and any residual `.`/`..`) so downstream code opens
    // exactly what we validated, then confirm the target is still a regular file.
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve file path: {}", e))?;

    if !canonical.is_file() {
        return Err("Path is not a file".to_string());
    }

    Ok(canonical)
}

#[tauri::command(async)]
fn import_pcap(file_path: String, state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let path = validate_import_path(&file_path)?;

    let mut capture = pcap::Capture::from_file(path.as_path())
        .map_err(|e| format!("Failed to read PCAP file: {}", e))?;

    let linktype = capture.get_datalink();
    if linktype.0 != 1 {
        return Err(format!(
            "Unsupported link type: {}. Only Ethernet (DLT_EN10MB) is supported.",
            linktype.0
        ));
    }

    let mut db = state
        .db_conn
        .lock()
        .map_err(|e| format!("Failed to lock db: {}", e))?;

    db.execute("DELETE FROM packets", [])
        .map_err(|e| format!("Failed to clear packets: {}", e))?;

    {
        let mut flows = state
            .flow_table
            .lock()
            .map_err(|e| format!("Failed to lock flow table: {}", e))?;
        flows.clear();
    }

    let mut packet_count = 0u64;
    let mut packet_id = 0u64;
    let mut batch: Vec<PacketBatchEntry> = Vec::new();
    const BATCH_SIZE: usize = 500;

    loop {
        match capture.next_packet() {
            Ok(packet) => {
                packet_id += 1;
                let data = packet.data.to_vec();
                let timestamp_ns =
                    capture::ns_from_parts(packet.header.ts.tv_sec, packet.header.ts.tv_usec);

                if let Some(mut summary) = dissector::parse_summary(&data, packet_id, timestamp_ns)
                {
                    // Keep the length the frame had on the wire: a PCAP record
                    // carries incl_len and orig_len separately, and the bytes we
                    // store are whatever the file held (possibly truncated).
                    summary.length = packet.header.len.max(summary.length);
                    let data_clone = data.clone();
                    batch.push((
                        packet_id as i64,
                        timestamp_ns,
                        summary.source_addr,
                        summary.dest_addr,
                        summary.protocol,
                        summary.length as i32,
                        summary.info,
                        summary.src_port,
                        summary.dst_port,
                        data,
                    ));

                    if let Some(key) = dissector::get_flow_key(&data_clone) {
                        if let Ok(mut flows) = state.flow_table.lock() {
                            flows.update(packet_id, timestamp_ns, summary.length, key);
                        }
                    }

                    if batch.len() >= BATCH_SIZE {
                        insert_batch(&mut db, &batch)?;
                        packet_count += batch.len() as u64;
                        batch.clear();
                    }
                }
            }
            Err(pcap::Error::TimeoutExpired) => continue,
            Err(_) => break,
        }
    }

    if !batch.is_empty() {
        insert_batch(&mut db, &batch)?;
        packet_count += batch.len() as u64;
    }

    log::info!("Imported {} packets from PCAP file", packet_count);
    Ok(packet_count as usize)
}

fn insert_batch(db: &mut Connection, batch: &[PacketBatchEntry]) -> Result<(), String> {
    match db.transaction() {
        Ok(tx) => {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO packets (id, timestamp_ns, source_addr, dest_addr, protocol, length, info, src_port, dst_port, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
            ).map_err(|e| format!("Prepare failed: {}", e))?;

            for (id, ts, src, dst, proto, len, info, src_port, dst_port, data) in batch {
                stmt.execute(rusqlite::params![
                    id, ts, src, dst, proto, len, info, src_port, dst_port, data
                ])
                .map_err(|e| format!("Insert failed: {}", e))?;
            }

            drop(stmt);
            tx.commit().map_err(|e| format!("Commit failed: {}", e))?;
        }
        Err(e) => return Err(format!("Transaction failed: {}", e)),
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    #[cfg(debug_assertions)]
    {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }
    #[cfg(not(debug_assertions))]
    {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    log::info!("Starting AuraCap Network Analyzer");

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            let db_conn = init_db(&handle).expect("Failed to initialize SQLite database");

            app.manage(AppState {
                stop_tx: Mutex::new(None),
                db_conn: Arc::new(Mutex::new(db_conn)),
                flow_table: Arc::new(Mutex::new(FlowTable::new())),
                rate_limiter: CaptureRateLimiter::new(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_interfaces,
            start_capture,
            stop_capture,
            get_packet_detail,
            export_pcap,
            export_pcap_all,
            get_packets,
            get_packet_count,
            get_flow_packets,
            get_stream_content,
            import_pcap,
            clear_packets
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("Failed to start Tauri application: {}", error);
            eprintln!("Failed to start Tauri application: {}", error);
            std::process::exit(1);
        });
}

#[cfg(test)]
mod path_validation_tests {
    use super::*;
    use std::fs;

    /// Unique-per-process scratch name so parallel tests cannot collide.
    fn scratch(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("auracap-{}-{}", std::process::id(), suffix))
    }

    // ---- export ----

    #[test]
    fn export_rejects_traversal_between_components() {
        // The old `contains("..")` guard let this one through because the `..`
        // was surrounded by other characters.
        let p = scratch("dir").join("../evil.pcap");
        assert!(validate_export_path_new(&p.to_string_lossy()).is_err());
    }

    #[test]
    fn export_rejects_leading_traversal() {
        assert!(validate_export_path_new("../../../etc/passwd.pcap").is_err());
    }

    #[test]
    fn export_rejects_relative_paths() {
        assert!(validate_export_path_new("capture.pcap").is_err());
    }

    #[test]
    fn export_rejects_null_bytes() {
        assert!(validate_export_path_new("/tmp/cap\0.pcap").is_err());
    }

    #[test]
    fn export_rejects_missing_or_wrong_extension() {
        assert!(validate_export_path_new("/tmp/capture").is_err());
        assert!(validate_export_path_new("/tmp/capture.txt").is_err());
        assert!(validate_export_path_new("/tmp/capture.pcapng").is_err());
    }

    #[test]
    fn export_rejects_missing_parent_directory() {
        assert!(validate_export_path_new("/no/such/dir/capture.pcap").is_err());
    }

    #[test]
    fn export_accepts_absolute_pcap_in_existing_directory() {
        let p = scratch("export.pcap");
        assert!(validate_export_path_new(&p.to_string_lossy()).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn export_rejects_overwriting_a_symlink() {
        let target = scratch("target.pcap");
        let link = scratch("link.pcap");
        let _ = fs::remove_file(&link);
        fs::write(&target, b"").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(validate_export_path_new(&link.to_string_lossy()).is_err());

        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&link);
    }

    // ---- import ----

    #[test]
    fn import_rejects_traversal_and_relative_paths() {
        assert!(validate_import_path("/tmp/../../etc/passwd.pcap").is_err());
        assert!(validate_import_path("capture.pcap").is_err());
    }

    #[test]
    fn import_rejects_missing_file() {
        let missing = scratch("missing.pcap");
        assert_eq!(
            validate_import_path(&missing.to_string_lossy()),
            Err("File does not exist".to_string())
        );
    }

    #[test]
    fn import_rejects_directory() {
        let dir = scratch("dir.pcap");
        fs::create_dir_all(&dir).unwrap();

        assert_eq!(
            validate_import_path(&dir.to_string_lossy()),
            Err("Path is not a file".to_string())
        );

        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn import_rejects_disallowed_extension() {
        assert_eq!(
            validate_import_path("/etc/hosts"),
            Err("File must have .pcap, .pcapng, or .cap extension".to_string())
        );
    }

    #[test]
    fn import_accepts_existing_capture_file() {
        let file = scratch("import.pcap");
        fs::write(&file, b"\xd4\xc3\xb2\xa1").unwrap();

        let resolved = validate_import_path(&file.to_string_lossy()).expect("should validate");
        assert!(resolved.is_absolute());
        assert!(resolved.is_file());

        let _ = fs::remove_file(&file);
    }
}

/// End-to-end coverage for the `port:` filter against the real schema: params
/// arrive as strings while the columns are INTEGER, so SQLite's type affinity
/// decides whether the comparison matches anything at all.
#[cfg(test)]
mod filter_sql_tests {
    use super::*;

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

    fn seeded_db() -> Connection {
        let db = Connection::open_in_memory().expect("in-memory db");
        db.execute(SCHEMA, []).expect("schema");
        // `info` deliberately contains both "8080" and "443" so any regression
        // to substring matching shows up immediately.
        db.execute(
            "INSERT INTO packets VALUES (1, 1, '10.0.0.1', '10.0.0.2', 'HTTPS', 60,
                                        '10.0.0.1:8080 -> 10.0.0.2:443', 8080, 443, x'00')",
            [],
        )
        .expect("insert 1");
        db.execute(
            "INSERT INTO packets VALUES (2, 2, '10.0.0.1', '10.0.0.3', 'HTTP', 60,
                                        '10.0.0.1:1234 -> 10.0.0.3:80', 1234, 80, x'00')",
            [],
        )
        .expect("insert 2");
        db
    }

    fn count_matching(db: &Connection, filter: &str) -> usize {
        let (where_clause, params) = build_filter_clause(filter);
        let query = format!("SELECT COUNT(*) FROM packets {}", where_clause);
        let mut stmt = db.prepare(&query).expect("prepare");
        let sql_params: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let count: i64 = stmt
            .query_row(&*sql_params, |row| row.get(0))
            .expect("count");
        count as usize
    }

    #[test]
    fn port_filter_matches_exact_ports_on_either_side() {
        let db = seeded_db();
        assert_eq!(count_matching(&db, "port:443"), 1);
        assert_eq!(count_matching(&db, "port:8080"), 1);
        assert_eq!(count_matching(&db, "port:80"), 1); // dst port of packet 2
        assert_eq!(count_matching(&db, "port:1234"), 1); // src port of packet 2
        assert_eq!(count_matching(&db, "port:9999"), 0);
    }

    #[test]
    fn port_filter_no_longer_matches_substrings_of_info() {
        let db = seeded_db();
        // The old `WHERE info LIKE '%44%'` matched packet 1 (info holds "443"),
        // and `'%10.0.0.1%'` matched both rows by address.
        assert_eq!(count_matching(&db, "port:44"), 0);
        assert_eq!(count_matching(&db, "port:10.0.0.1"), 0);
    }

    #[test]
    fn port_filter_rejects_non_numeric_ports() {
        let db = seeded_db();
        assert_eq!(count_matching(&db, "port:abc"), 0);
        assert_eq!(count_matching(&db, "port:"), 0);
    }
}

#[cfg(test)]
mod flow_lookup_tests {
    use super::*;

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

    /// Ethernet + IPv4 + TCP frame: 10.0.0.1:12345 -> 93.184.216.34:80.
    fn tcp_frame() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00; 6]); // dst mac
        data.extend_from_slice(&[0x11; 6]); // src mac
        data.extend_from_slice(&[0x08, 0x00]); // ethertype: IPv4
        data.extend_from_slice(&[0x45, 0x00, 0x00, 0x28]); // v4, 40-byte IP payload
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x06, 0x00, 0x00]); // ttl, TCP, checksum
        data.extend_from_slice(&[10, 0, 0, 1]);
        data.extend_from_slice(&[93, 184, 216, 34]);
        data.extend_from_slice(&[0x30, 0x39]); // src port 12345
        data.extend_from_slice(&[0x00, 0x50]); // dst port 80
        data.extend_from_slice(&[0, 0, 0, 1]); // seq
        data.extend_from_slice(&[0, 0, 0, 0]); // ack
        data.push(0x50); // data offset
        data.push(0x02); // SYN
        data.extend_from_slice(&[0x20, 0x00]); // window
        data.extend_from_slice(&[0, 0, 0, 0]); // checksum, urgent pointer
        data
    }

    fn state_with(packets: &[(i64, &[u8])]) -> AppState {
        let db = Connection::open_in_memory().expect("in-memory db");
        db.execute(SCHEMA, []).expect("schema");
        for (id, data) in packets {
            db.execute(
                "INSERT INTO packets (id, timestamp_ns, source_addr, dest_addr, protocol,
                                      length, info, src_port, dst_port, data)
                 VALUES (?1, 1, '10.0.0.1', '93.184.216.34', 'TCP', 60, '', 12345, 80, ?2)",
                rusqlite::params![id, data],
            )
            .expect("insert");
        }

        AppState {
            stop_tx: Mutex::new(None),
            db_conn: Arc::new(Mutex::new(db)),
            flow_table: Arc::new(Mutex::new(FlowTable::new())),
            rate_limiter: CaptureRateLimiter::new(),
        }
    }

    #[test]
    fn flow_key_is_derived_from_the_packet_row() {
        let frame = tcp_frame();
        let state = state_with(&[(1, &frame), (2, &b"\x00"[..])]);

        let expected = dissector::get_flow_key(&frame).expect("frame has a flow key");
        assert_eq!(flow_key_for_packet(&state, 1).expect("resolved"), expected);

        // A stored packet that is not IP/TCP/UDP belongs to no flow.
        assert!(flow_key_for_packet(&state, 2).is_err());
        // A packet that was never stored cannot belong to one either.
        assert!(flow_key_for_packet(&state, 999).is_err());
    }
}

#[cfg(test)]
mod stream_assembler_tests {
    use super::*;

    /// Ethernet + IPv4 + TCP frame with full control over the 5-tuple and seq.
    fn tcp_frame(
        src_ip: [u8; 4],
        src_port: u16,
        dst_ip: [u8; 4],
        dst_port: u16,
        seq: u32,
        payload: &[u8],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00; 6]); // dst mac
        data.extend_from_slice(&[0x11; 6]); // src mac
        data.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4

        let total_len = 20 + 20 + payload.len();
        data.extend_from_slice(&[0x45, 0x00, (total_len >> 8) as u8, total_len as u8]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x06, 0x00, 0x00]); // TTL, protocol TCP
        data.extend_from_slice(&src_ip);
        data.extend_from_slice(&dst_ip);
        data.extend_from_slice(&src_port.to_be_bytes());
        data.extend_from_slice(&dst_port.to_be_bytes());
        data.extend_from_slice(&seq.to_be_bytes());
        data.extend_from_slice(&[0, 0, 0, 0]); // ack number
        data.push(0x50); // data offset
        data.push(if payload.is_empty() { 0x10 } else { 0x18 }); // ACK / PSH+ACK
        data.extend_from_slice(&[0x20, 0x00]); // window
        data.extend_from_slice(&[0, 0]); // checksum
        data.extend_from_slice(&[0, 0]); // urgent pointer
        data.extend_from_slice(payload);
        data
    }

    /// Ethernet + IPv4 + UDP datagram.
    fn udp_frame(
        src_ip: [u8; 4],
        src_port: u16,
        dst_ip: [u8; 4],
        dst_port: u16,
        payload: &[u8],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00; 6]);
        data.extend_from_slice(&[0x11; 6]);
        data.extend_from_slice(&[0x08, 0x00]);

        let ip_len = 20 + 8 + payload.len();
        data.extend_from_slice(&[0x45, 0x00, (ip_len >> 8) as u8, ip_len as u8]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x11, 0x00, 0x00]); // TTL, protocol UDP
        data.extend_from_slice(&src_ip);
        data.extend_from_slice(&dst_ip);
        data.extend_from_slice(&src_port.to_be_bytes());
        data.extend_from_slice(&dst_port.to_be_bytes());
        data.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        data.extend_from_slice(&[0, 0]); // checksum (optional for IPv4 UDP)
        data.extend_from_slice(payload);
        data
    }

    fn addr(ip: [u8; 4]) -> String {
        format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
    }

    #[test]
    fn both_directions_of_a_loopback_flow_stay_apart() {
        let host = [127, 0, 0, 1];
        let mut assembler = StreamAssembler::new(6);
        assembler.push(
            1,
            &tcp_frame(host, 50000, host, 80, 0, b"GET /"),
            addr(host),
        );
        assembler.push(
            2,
            &tcp_frame(host, 80, host, 50000, 0, b"HTTP/1.1 200 OK"),
            addr(host),
        );

        let messages = assembler.finish();
        // Sides are keyed by (address, port): keying on the address alone would
        // merge both directions into one sequence space, where the reply's seq 0
        // looks like a retransmission and the response disappears entirely.
        assert_eq!(messages.len(), 2);
        assert!(messages[0].is_client);
        assert_eq!(messages[0].data, b"GET /");
        assert!(!messages[1].is_client);
        assert_eq!(messages[1].data, b"HTTP/1.1 200 OK");
    }

    #[test]
    fn tcp_is_ordered_and_deduplicated_before_it_is_shown() {
        let client = [10, 0, 0, 1];
        let server = [10, 0, 0, 2];
        let mut assembler = StreamAssembler::new(6);
        // Arrival order is: tail first, then head, then a retransmission of it.
        assembler.push(
            3,
            &tcp_frame(client, 1234, server, 80, 5, b" world"),
            addr(client),
        );
        assembler.push(
            1,
            &tcp_frame(client, 1234, server, 80, 0, b"hello"),
            addr(client),
        );
        assembler.push(
            2,
            &tcp_frame(client, 1234, server, 80, 0, b"hello"),
            addr(client),
        );

        let messages = assembler.finish();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].data, b"hello world");
        assert_eq!(messages[0].missing_before, 0);
        assert!(messages[0].is_client); // it spoke first
                                        // The timestamp belongs to the packet that carried the first byte.
        assert_eq!(messages[0].timestamp, 1);
    }

    #[test]
    fn a_gap_is_reported_instead_of_gluing_the_halves_together() {
        let client = [10, 0, 0, 1];
        let server = [10, 0, 0, 2];
        let mut assembler = StreamAssembler::new(6);
        assembler.push(
            1,
            &tcp_frame(client, 1234, server, 80, 0, b"hello"),
            addr(client),
        );
        assembler.push(
            2,
            &tcp_frame(client, 1234, server, 80, 100, b"world"),
            addr(client),
        );

        let messages = assembler.finish();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].data, b"hello");
        assert_eq!(messages[1].data, b"world");
        assert_eq!(messages[1].missing_before, 95);
    }

    #[test]
    fn the_side_that_spoke_first_is_the_client() {
        let server = [10, 0, 0, 9];
        let client = [10, 0, 0, 1];
        let mut assembler = StreamAssembler::new(6);
        // Capture without the handshake: the server's banner arrives first.
        // The flow key would have called this side the server — it sorts the
        // 5-tuple, it does not know who connected to whom.
        assembler.push(
            1,
            &tcp_frame(server, 25, client, 44444, 0, b"220 mail\r\n"),
            addr(server),
        );
        assembler.push(
            2,
            &tcp_frame(client, 44444, server, 25, 0, b"EHLO me\r\n"),
            addr(client),
        );

        let messages = assembler.finish();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].is_client);
        assert_eq!(messages[0].data, b"220 mail\r\n");
        assert!(!messages[1].is_client);
        assert_eq!(messages[1].data, b"EHLO me\r\n");
    }

    #[test]
    fn udp_stays_one_message_per_datagram() {
        let host = [10, 0, 0, 1];
        let resolver = [10, 0, 0, 53];
        let mut assembler = StreamAssembler::new(17);
        assembler.push(
            1,
            &udp_frame(host, 5000, resolver, 53, b"query"),
            addr(host),
        );
        assembler.push(
            2,
            &udp_frame(resolver, 53, host, 5000, b"answer"),
            addr(resolver),
        );

        let messages = assembler.finish();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].is_client);
        assert_eq!(messages[0].data, b"query");
        assert!(!messages[1].is_client);
        assert_eq!(messages[1].data, b"answer");
        assert_eq!(messages[0].missing_before, 0);
    }
}
