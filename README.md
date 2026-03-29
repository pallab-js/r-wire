# AuraCap Network Analyzer

[![CI](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](#)

A modern, high-performance network packet analyzer and forensic suite built with **Tauri**, **SvelteKit 5**, and **Rust**. AuraCap is designed as a professional, standalone, and local-first alternative to traditional network tools, offering deep technical introspection with a clean, interactive user experience.

## 🚀 Professional Forensic Features

### 🔍 Advanced Introspection
- **Interactive Tree-Hex Mapping**: Synchronized protocol tree and hex view. Hovering over a field instantly highlights its raw binary representation.
- **Conversational "Follow Stream"**: Reassemble bidirectional TCP and UDP conversations into clean request/response views.
- **Intelligent Payload Inspectors**: Automatic local decoding and beautification for **JSON**, **JWT**, and plain-text payloads.
- **Expert Info System**: Automatic deterministic detection of protocol anomalies (e.g., TCP Zero Window, Low TTL).

### 📊 Visual Storytelling
- **Sequence Diagrams**: Visual ladder logic diagrams for conversational flows between clients and servers.
- **Flow Timeline**: High-resolution temporal distribution of packets within a stream to identify bursts or latency.
- **Network Topology Map**: Real-time macro-level conversational map of top network talkers.
- **Shannon Entropy Analysis**: Algorithmic randomness calculation to identify encrypted or malicious payloads.

### 💼 Professional UI/UX
- **Resizable Split-Pane Dashboard**: Fully adjustable vertical and horizontal layouts for a customized workspace.
- **SQLite-Backed Virtualization**: Capable of handling millions of packets with a constant, lean memory footprint.
- **Modern Dark Theme**: Optimized for long-term technical analysis with VS Code-inspired syntax highlighting.

## 🛡️ Core Philosophy: Independent & Standalone
- **Local-First**: All dissection, reassembly, and intelligence logic happens 100% on your machine.
- **Zero Cloud/AI**: Deterministic, verifiable algorithms based on RFC specifications. No external APIs, no tracking, no "black-box" models.
- **Standalone Binary**: Native performance without Docker or container overhead.
- **Private & Public Network Ready**: Works flawlessly in secure air-gapped environments or public networks.

## Prerequisites

- **Node.js** 18+ and npm
- **Rust** (latest stable version)
- **System Dependencies**:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libpcap-dev` or equivalent
  - **Windows**: Npcap

## Installation & Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/pallab-js/r-wire.git
   cd r-wire
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Run in development mode**:
   ```bash
   # Note: Packet capture usually requires sudo/admin privileges
   sudo npm run tauri dev
   ```

## Technical Stack

- **Frontend**: SvelteKit 5.x, TypeScript, Vitest
- **Backend**: Rust (Tauri), `pcap`, `pnet`, `rusqlite`
- **Database**: Local SQLite for high-performance indexing and pagination
- **Styling**: Tailwind CSS (Tailwind 4)

## Testing

AuraCap maintains high technical integrity through a comprehensive automated test suite:

```bash
# Run full automated test suite
npm run test:unit && cd src-tauri && cargo test
```

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
