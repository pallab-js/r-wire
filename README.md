# AuraCap Network Analyzer

[![CI](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/r-wire/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](#)

A modern, cross-platform network packet analyzer built with **Tauri**, **SvelteKit 5**, and **Rust**. AuraCap provides real-time packet capture, analysis, and visualization capabilities with an intuitive graphical interface.

## Features

### 🎯 Core Functionality
- **Real-time Packet Capture**: Capture network traffic from any available interface
- **Multi-Protocol Support**: Analyze TCP, UDP, ICMP, and other network protocols
- **Live Packet List**: View captured packets in real-time with color-coded protocol indicators
- **Advanced Filtering**: Filter packets by protocol, IP address, port, or custom search terms
- **PCAP Export**: Export captured packets to standard PCAP format for analysis in Wireshark or other tools

### 📊 Analysis & Visualization
- **Protocol Tree View**: Hierarchical view of packet layers (Ethernet, IP, TCP/UDP, Application)
- **Hex View**: Raw packet data in hexadecimal and ASCII format
- **Statistics Dashboard**: 
  - Total packets and bytes captured
  - Average packet size
  - Protocol distribution with visual progress bars
  - Top 10 source and destination IP addresses
- **Color-Coded Packets**: Visual protocol identification with color-coded packet rows

### 🎨 User Interface
- **Modern UI**: Clean, responsive interface built with SvelteKit
- **Tabbed Views**: Easy navigation between packet details and statistics
- **Real-time Updates**: Live statistics and packet list updates
- **Intuitive Controls**: Simple start/stop/restart capture controls

## Prerequisites

- **Node.js** 18+ and npm
- **Rust** (latest stable version)
- **System Dependencies**:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libpcap-dev` or equivalent
  - **Windows**: WinPcap or Npcap

## Installation

### Development Setup

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
   npm run tauri dev
   ```
   
   **Note**: On macOS and Linux, you may need to run with `sudo` for packet capture permissions:
   ```bash
   sudo npm run tauri dev
   ```

### Building for Production

Build the application for your platform:

```bash
npm run tauri build
```

The built application will be in `src-tauri/target/release/`.

## Usage

1. **Start the application** (may require administrator/sudo privileges for packet capture)

2. **Select a network interface** from the dropdown (e.g., `en0`, `eth0`, `lo0`)

3. **Click "Start"** to begin capturing packets

4. **View captured packets** in the main list:
   - Click any packet to view detailed information
   - Use the filter box to search/filter packets
   - Switch between "Packet Details" and "Statistics" tabs

5. **Export to PCAP**:
   - Click "Export PCAP" to save captured packets
   - Open the file in Wireshark or other packet analyzers

### Filter Syntax

- `protocol:tcp` - Filter by protocol
- `ip:192.168.1.1` - Filter by IP address
- `port:80` - Filter by port number
- `src:192.168.1.1` - Filter by source IP
- `dst:8.8.8.8` - Filter by destination IP
- General text search - Searches across all packet fields

## Project Structure

```
r-wire/
├── src/                    # Frontend (SvelteKit)
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores.ts      # State management
│   │   └── utils/         # Utility functions
│   └── routes/            # SvelteKit routes
├── src-tauri/             # Backend (Rust/Tauri)
│   ├── src/               # Rust source code
│   └── Cargo.toml         # Rust dependencies
├── static/                # Static assets
└── package.json           # Node.js dependencies
```

## Technology Stack

- **Frontend**: SvelteKit 5.x, TypeScript, Vite 6.x
- **Backend**: Rust, Tauri 1.5
- **Networking**: `pcap`, `pnet`
- **Concurrency**: `tokio`
- **Logging**: `env_logger`
- **Testing**: Vitest (Frontend), Cargo Test (Backend)

## Testing

Run the test suites to ensure code quality:

```bash
# Run Frontend unit tests (Vitest)
npm run test:unit

# Run Frontend type checking (Svelte-check)
npm run check

# Run Backend unit tests (Cargo)
cd src-tauri && cargo test

# Run Rust formatting check
cd src-tauri && cargo fmt -- --check

# Run Rust linting (Clippy)
cd src-tauri && cargo clippy -- -D warnings
```

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/) for cross-platform desktop applications
- Uses [pcap](https://github.com/rust-pcap/pcap) and [pnet](https://github.com/libpnet/libpnet) for packet capture and parsing

## Support

For issues, questions, or contributions, please open an issue on the [GitHub repository](https://github.com/pallab-js/r-wire/issues).
