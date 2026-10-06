# AuraCap

[![CI](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform: macOS Apple Silicon](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-black.svg)](https://www.apple.com/mac/)

**AuraCap** is a free, open-source network packet analyzer for macOS (Apple Silicon), built with
Tauri and Rust. It pairs Wireshark-grade insight with a smaller surface: live capture, display
filters, stream reassembly and PCAP export in a native, dark-themed app.

## Features

- **Live capture** on any interface, with BPF capture filters — memory-bounded (auto-stops at
  768 MB) so a runaway capture can't take the machine down.
- **Display filters** evaluated as you type: `protocol:http`, `port:443`, `ip:10.0.0.1`,
  `src:192.168.1.5`, `dst:…`.
- **Protocol details** for Ethernet, IPv4/IPv6, TCP, UDP, ICMP, DNS and HTTP — with a hex view and
  payload decoding for JSON and JWT.
- **Follow stream** reassembles each TCP direction into readable conversations and reports gaps
  instead of hiding them; UDP stays in capture order.
- **Statistics & risk**: traffic rates, protocol distribution, artifact extraction and
  evidence-based risk scoring.
- **PCAP import/export** to round-trip captures with Wireshark.
- **Built for scale**: virtualized packet list over millions of packets — summaries in memory,
  packets in SQLite.
- **Dark theme only**, one calm high-contrast UI (see [DESIGN.md](DESIGN.md)).

## Quick start

**Requirements:** macOS on Apple Silicon · [Node.js](https://nodejs.org) 22+ ·
[Rust](https://rustup.rs) · root privileges for packet capture

```bash
git clone https://github.com/pallab-js/r-wire.git
cd r-wire
npm install

sudo npm run tauri dev   # development — sudo is required to capture
npm run tauri build      # production build → macOS .dmg
```

No prebuilt installers yet: CI builds a `.dmg` and attaches it to a **draft** GitHub Release
whenever a `v*` tag is pushed. Windows and Linux builds are not set up yet.

## Development

The same checks CI runs on every push:

```bash
npm run lint         # Prettier + ESLint
npm run check        # svelte-check / TypeScript
npm run test:unit    # Vitest (frontend)
npm run build        # production build

cd src-tauri
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

CI also audits dependencies (`cargo audit`, `npm audit`) and tests on a Linux runner plus the
Node 22 and 24 frontend matrix.

## Architecture

| Layer            | Stack                                    | Code                         |
| ---------------- | ---------------------------------------- | ---------------------------- |
| Interface        | SvelteKit · TypeScript · Tailwind CSS    | `src/lib/`, `src/routes/`    |
| Desktop shell    | Tauri 1.x — commands and events over IPC | `src-tauri/src/lib.rs`       |
| Capture          | libpcap (`pcap`) · dissection (`pnet`)   | `capture.rs`, `dissector.rs` |
| Storage & export | SQLite (packets) · in-memory summaries   | `state.rs`, `export.rs`      |

Packet capture requires elevated privileges: the process opens interfaces directly, so both dev
and packaged builds need `sudo`/Administrator.

## Roadmap

Current release: **v0.1.0**. In progress for **v0.2.0**: smart filters, one-click analysis and
filter autocomplete (the guided tour already ships). Further out: TLS decryption. Full plan with
statuses in [ROADMAP.md](ROADMAP.md).

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow
(fork → branch → PR). Please run the development checks above before opening a PR.

## License

MIT — see [LICENSE](LICENSE).
