# AuraCap Agent Instructions

## Project Overview

- **Type**: Desktop app (Tauri + SvelteKit + Rust)
- **Platform**: macOS primary, Windows/Linux via CI
- **License**: MIT

## Key Commands

```bash
# Development
npm run tauri dev          # Run in dev mode (requires sudo on macOS for packet capture)

# Build & Test
npm run check              # TypeScript/Svelte type check
npm run lint               # ESLint + Prettier check
npm run format             # Auto-format code
npm run test:unit          # Vitest frontend tests

# Rust
cd src-tauri && cargo test        # Rust unit tests
cd src-tauri && cargo clippy     # Rust lints
cd src-tauri && cargo fmt        # Rust format

# Full verification
npm run check && npm run test:unit && cd src-tauri && cargo test && cargo clippy
```

## Architecture

```
Frontend (SvelteKit/TypeScript)
├── src/lib/components/   # UI components
├── src/lib/stores.ts    # Svelte stores (state management)
├── src/lib/utils/        # Filter, statistics utilities
└── src/routes/          # SvelteKit routes

Backend (Rust/Tauri)
├── src-tauri/src/lib.rs      # Tauri commands, capture logic
├── src-tauri/src/capture.rs  # Packet capture (pcap)
├── src-tauri/src/dissector.rs # Protocol dissection
├── src-tauri/src/state.rs     # Flow tracking
└── src-tauri/src/export.rs    # PCAP export
```

## Critical Notes

### Memory Management

- Capture auto-stops at 768MB memory usage (prevents crash on 8GB systems)
- Warning at 512MB
- Snaplen capped at 1600 bytes (reduces memory per packet)

### Packet Capture

- Requires elevated privileges: `sudo npm run tauri dev`
- Packets stored in SQLite, summaries in memory
- Max 5 million packets before auto-stop

### UI Design System

- Theme toggle in header (light/dark)
- Uses CSS variables from `app.css`
- Dark mode: set `data-theme="dark"` on `<html>`

### Protocol Detection

- TCP/UDP ports determine app-layer protocol (HTTP, DNS, HTTPS, etc.)
- Port 80 → HTTP, Port 443 → HTTPS, Port 53 → DNS
- JSON detection via payload content inspection

## Testing

- Frontend: Vitest in `src/**/*.test.ts`
- Backend: Rust `#[test]` functions
- No integration tests for Tauri IPC

## Common Issues

- **Build cache corruption**: `sudo rm -rf src-tauri/target` then `cargo build`
- **LSP errors about adapter-static**: Ignore - LSP config issue, not a real error
- **Missing type exports**: Check `src/lib/stores.ts` for actual exports
- **Permission denied on cargo**: Clear target directory with sudo
- **Rust borrow checker**: Strings in match arms need `.clone()` when used multiple times

## File Conventions

- Components: PascalCase `.svelte` files
- Utilities: camelCase `.ts` files
- Rust: snake_case `.rs` files
- CSS: Tailwind + custom CSS variables in `app.css`

## Roadmap Context

- v0.2.0: Current release (beginner UX, dark mode, enhanced protocols)
- v0.3.0: TLS decryption, bookmarks (future)
- TLS decryption requires SSLKEYLOGFILE or private key - complex, not yet implemented
