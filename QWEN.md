# AuraCap Network Analyzer — Project Context

## Project Overview

**AuraCap** is a local-first, cross-platform network packet analyzer and forensic suite built with **Tauri 1.x**, **SvelteKit 5**, and **Rust**. It serves as a standalone, professional-grade alternative to tools like Wireshark, offering deep protocol dissection, interactive hex/protocol tree mapping, TCP/UDP stream reassembly, SQLite-backed packet storage, and visual forensic features (sequence diagrams, flow timelines, entropy analysis).

### Key Design Principles

- **Local-first**: All dissection, reassembly, and intelligence logic runs 100% on the machine. No cloud APIs, no external dependencies.
- **Native performance**: Rust backend with `pcap` and `pnet` for packet capture and protocol parsing.
- **Standalone binary**: No Docker or container overhead. Ships as a native desktop app.
- **Privacy**: Works in air-gapped environments. No tracking, no telemetry.

## Technology Stack

| Layer          | Technology                                                     |
| -------------- | -------------------------------------------------------------- |
| **Frontend**   | SvelteKit 5.x, TypeScript, Tailwind CSS 4, Chart.js            |
| **Backend**    | Rust (Tauri 1.x), `pcap`, `pnet`, `tokio`, `rusqlite`          |
| **Database**   | SQLite (local file, `capture.db`)                              |
| **Testing**    | Vitest + Testing Library (frontend), Rust unit tests (backend) |
| **Linting**    | ESLint + TypeScript ESLint + Svelte ESLint + Security ESLint   |
| **Formatting** | Prettier + Prettier Plugin Svelte                              |
| **Build**      | Vite 6, Tauri CLI                                              |

## Project Structure

```
r-wire/
├── src/                          # SvelteKit frontend
│   ├── app.css                   # Global styles + CSS custom properties
│   ├── app.html                  # HTML shell
│   ├── lib/
│   │   ├── components/           # Reusable Svelte components
│   │   │   ├── CaptureControls   # Start/stop capture UI
│   │   │   ├── PacketList        # Paginated packet table
│   │   │   ├── ProtocolTree      # Protocol dissection tree
│   │   │   ├── HexView           # Raw hex viewer
│   │   │   ├── PayloadInspector  # JSON/JWT/text payload decoder
│   │   │   ├── SequenceDiagram   # Visual conversation diagram
│   │   │   ├── FlowTimeline      # Temporal packet timeline
│   │   │   ├── StreamDialog      # "Follow stream" modal
│   │   │   ├── ForensicArtifacts # Export/forensic tools
│   │   │   ├── ForensicIntelligence # Expert info system
│   │   │   ├── ForensicNarrative   # Narrative view
│   │   │   ├── StatisticsPanel   # Network statistics
│   │   │   └── PacketDetailTabs  # Tabbed detail view
│   │   └── stores.ts             # Svelte stores (selectedPacket, etc.)
│   └── routes/
│       ├── +layout.svelte        # Root layout
│       └── +page.svelte          # Main dashboard (split-pane layout)
├── src-tauri/                    # Tauri + Rust backend
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # Tauri configuration
│   ├── build.rs                  # Tauri build script
│   └── src/
│       ├── main.rs               # Tauri entry point
│       ├── lib.rs                # Tauri commands + app state (main logic)
│       ├── capture.rs            # pcap capture loop
│       ├── dissector.rs          # Protocol dissection (Ethernet, IP, TCP, UDP, etc.)
│       ├── model.rs              # Data models (PacketSummary, PacketDetail, etc.)
│       ├── state.rs              # Flow table + connection tracking
│       └── export.rs             # PCAP export logic
├── static/                       # Static assets (favicon, etc.)
├── package.json                  # Node.js dependencies + scripts
├── svelte.config.js              # SvelteKit config (SPA mode via adapter-static)
├── vite.config.js                # Vite config (Tauri dev settings)
├── tsconfig.json                 # TypeScript config
├── vitest-setup.ts               # Vitest setup (jest-dom matchers)
├── .eslintrc.cjs                 # ESLint config (security plugin included)
└── .prettierrc                   # Prettier config
```

## Building and Running

### Prerequisites

- **Node.js 18+** and **npm**
- **Rust** (latest stable)
- **System dependencies**:
  - macOS: Xcode Command Line Tools
  - Linux: `libpcap-dev` or equivalent
  - Windows: Npcap

### Development

```bash
# Install dependencies
npm install

# Start Tauri dev mode (builds frontend + launches desktop app)
# Note: Packet capture usually requires sudo/admin privileges
sudo npm run tauri dev

# Frontend-only dev server (no Tauri backend)
npm run dev
```

### Production Build

```bash
# Build the desktop application
npm run tauri build

# Frontend-only build (static files in build/)
npm run build
npm run preview
```

### Testing

```bash
# Frontend unit tests
npm run test:unit

# Frontend watch mode
npm run test:watch

# Rust backend tests
cd src-tauri && cargo test

# Run all tests
npm run test:unit && cd src-tauri && cargo test
```

### Linting & Type Checking

```bash
# Lint (Prettier + ESLint)
npm run lint

# Format code
npm run format

# Svelte type checking
npm run check
npm run check:watch
```

### Security Auditing

```bash
# Check npm vulnerabilities
npm run audit

# Auto-fix npm vulnerabilities
npm run audit:fix

# Check Rust dependencies (requires cargo-audit)
npm run audit:rust
```

## Architecture

### Frontend → Backend Communication

The app uses **Tauri commands** (Rust functions exposed via `#[tauri::command]`) for all backend operations. Key commands:

- `list_interfaces()` — List available network interfaces
- `start_capture(interface_name, filter)` — Begin packet capture
- `stop_capture()` — Stop active capture
- `get_packets(offset, limit, filter)` — Paginated packet retrieval
- `get_packet_count(filter)` — Packet count for pagination
- `get_packet_detail(id)` — Full protocol dissection for a packet
- `get_flow_packets(packet_id)` — All packets in the same flow
- `get_stream_content(packet_id)` — Reassembled TCP/UDP stream
- `export_pcap(file_path, packet_ids)` — Export selected packets
- `export_pcap_all(file_path)` — Export all packets

### State Management

- **Svelte stores** (`src/lib/stores.ts`): Frontend reactive state (selected packet, highlighted ranges, etc.)
- **Tauri state** (`AppState` in `lib.rs`): Rust-side state (stop signal channel, SQLite connection, flow table, rate limiter)
- **SQLite database**: Persistent packet storage with WAL mode for write-heavy workloads

### Security Model

- **CSP**: Strict Content Security Policy in `tauri.conf.json` — `script-src 'self'`, no `'unsafe-inline'` for scripts
- **Allowlist**: Minimal Tauri permissions — only `shell-open` and `dialog-save`
- **Rate limiting**: Capture operations are rate-limited (max 10/min, min 5s interval)
- **Input validation**: All Tauri commands validate inputs (interface names, BPF filters, file paths, packet IDs, pagination params)
- **Path traversal protection**: Export paths are validated to prevent directory traversal
- **`withGlobalTauri: false`**: Tauri API is not exposed globally; components import from `@tauri-apps/api`

## Design System

The UI follows a **warm minimalism** design language inspired by Cursor's visual identity:

- **Backgrounds**: Warm cream (`#f2f1ed`, `#e6e5e0`) — not pure white
- **Text**: Warm near-black (`#26251e`) with yellow-brown undertone
- **Accent**: Orange (`#f54e00`) for CTAs, warm crimson (`#cf2d56`) for hover/error
- **Success**: Muted teal (`#1f8a65`)
- **Feature colors**: Thinking (peach `#dfa88f`), Grep (sage `#9fc9a2`), Read (blue `#9fbbe0`), Edit (lavender `#c0a8dd`)
- **Borders**: `oklab()` color space for perceptually uniform warm brown borders
- **Typography**: Three-voice system — gothic display (headings), serif (body), mono (code)
- **Spacing**: 8px base unit with fine sub-8px scale (1.5, 2, 2.5, 3, 4, 5, 6px)
- **Border radius**: 8px standard, 9999px (full pill) for tags/buttons

See `DESIGN.md` for the complete design system documentation.

## Coding Conventions

### TypeScript / Svelte

- Use TypeScript for type safety
- Follow Svelte 5 best practices (runes where applicable)
- Components should be focused and single-purpose
- Use Svelte stores for shared state
- Meaningful variable and function names

### Rust

- Follow Rust idioms and `clippy` recommendations
- Use `Result` types for error handling — avoid `unwrap()` in production code
- Add doc comments for public APIs
- Run `cargo fmt` and `cargo clippy` before committing
- Module organization: `capture.rs` (capture logic), `dissector.rs` (protocol parsing), `model.rs` (data structures), `state.rs` (flow tracking), `export.rs` (file export)

### Commit Messages

Use conventional commit format:

- `feat: add new feature`
- `fix: resolve bug`
- `docs: update documentation`
- `refactor: improve code structure`
- `test: add tests`
- `chore: update dependencies`

## Known Issues & Future Work

### Dependency Concerns

- **npm**: `cookie@0.6.0` (via `@sveltejs/kit`) has a low-severity vulnerability. Waiting for upstream fix.
- **Rust**: Tauri 1.x uses GTK3 ecosystem (unmaintained). Migration to Tauri 2.x (GTK4) recommended long-term.

### Platform Notes

- Currently targets **macOS** primarily (`bundle.targets: ["app"]` in tauri.conf)
- Windows and Linux support require additional testing and configuration
