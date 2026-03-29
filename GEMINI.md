# AuraCap (r-wire) Context & Guidelines

AuraCap is a high-performance network packet analyzer and forensic suite built with **Tauri**, **SvelteKit 5**, and **Rust**.

## Project Overview

- **Architecture**: 
  - **Backend (Rust)**: Handles low-level capture (`pcap`), protocol dissection (`pnet`), and stateful connection tracking.
  - **Storage (SQLite)**: All captured packets are indexed in a local `capture.db` at the project root for high-performance pagination and filtering.
  - **Frontend (SvelteKit 5)**: A professional, resizable dashboard featuring advanced analytical views (Timeline, Sequence, Intelligence).
- **Communication**: Tauri commands for data retrieval and state management; events for real-time packet streaming.

## Key Files & Modules

- `src-tauri/src/lib.rs`: Tauri command hub and SQLite initialization.
- `src-tauri/src/state.rs`: **FlowTable** logic for 5-tuple connection tracking.
- `src-tauri/src/dissector.rs`: Forensic dissection engine (Entropy, OUI lookup, Magic Bytes, Narrative generation).
- `src/lib/stores.ts`: Central state including `highlightedRange` for Tree-Hex sync.
- `src/lib/components/`: Modular forensic tabs (Narrative, Intelligence, Artifacts, Timeline, Sequence).

## Development Workflows

### Testing Standards
Maintain technical integrity by running the full suite:
- **Full Suite**: `npm run test:unit && cd src-tauri && sudo cargo test`
- **Backend**: `cd src-tauri && cargo test`
- **Frontend**: `npm run test:unit`

### Key Commands (Tauri)
- `start_capture(interface, filter)`: Begins capture with optional BPF filter.
- `get_packets(offset, limit, filter)`: Cursor-based pagination from SQLite.
- `get_stream_content(packet_id)`: Reassembles TCP/UDP conversations.
- `get_flow_packets(packet_id)`: Retrieves all summaries in a conversation.

## Technical Standards & Conventions

### Backend (Rust)
- **Statefulness**: Use `FlowTable` for any logic requiring conversational context.
- **Independence**: All intelligence logic must be local and deterministic (No AI/Cloud).
- **Performance**: Use zero-copy parsing where possible; block-based DB writes.

### Frontend (SvelteKit)
- **Virtualization**: Never store full packet arrays in memory. Use `totalFilteredCount` and windowed fetching.
- **Interactivity**: Maintain the Tree-Hex synchronization via the `highlightedRange` store.
- **Accessibility**: All interactive elements must have proper ARIA roles and labels.

## Operational Mandates
- **Database Location**: Always initialize the SQLite database at the project root (relative to `src-tauri` or root) to avoid Tauri's dev-rebuild loop.
- **Standalone Integrity**: Zero external dependencies for core analytical features.
