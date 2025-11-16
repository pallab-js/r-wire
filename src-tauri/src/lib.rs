pub mod model;
pub mod dissector;
pub mod capture;
pub mod export;

use std::sync::Mutex;
use tokio::sync::mpsc;
use std::collections::BTreeMap;
use std::sync::Arc;
use crate::model::CachedPacket;

// Initialize logging
#[cfg(not(debug_assertions))]
use log::LevelFilter;

pub struct AppState {
    // Sender to signal the capture task to stop
    pub stop_tx: Mutex<Option<mpsc::Sender<()>>>,
    // Use BTreeMap for ordered keys (efficient eviction of oldest packets)
    pub packet_cache: Arc<Mutex<BTreeMap<u64, CachedPacket>>>,
}

/// Lists all available network interfaces for packet capture.
///
/// Returns a vector of interface names that can be used for capturing network packets.
/// This function requires appropriate permissions to access network interfaces.
///
/// # Returns
/// - `Ok(Vec<String>)`: List of available network interface names
/// - `Err(String)`: Error message if interface enumeration fails
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
///
/// This function initiates asynchronous packet capture using libpcap. The capture runs
/// in a background task and emits packet batches to the frontend via Tauri events.
/// Only one capture session can be active at a time.
///
/// # Arguments
/// * `interface_name` - Name of the network interface to capture on
/// * `app_handle` - Tauri app handle for emitting events
/// * `state` - Application state containing capture control structures
///
/// # Returns
/// - `Ok(())`: Capture started successfully
/// - `Err(String)`: Error message if capture cannot be started
///
/// # Events Emitted
/// - `"new_packet_batch"`: Emitted periodically with batches of captured packets
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

    // Clear packet cache
    state.packet_cache.lock().map_err(|e| format!("Failed to clear cache: {}", e))?.clear();

    // Clone the packet cache Arc for the task
    let packet_cache = Arc::clone(&state.packet_cache);

    // Spawn the capture task
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        if let Err(e) = capture::run_capture(app_handle_clone, interface_name, stop_rx, packet_cache).await {
            eprintln!("Capture error: {}", e);
        }
    });

    Ok(())
}

/// Stops the currently active packet capture session.
///
/// Signals the capture task to stop gracefully. Any remaining packets in the
/// current batch will be emitted before the capture fully stops.
///
/// # Arguments
/// * `state` - Application state containing the capture control channel
///
/// # Returns
/// - `Ok(())`: Stop signal sent successfully
/// - `Err(String)`: Error message if stopping fails
#[tauri::command]
fn stop_capture(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut stop_tx_guard = state.stop_tx.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    if let Some(tx) = stop_tx_guard.take() {
        tx.try_send(()).map_err(|e| format!("Failed to send stop signal: {}", e))?;
    }
    Ok(())
}

/// Retrieves detailed protocol dissection for a specific packet.
///
/// Performs full protocol analysis on the raw packet data, breaking it down
/// into protocol layers (Ethernet, IP, TCP/UDP, Application) with field details.
///
/// # Arguments
/// * `id` - Unique identifier of the packet to analyze
/// * `state` - Application state containing the packet cache
///
/// # Returns
/// - `Ok(PacketDetail)`: Detailed packet analysis with protocol layers
/// - `Err(String)`: Error message if packet not found or dissection fails
#[tauri::command]
async fn get_packet_detail(
    id: u64,
    state: tauri::State<'_, AppState>
) -> Result<model::PacketDetail, String> {
    let cache = state.packet_cache.lock().map_err(|e| format!("Failed to lock cache: {}", e))?;
    if let Some(cached) = cache.get(&id) {
        if let Some(detail) = dissector::dissect_packet(&cached.data, id) {
            Ok(detail)
        } else {
            Err("Failed to dissect packet.".to_string())
        }
    } else {
        Err("Packet not found in cache.".to_string())
    }
}

/// Exports selected packets to a PCAP file.
///
/// Creates a standard PCAP file containing the raw packet data for the specified
/// packet IDs. The packets are written in chronological order based on their IDs.
///
/// # Arguments
/// * `file_path` - Path where the PCAP file should be created
/// * `packet_ids` - List of packet IDs to include in the export
/// * `state` - Application state containing the packet cache
///
/// # Returns
/// - `Ok(usize)`: Number of packets successfully exported
/// - `Err(String)`: Error message if export fails
///
/// # PCAP Format
/// Uses libpcap format with microsecond timestamp precision.
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
    
    let cache = state.packet_cache.lock().map_err(|e| format!("Failed to lock cache: {}", e))?;
    let path = PathBuf::from(file_path);
    
    // Build packet list from IDs using actual timestamps from cache
    let mut packet_list = Vec::new();
    for id in &packet_ids {
        if let Some(cached) = cache.get(id) {
            // Use actual timestamp from cache
            if let Some(summary) = dissector::parse_summary(&cached.data, *id, cached.timestamp_ns) {
                packet_list.push(summary);
            }
        }
    }
    
    if packet_list.is_empty() {
        return Err("No valid packets found in cache".to_string());
    }
    
    // Sort by ID to maintain order
    packet_list.sort_by_key(|p| p.id);
    
    let exported_count = packet_list.len();
    export::export_pcap(&cache, &packet_list, path)?;
    
    Ok(exported_count)
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
        .manage(AppState {
            stop_tx: Mutex::new(None),
            packet_cache: Arc::new(Mutex::new(BTreeMap::new())),
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
