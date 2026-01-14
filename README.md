# Pebble 🪨
**P2P Local Network Device Discovery System**

Pebble is a decentralized file/folder synchronization tool built with **Tauri 2.0** and **libp2p**. It automatically discovers other devices on the same local network (Wi-Fi) without a central server.

## 🚀 Key Features
- **Serverless Discovery**: Uses mDNS protocol to find peers in the local network.
- **Tauri 2.0 Integration**: High-performance Rust backend with a lightweight web-based UI.
- **Real-time Updates**: Notifies the frontend immediately when a new device is found via Tauri's event system.

## 🛠 Tech Stack
- **Framework**: [Tauri 2.0](https://v2.tauri.app/)
- **Networking**: [libp2p](https://libp2p.io/) (v0.52+)
- **Runtime**: Tokio (Async Rust)
- **Frontend**: React / TypeScript (or your choice)

## 🏗 Project Structure
- `src-tauri/src/network/`: Core libp2p swarm engine and mDNS behavior.
- `src-tauri/examples/`: Standalone test nodes for peer discovery testing.

## 🚦 Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/)