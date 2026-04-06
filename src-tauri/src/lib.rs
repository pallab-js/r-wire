pub mod model;
pub mod dissector;
pub mod capture;
pub mod export;
pub mod state;

use std::sync::Mutex;
use tokio::sync::mpsc;
use std::sync::Arc;
use rusqlite::Connection;
use state::FlowTable;
use tauri::Manager;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// Initialize logging
#[cfg(not(debug_assertions))]
use log::LevelFilter;

/// Rate limiter for capture operations to prevent DoS.
pub struct CaptureRateLimiter {
    last_capture_time: Mutex<Option<Instant>>,
    capture_count: AtomicUsize,
    max_captures_per_minute: usize,
    min_interval_seconds: u64,
}

impl CaptureRateLimiter {
    fn new() -> Self {
        Self {
            last_capture_time: Mutex::new(None),
            capture_count: AtomicUsize::new(0),
            max_captures_per_minute: 10, // Max 10 captures per minute
            min_interval_seconds: 5,     // Min 5 seconds between captures
        }
    }

    fn check_rate_limit(&self) -> Result<(), String> {
        let now = Instant::now();
        
        // Check interval
        if let Some(last_time) = self.last_capture_time.lock()
            .map_err(|e| format!("Failed to lock rate limiter: {}", e))?.as_ref() 
        {
            let elapsed = now.duration_since(*last_time).as_secs();
            if elapsed < self.min_interval_seconds {
                return Err(format!(
                    "Rate limited: Please wait {} seconds before starting another capture",
                    self.min_interval_seconds - elapsed
                ));
            }
        }
        
        // Check count (reset if more than a minute has passed)
        let count = self.capture_count.load(Ordering::Relaxed);
        if count >= self.max_captures_per_minute {
            return Err("Rate limited: Too many capture attempts. Please wait a minute.".to_string());
        }
        
        Ok(())
    }

    fn record_capture(&self) {
        let now = Instant::now();
        if let Ok(mut last_time) = self.last_capture_time.lock() {
            *last_time = Some(now);
        }
        
        // Reset count if more than a minute has passed
        let count = self.capture_count.load(Ordering::Relaxed);
        if count >= self.max_captures_per_minute {
            self.capture_count.store(0, Ordering::Relaxed);
        } else {
            self.capture_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Validates export path for new files (doesn't need to exist yet).
fn validate_export_path_new(file_path: &str) -> Result<std::path::PathBuf, String> {
    let path = Path::new(file_path);
    
    // Reject paths with null bytes
    if file_path.contains('\0') {
        return Err("Invalid file path: contains null bytes".to_string());
    }
    
    // Reject paths attempting traversal
    if file_path.contains("..") {
        // Allow if it's the only component (relative path)
        let normalized = file_path.replace("..", "");
        if normalized.is_empty() || normalized.chars().all(|c| c == '/' || c == '\\') {
            return Err("Path traversal detected: '..' not allowed".to_string());
        }
    }
    
    // Validate file extension is .pcap
    if let Some(ext) = path.extension() {
        if ext.to_string_lossy().to_lowercase() != "pcap" {
            return Err("File must have .pcap extension".to_string());
        }
    } else {
        return Err("File must have .pcap extension".to_string());
    }
    
    Ok(path.to_path_buf())
}

/// Validates network interface name to prevent injection attacks.
fn validate_interface_name(name: &str) -> Result<(), String> {
    // Interface names should only contain alphanumeric chars, hyphens, underscores, and dots
    if name.is_empty() || name.len() > 256 {
        return Err("Interface name must be between 1 and 256 characters".to_string());
    }
    
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err("Interface name contains invalid characters".to_string());
    }
    
    Ok(())
}

/// Validates BPF filter string to prevent command injection.
fn validate_bpf_filter(filter: &str) -> Result<(), String> {
    // BPF filters should be reasonable length
    if filter.len() > 1024 {
        return Err("BPF filter too long (max 1024 characters)".to_string());
    }
    
    // Reject filters with shell metacharacters
    let dangerous_chars = [';', '&', '|', '`', '$', '(', ')', '{', '}', '<', '>', '!', '\n', '\r'];
    for ch in dangerous_chars {
        if filter.contains(ch) {
            return Err(format!("BPF filter contains invalid character: {}", ch));
        }
    }
    
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
    state: tauri::State<'_, AppState>
) -> Result<Vec<model::PacketSummary>, String> {
    // Validate pagination parameters
    validate_pagination(offset, limit)?;
    
    // Validate filter if provided
    if let Some(ref f) = filter {
        validate_filter(f)?;
    }
    
    let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
    
    let (where_clause, params) = if let Some(f) = filter {
        build_filter_clause(&f)
    } else {
        ("".to_string(), vec![])
    };

    let query = format!(
        "SELECT id, timestamp_ns, source_addr, dest_addr, protocol, length, info FROM packets {} ORDER BY id ASC LIMIT ? OFFSET ?",
        where_clause
    );

    let mut stmt = db.prepare(&query).map_err(|e| format!("Prepare failed: {}", e))?;
    
    // Convert Vec<String> to Vec<&dyn ToSql>
    let mut sql_params: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let limit_i64 = limit as i64;
    let offset_i64 = offset as i64;
    sql_params.push(&limit_i64);
    sql_params.push(&offset_i64);

    let packet_rows = stmt.query_map(&*sql_params, |row| {
        Ok(model::PacketSummary {
            id: row.get::<_, i64>(0)? as u64,
            timestamp: row.get(1)?,
            source_addr: row.get(2)?,
            dest_addr: row.get(3)?,
            protocol: row.get(4)?,
            length: row.get(5)?,
            info: row.get(6)?,
        })
    }).map_err(|e| format!("Query failed: {}", e))?;

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
    state: tauri::State<'_, AppState>
) -> Result<usize, String> {
    // Validate filter if provided
    if let Some(ref f) = filter {
        validate_filter(f)?;
    }
    
    let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
    
    let (where_clause, params) = if let Some(f) = filter {
        build_filter_clause(&f)
    } else {
        ("".to_string(), vec![])
    };

    let query = format!("SELECT COUNT(*) FROM packets {}", where_clause);
    let mut stmt = db.prepare(&query).map_err(|e| format!("Prepare failed: {}", e))?;
    
    let sql_params: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    
    let count: i64 = stmt.query_row(&*sql_params, |row| row.get(0)).map_err(|e| format!("Count failed: {}", e))?;
    
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
        return ("WHERE protocol LIKE ?".to_string(), vec![format!("%{}%", val)]);
    }
    if filter.starts_with("ip:") {
        let val = filter.replace("ip:", "").trim().to_string();
        return ("WHERE source_addr LIKE ? OR dest_addr LIKE ?".to_string(), vec![format!("%{}%", val), format!("%{}%", val)]);
    }
    if filter.starts_with("src:") {
        let val = filter.replace("src:", "").trim().to_string();
        return ("WHERE source_addr LIKE ?".to_string(), vec![format!("%{}%", val)]);
    }
    if filter.starts_with("dst:") {
        let val = filter.replace("dst:", "").trim().to_string();
        return ("WHERE dest_addr LIKE ?".to_string(), vec![format!("%{}%", val)]);
    }
    if filter.starts_with("port:") {
        let val = filter.replace("port:", "").trim().to_string();
        return ("WHERE info LIKE ?".to_string(), vec![format!("%{}%", val)]);
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
#[tauri::command]
fn list_interfaces() -> Result<Vec<String>, String> {
    match pcap::Device::list() {
        Ok(devices) => {
            Ok(devices.iter().map(|d| d.name.clone()).collect())
        }
        Err(e) => Err(format!("Failed to list interfaces: {}", e))
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
        validate_bpf_filter(f)?;
    }
    
    // Check rate limit
    state.rate_limiter.check_rate_limit()?;
    
    // Check if already capturing
    let mut stop_tx_guard = state.stop_tx.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    if stop_tx_guard.is_some() {
        return Err("Capture already in progress".to_string());
    }
    
    // Record this capture for rate limiting
    state.rate_limiter.record_capture();

    // Create channel for stop signal
    let (stop_tx, stop_rx) = mpsc::channel(1);
    *stop_tx_guard = Some(stop_tx);
    drop(stop_tx_guard); // Release lock early

    // Clear packet table and flow table
    {
        let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
        db.execute("DELETE FROM packets", []).map_err(|e| format!("Failed to clear packets: {}", e))?;
        
        let mut flows = state.flow_table.lock().map_err(|e| format!("Failed to lock flow table: {}", e))?;
        flows.clear();
    }

    // Clone the DB Arc for the task
    let db_conn = Arc::clone(&state.db_conn);
    let flow_table = Arc::clone(&state.flow_table);

    // Spawn the capture task
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        if let Err(e) = capture::run_capture(app_handle_clone, interface_name, filter, stop_rx, db_conn, flow_table).await {
            eprintln!("Capture error: {}", e);
        }
    });

    Ok(())
}

/// Exports all packets from the database to a PCAP file.
#[tauri::command]
fn export_pcap_all(
    file_path: String,
    state: tauri::State<'_, AppState>
) -> Result<usize, String> {
    // Validate file path to prevent path traversal
    let path = validate_export_path_new(&file_path)?;

    let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;

    let mut stmt = db.prepare("SELECT id, timestamp_ns, data FROM packets ORDER BY id ASC").map_err(|e| format!("Prepare failed: {}", e))?;
    
    let rows = stmt.query_map([], |row| {
        let id_i64: i64 = row.get(0)?;
        let id = id_i64 as u64;
        let timestamp_ns: i64 = row.get(1)?;
        let data: Vec<u8> = row.get(2)?;
        Ok((id, timestamp_ns, data))
    }).map_err(|e| format!("Query failed: {}", e))?;
    
    let mut packet_list = Vec::new();
    for row in rows {
        if let Ok((id, timestamp_ns, data)) = row {
            if let Some(summary) = dissector::parse_summary(&data, id, timestamp_ns) {
                packet_list.push((summary, data, timestamp_ns));
            }
        }
    }
    
    if packet_list.is_empty() {
        return Err("No packets found in database".to_string());
    }
    
    let exported_count = packet_list.len();
    export::export_pcap_db(&packet_list, path)?;
    
    Ok(exported_count)
}

/// Retrieves all packet summaries belonging to the same flow as the given packet.
#[tauri::command]
async fn get_flow_packets(
    packet_id: u64,
    state: tauri::State<'_, AppState>
) -> Result<Vec<model::PacketSummary>, String> {
    // Validate packet_id is not zero
    if packet_id == 0 {
        return Err("Invalid packet ID".to_string());
    }
    
    let flows = state.flow_table.lock().map_err(|e| format!("Failed to lock flow table: {}", e))?;
    
    // Find the flow key for this packet ID
    let flow_key = flows.flows.iter()
        .find(|(_, flow)| flow.packet_ids.contains(&packet_id))
        .map(|(key, _)| key.clone());
    
    if let Some(key) = flow_key {
        let flow = flows.flows.get(&key)
            .ok_or_else(|| "Flow data not found after lookup".to_string())?;
        let packet_ids = flow.packet_ids.clone();
        drop(flows); // Release lock
        
        let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
        
        let mut packet_list = Vec::new();
        // Fetch summaries for all IDs in this flow
        for chunk in packet_ids.chunks(999) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let query = format!(
                "SELECT id, timestamp_ns, source_addr, dest_addr, protocol, length, info FROM packets WHERE id IN ({}) ORDER BY id ASC", 
                placeholders
            );
            let mut stmt = db.prepare(&query).map_err(|e| format!("Prepare failed: {}", e))?;
            
            let i64_ids: Vec<i64> = chunk.iter().map(|&id| id as i64).collect();
            let params: Vec<&dyn rusqlite::ToSql> = i64_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
            
            let rows = stmt.query_map(&*params, |row| {
                Ok(model::PacketSummary {
                    id: row.get::<_, i64>(0)? as u64,
                    timestamp: row.get(1)?,
                    source_addr: row.get(2)?,
                    dest_addr: row.get(3)?,
                    protocol: row.get(4)?,
                    length: row.get(5)?,
                    info: row.get(6)?,
                })
            }).map_err(|e| format!("Query failed: {}", e))?;
            
            for row in rows {
                if let Ok(summary) = row {
                    packet_list.push(summary);
                }
            }
        }
        Ok(packet_list)
    } else {
        Err("Packet does not belong to a tracked flow.".to_string())
    }
}

/// Reassembles the transport layer stream for the given packet's flow.
#[tauri::command]
async fn get_stream_content(
    packet_id: u64,
    state: tauri::State<'_, AppState>
) -> Result<Vec<model::StreamMessage>, String> {
    // Validate packet_id is not zero
    if packet_id == 0 {
        return Err("Invalid packet ID".to_string());
    }
    
    let flows = state.flow_table.lock().map_err(|e| format!("Failed to lock flow table: {}", e))?;
    
    let flow_entry = flows.flows.iter()
        .find(|(_, flow)| flow.packet_ids.contains(&packet_id));
    
    if let Some((key, flow)) = flow_entry {
        let packet_ids = flow.packet_ids.clone();
        let flow_key = key.clone();
        drop(flows);
        
        let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
        let mut messages: Vec<model::StreamMessage> = Vec::new();
        
        // Fetch raw data for all packets in the flow
        for chunk in packet_ids.chunks(999) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let query = format!(
                "SELECT timestamp_ns, data, source_addr FROM packets WHERE id IN ({}) ORDER BY timestamp_ns ASC",
                placeholders
            );
            let mut stmt = db.prepare(&query).map_err(|e| format!("Prepare failed: {}", e))?;
            
            let i64_ids: Vec<i64> = chunk.iter().map(|&id| id as i64).collect();
            let params: Vec<&dyn rusqlite::ToSql> = i64_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
            
            let rows = stmt.query_map(&*params, |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?, row.get::<_, String>(2)?))
            }).map_err(|e| format!("Query failed: {}", e))?;
            
            for row in rows {
                if let Ok((ts, data, src_addr)) = row {
                    if let Some(payload) = dissector::get_transport_payload(&data) {
                        if payload.is_empty() { continue; }
                        
                        // Determine if this is client -> server
                        // For canonicalized FlowKey, the first address in the 5-tuple is our "client" reference
                        let is_client = src_addr == flow_key.src_ip.to_string();
                        
                        // Merge with last message if from same side
                        if let Some(last) = messages.last_mut() {
                            if last.is_client == is_client {
                                last.data.extend(payload);
                                continue;
                            }
                        }
                        
                        messages.push(model::StreamMessage {
                            is_client,
                            data: payload,
                            timestamp: ts,
                        });
                    }
                }
            }
        }
        Ok(messages)
    } else {
        Err("Flow not found.".to_string())
    }
}

/// Stops the currently active packet capture session.
#[tauri::command]
fn stop_capture(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut stop_tx_guard = state.stop_tx.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    if let Some(tx) = stop_tx_guard.take() {
        tx.try_send(()).map_err(|e| format!("Failed to send stop signal: {}", e))?;
    }
    Ok(())
}

/// Retrieves detailed protocol dissection for a specific packet.
#[tauri::command]
async fn get_packet_detail(
    id: u64,
    state: tauri::State<'_, AppState>
) -> Result<model::PacketDetail, String> {
    // Validate packet ID
    if id == 0 {
        return Err("Invalid packet ID".to_string());
    }
    
    let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
    
    let id_i64 = id as i64;
    let mut stmt = db.prepare("SELECT data FROM packets WHERE id = ?1").map_err(|e| format!("Prepare failed: {}", e))?;
    let packet_data: Option<Vec<u8>> = stmt.query_row([id_i64], |row| row.get(0)).ok();
    
    if let Some(data) = packet_data {
        if let Some(detail) = dissector::dissect_packet(&data, id) {
            Ok(detail)
        } else {
            Err("Failed to dissect packet.".to_string())
        }
    } else {
        Err("Packet not found in database.".to_string())
    }
}

/// Exports selected packets to a PCAP file.
#[tauri::command]
fn export_pcap(
    file_path: String,
    packet_ids: Vec<u64>,
    state: tauri::State<'_, AppState>
) -> Result<usize, String> {
    // Validate packet IDs
    validate_packet_ids(&packet_ids)?;
    
    // Validate file path to prevent path traversal
    let path = validate_export_path_new(&file_path)?;

    let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
    
    // Fetch packets from DB in chronological order
    let mut packet_list = Vec::new();
    for chunk in packet_ids.chunks(999) { // SQLite bind limit is typically 999
        let placeholders = vec!["?"; chunk.len()].join(",");
        let query = format!("SELECT id, timestamp_ns, data FROM packets WHERE id IN ({}) ORDER BY id ASC", placeholders);
        let mut stmt = db.prepare(&query).map_err(|e| format!("Prepare failed: {}", e))?;
        
        let i64_ids: Vec<i64> = chunk.iter().map(|&id| id as i64).collect();
        let params: Vec<&dyn rusqlite::ToSql> = i64_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        
        let rows = stmt.query_map(&*params, |row| {
            let id_i64: i64 = row.get(0)?;
            let id = id_i64 as u64;
            let timestamp_ns: i64 = row.get(1)?;
            let data: Vec<u8> = row.get(2)?;
            Ok((id, timestamp_ns, data))
        }).map_err(|e| format!("Query failed: {}", e))?;
        
        for row in rows {
            if let Ok((id, timestamp_ns, data)) = row {
                if let Some(summary) = dissector::parse_summary(&data, id, timestamp_ns) {
                    packet_list.push((summary, data, timestamp_ns));
                }
            }
        }
    }
    
    if packet_list.is_empty() {
        return Err("No valid packets found in database".to_string());
    }
    
    packet_list.sort_by_key(|p| p.0.id);
    
    let exported_count = packet_list.len();
    export::export_pcap_db(&packet_list, path)?;
    
    Ok(exported_count)
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
            data BLOB NOT NULL
        )",
        [],
    )?;

    // Create an index on id for fast pagination
    conn.execute("CREATE INDEX IF NOT EXISTS idx_packets_id ON packets(id)", [])?;

    // Optimizations for write-heavy workload
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
    ")?;

    Ok(conn)
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
            get_stream_content
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("Failed to start Tauri application: {}", error);
            eprintln!("Failed to start Tauri application: {}", error);
            std::process::exit(1);
        });
}
