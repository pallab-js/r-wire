# AuraCap - Professional Network Analyzer

<div align="center">

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform: macOS](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-black)](https://apple.com/mac)
[![CI](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/pallab-js/r-wire?include_prereleases&label=latest)](https://github.com/pallab-js/r-wire/releases)

**AuraCap** is a free, open-source network packet analyzer built for macOS Apple Silicon. Designed for network engineers, cybersecurity professionals, and beginners who need powerful packet analysis without Wireshark's complexity.

</div>

---

## Why AuraCap?

| Aspect             | Wireshark              | AuraCap                                |
| ------------------ | ---------------------- | -------------------------------------- |
| **Platform**       | Cross-platform         | macOS-native (Apple Silicon optimized) |
| **Learning Curve** | Steep                  | Beginner-friendly                      |
| **Interface**      | Complex menus          | Clean, focused UI                      |
| **Startup**        | Slow, heavy            | Instant launch                         |
| **Design**         | Dated                  | Modern dark theme                      |
| **Dependencies**   | Requires WinPcap/Npcap | Built-in (Tauri)                       |
| **Price**          | Free (but complex)     | Free & open-source                     |

AuraCap provides professional-grade packet analysis in a fraction of the complexity—without sacrificing the power experts need.

---

## Who Is AuraCap For?

### Network Engineers

- Debug network issues faster with instant packet summaries
- Filter traffic with simple display filters
- Export captures for further analysis in Wireshark

### Cybersecurity Professionals

- Analyze suspicious traffic on-the-fly
- Inspect payloads (JSON, JWT, hex)
- Follow TCP/UDP streams in human-readable form

### Students & Beginners

- Learn networking without overwhelming details
- Natural language packet explanations
- No prior experience required

---

## Quick Start

### Download

No pre-built release has been published yet — build from source below.

> CI builds a macOS `.dmg` and attaches it to a **draft** GitHub Release when a `v*` tag is pushed. Windows and Linux installers are not built yet.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/pallab-js/r-wire.git
cd r-wire

# Install dependencies
npm install

# Run in development mode (requires sudo for packet capture)
sudo npm run tauri dev

# Build for production
npm run tauri build
```

> **Note**: Packet capture requires elevated privileges. Run with `sudo` on macOS/Linux or as Administrator on Windows.

---

## Features

### Core Capabilities

- **Real-time capture** from any network interface
- **Instant filtering** with display filters (e.g., `protocol:tcp`, `port:443`)
- **BPF support** for advanced capture filters
- **Export to PCAP** for external analysis

### Packet Analysis

- **Packet list** with virtual scrolling (handles millions of packets)
- **Packet details** with protocol layer breakdown
- **Hex view** for raw byte inspection
- **Follow Stream** for TCP/UDP conversation reassembly

### Professional Tools

- **Protocol statistics** (traffic rates, protocol distribution)
- **Interface selection** for multi-NIC environments
- **Artifact extraction** from packets

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  SvelteKit  │  │  Tailwind  │  │   TypeScript        │ │
│  │   (UI)      │  │  (Style)  │  │   (Logic)          │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└──────────────────────────┬──────────────────────────────────┘
                           │ Tauri IPC
┌──────────────────────────┴──────────────────────────────────┐
│                         Backend                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │    Rust     │  │   PCAP     │  │      SQLite         │ │
│  │   (Core)    │  │ (Capture)  │  │    (Storage)        │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

**Tech Stack:**

- **Frontend**: SvelteKit, TypeScript, Tailwind CSS
- **Backend**: Rust, pcapcapture, SQLite
- **Desktop**: Tauri 2.x (native performance)

---

## Design Philosophy

AuraCap follows the [Supabase-inspired design system](DESIGN.md):

- **Dark-mode-only** - Optimized for long analysis sessions
- **Emerald accents** - Subtle brand color for key actions
- **Minimalist** - Every pixel serves a purpose
- **Performance-first** - Virtual scrolling handles millions of packets

---

## Development

```bash
# Run tests
npm run test:unit && cd src-tauri && cargo test

# Lint and format
npm run lint
npm run format

# Type check
npm run check
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

## Roadmap

See [ROADMAP.md](ROADMAP.md) for upcoming features.

---

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

<div align="center">

**AuraCap** — Network analysis for everyone.

_Made with precision for macOS Apple Silicon._

</div>
