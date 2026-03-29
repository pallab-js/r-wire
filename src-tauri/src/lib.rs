pub mod model;
pub mod dissector;
pub mod capture;
pub mod export;

use std::sync::Mutex;
use tokio::sync::mpsc;
use std::sync::Arc;
use rusqlite::Connection;

// Initialize logging
#[cfg(not(debug_assertions))]
use log::LevelFilter;

pub struct AppState {
    // Sender to signal the capture task to stop
    pub stop_tx: Mutex<Option<mpsc::Sender<()>>>,
    // SQLite connection for packet storage
    pub db_conn: Arc<Mutex<Connection>>,
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

    // Clear packet table
    {
        let db = state.db_conn.lock().map_err(|e| format!("Failed to lock db: {}", e))?;
        db.execute("DELETE FROM packets", []).map_err(|e| format!("Failed to clear packets: {}", e))?;
    }

    // Clone the DB Arc for the task
    let db_conn = Arc::clone(&state.db_conn);

    // Spawn the capture task
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        if let Err(e) = capture::run_capture(app_handle_clone, interface_name, stop_rx, db_conn).await {
            eprintln!("Capture error: {}", e);
        }
    });

    Ok(())
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

fn init_db() -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open("capture.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS packets (
            id INTEGER PRIMARY KEY,
            timestamp_ns INTEGER NOT NULL,
            data BLOB NOT NULL
        )",
        [],
    )?;
    
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
    
    let db_conn = init_db().expect("Failed to initialize SQLite database");

    tauri::Builder::default()
        .manage(AppState {
            stop_tx: Mutex::new(None),
            db_conn: Arc::new(Mutex::new(db_conn)),
        })
        .invoke_handler(tauri::generate_handler![
            list_interfaces,
            start_capture,
            stop_capture,
            get_packet_detail,
            export_pcap
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("Failed to start Tauri application: {}", error);
            eprintln!("Failed to start Tauri application: {}", error);
            std::process::exit(1);
        });
}
