# AuraCap Network Analyzer

<div align="center">

[![CI](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](#)
[![Release](https://img.shields.io/github/v/release/pallab-js/r-wire?include_prereleases&label=latest)](#)

**Professional-grade network packet analyzer built for clarity, not complexity.**

_AuraCap is a modern, high-performance network forensics tool designed to make packet analysis accessible to everyone—from network administrators to security enthusiasts._

</div>

---

## Why AuraCap?

| Feature                | Wireshark                 | AuraCap                     |
| ---------------------- | ------------------------- | --------------------------- |
| Beginner-Friendly UI   | ❌ Complex menus          | ✅ Clean, focused interface |
| Instant Packet Summary | ❌ Manual decoding        | ✅ Auto-generated narrative |
| Modern Design          | ❌ Dated UI               | ✅ Warm minimal design      |
| Local-First            | ✅                        | ✅                          |
| No Dependencies        | ❌ Requires WinPcap/Npcap | ✅ Built-in (Tauri)         |

---

## Quick Start

### Download Pre-built Release

```bash
# macOS (DMG)
# Download from: https://github.com/pallab-js/r-wire/releases

# Linux (AppImage)
chmod +x AuraCap*.AppImage && ./AuraCap*.AppImage

# Windows (MSI/EXE)
# Download and run the installer
```

### Build from Source

#### Prerequisites

| OS      | Requirements              |
| ------- | ------------------------- |
| macOS   | Xcode Command Line Tools  |
| Linux   | `libpcap-dev`, WebKit2GTK |
| Windows | Npcap SDK                 |

#### Build Steps

```bash
# 1. Clone the repository
git clone https://github.com/pallab-js/r-wire.git
cd r-wire

# 2. Install dependencies
npm install

# 3. Run in development mode
sudo npm run tauri dev

# 4. Build for production
npm run tauri build
```

> ⚠️ **Note**: Packet capture requires elevated privileges. Run with `sudo` on macOS/Linux or as Administrator on Windows.

---

## Features

### For Beginners

- **Essentials View** - Get instant packet summaries without deep protocol knowledge
- **Natural Language Narrative** - Automatic plain-English explanation of each packet
- **Smart Payload Detection** - Auto-detects and formats JSON, JWT, and text payloads

### For Professionals

- **Protocol Tree** - Full RFC-compliant packet dissection
- **Hex View** - Raw byte inspection with highlighting
- **Artifacts Export** - Export individual packets and artifacts (PDF, JPG, PNG)
- **Follow Stream** - Reassembled TCP/UDP stream view
- **Statistics Panel** - Traffic rate charts, protocol distribution, top talkers

### Design Philosophy

- **Warm Minimalism** - Designed to reduce eye strain during long sessions
- **Theme Toggle** - Light/dark mode support

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Svelte    │  │  Tailwind   │  │      Chart.js       │ │
│  │   (UI)      │  │   (Style)   │  │   (Statistics)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└──────────────────────────┬──────────────────────────────────┘
                           │ Tauri IPC
┌──────────────────────────┴──────────────────────────────────┐
│                         Backend                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Rust      │  │    PCAP     │  │       SQLite        │ │
│  │  (Core)     │  │ (Capture)   │  │    (Storage)        │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Development

```bash
# Run tests
npm run test:unit && cd src-tauri && cargo test

# Lint
npm run lint

# Type check
npm run check

# Format code
npm run format
```

---

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md).

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing`)
5. Open a Pull Request

---

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for upcoming features and future plans.

---

<div align="center">

**Made with ❤️ by the AuraCap Team**

</div>
