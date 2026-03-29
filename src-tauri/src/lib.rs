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

// Initialize logging
#[cfg(not(debug_assertions))]
use log::LevelFilter;

pub struct AppState {
    // Sender to signal the capture task to stop
    pub stop_tx: Mutex<Option<mpsc::Sender<()>>>,
    // SQLite connection for packet storage
    pub db_conn: Arc<Mutex<Connection>>,
    // Global flow table for connection tracking
    pub flow_table: Arc<Mutex<FlowTable>>,
}

/// Retrieves a paginated list of packets, optionally filtered.
#[tauri::command]
async fn get_packets(
    offset: usize,
    limit: usize,
    filter: Option<String>,
    state: tauri::State<'_, AppState>
) -> Result<Vec<model::PacketSummary>, String> {
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
    // Check if already capturing
    let mut stop_tx_guard = state.stop_tx.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    if stop_tx_guard.is_some() {
        return Err("Capture already in progress".to_string());
    }

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
    use std::path::PathBuf;
    
    let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
    let path = PathBuf::from(file_path);
    
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
    let flows = state.flow_table.lock().map_err(|e| format!("Failed to lock flow table: {}", e))?;
    
    // Find the flow key for this packet ID
    let flow_key = flows.flows.iter()
        .find(|(_, flow)| flow.packet_ids.contains(&packet_id))
        .map(|(key, _)| key.clone());
    
    if let Some(key) = flow_key {
        let flow = flows.flows.get(&key).unwrap();
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
    use std::path::PathBuf;
    
    if packet_ids.is_empty() {
        return Err("No packets to export".to_string());
    }
    
    let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
    let path = PathBuf::from(file_path);
    
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
    let mut root_dir = std::env::current_dir()?;
    
    // If we are running from src-tauri (common in dev), move up to project root
    if root_dir.ends_with("src-tauri") {
        if let Some(parent) = root_dir.parent() {
            root_dir = parent.to_path_buf();
        }
    }
    
    let db_path = root_dir.join("capture.db");
    log::info!("Initializing database at: {:?}", db_path);

    let conn = Connection::open(db_path)?;
    
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
