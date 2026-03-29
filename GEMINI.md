# AuraCap (r-wire) Context & Guidelines

AuraCap is a modern, cross-platform network packet analyzer (similar to Wireshark) built with **Tauri**, **SvelteKit**, and **Rust**.

## Project Overview

- **Architecture**: 
  - **Backend (Rust)**: Handles low-level packet capture (`pcap` crate), protocol parsing (`pnet` crate), and state management.
  - **Frontend (SvelteKit 5)**: Provides a responsive dashboard, packet list, protocol tree, hex view, and real-time statistics.
  - **Communication**: Uses Tauri commands for control (start/stop/export) and Tauri events (`new_packet_batch`) for streaming captured packets to the frontend.
- **Key Files**:
  - `src-tauri/src/main.rs` & `lib.rs`: Entry point and Tauri command handlers.
  - `src-tauri/src/capture.rs`: The core packet capture loop and batching logic.
  - `src-tauri/src/dissector.rs`: Protocol analysis and summary parsing.
  - `src/lib/stores.ts`: Central state management using Svelte stores.
  - `src/lib/utils/filter.ts`: Packet filtering logic.
  - `src/lib/utils/statistics.ts`: Real-time traffic analysis.

## Development Workflows

### Building and Running

- **Development**: `npm run tauri dev`
  - *Note*: On macOS and Linux, packet capture usually requires root privileges: `sudo npm run tauri dev`.
- **Production Build**: `npm run tauri build`
- **Linting & Types**: `npm run check`
- **Rust Tests**: `cd src-tauri && cargo test`

### Key Commands (Tauri)
- `list_interfaces`: Enumerates available network devices.
- `start_capture(interface)`: Begins asynchronous capture in a background task.
- `stop_capture`: Signals the background task to terminate.
- `get_packet_detail(id)`: Requests deep dissection of a cached packet.
- `export_pcap(path, ids)`: Saves selected packets to a standard PCAP file.

## Technical Standards & Conventions

### Backend (Rust)
- **Concurrency**: Uses `tokio` for async tasks and `std::thread` for the blocking `pcap` capture loop.
- **Memory Management**: 
  - Packet data is cached in a `BTreeMap` within `AppState` (`Arc<Mutex<AppState>>`).
  - Cache size is limited to `MAX_CACHE_SIZE` (100,000 packets) to prevent memory exhaustion.
- **Error Handling**: Uses `Result<T, String>` for Tauri commands to provide clear error messages to the frontend.
- **Logging**: Uses `env_logger` and `log` crate.

### Frontend (SvelteKit)
- **Framework**: Svelte 5. Uses standard Svelte stores (`writable`, `derived`) for state.
- **Performance**: 
  - Packets are emitted in batches (default 50 packets or 250ms) to minimize IPC overhead.
  - The frontend packet list is capped at `MAX_FRONTEND_PACKETS` (50,000) to maintain UI responsiveness.
  - Heavy operations (filtering, statistics) are performed in `derived` stores to ensure reactivity without redundant calculations.
- **Styling**: Modern UI with protocol-specific color coding.

### Protocol Support
- Currently supports Ethernet, IPv4, IPv6, TCP, UDP, and ICMP.
- Summary parsing provides quick info for the packet list; full dissection provides a hierarchical tree view.

## Troubleshooting

- **Permission Errors**: If capture fails, ensure the user has access to `/dev/bpf*` (macOS) or `CAP_NET_RAW` (Linux). Running with `sudo` is the common development workaround.
- **IPC Bottlenecks**: If the UI lags during high-traffic captures, consider increasing the `BATCH_SIZE` or decreasing the `MAX_FRONTEND_PACKETS` in `src/lib/stores.ts`.
