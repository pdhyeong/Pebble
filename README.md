# Pebble

**Secure P2P File Synchronization Service**

Pebble is a privacy-focused, peer-to-peer file synchronization application built with Flutter and Rust. It enables seamless file sharing across devices on the same network with enterprise-grade security.

## ✨ Features

### Phase 1: File Monitoring ✅
- **Real-time File Watching**: Automatically detect file changes using `notify`
- **File Integrity**: Calculate file hashes with BLAKE3
- **SQLite Database**: Track file metadata and sync status
- **Background Processing**: Non-blocking file monitoring with Tokio

### Phase 2: Device Discovery ✅
- **UDP Broadcast**: Discover devices on the same LAN automatically
- **HMAC-SHA256 Authentication**: Secure message signing
- **Replay Attack Prevention**: Timestamp-based validation
- **Auto Timeout**: Remove offline devices after 15 seconds

### Phase 3: Secure File Transfer ✅
- **TLS 1.3 Encryption**: End-to-end encrypted file transfers
- **Self-Signed Certificates**: Automatic certificate generation with rcgen
- **Certificate Pinning**: MITM attack prevention
- **Chunked Transfer**: 1MB chunks for efficient large file handling
- **Resume Support**: Continue interrupted transfers from last chunk
- **Flow Control**: Network bandwidth management

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Flutter UI Layer                       │
│         (Cross-platform: iOS, Android, macOS)           │
└────────────────────┬────────────────────────────────────┘
                     │ FFI (flutter_rust_bridge)
┌────────────────────▼────────────────────────────────────┐
│                  Rust Core Layer                        │
├─────────────────────────────────────────────────────────┤
│  • File Watcher (notify)                                │
│  • Discovery Service (UDP + HMAC)                       │
│  • Transfer Engine (TLS 1.3 + Tokio)                    │
│  • Database (SQLite)                                    │
│  • Cryptography (BLAKE3, HMAC-SHA256)                   │
└─────────────────────────────────────────────────────────┘
```

## 🛠️ Technology Stack

### Frontend
- **Flutter**: Cross-platform UI framework
- **Dart**: Programming language

### Backend (Rust)
- **tokio**: Async runtime
- **rustls**: TLS 1.3 implementation
- **notify**: File system watcher
- **rusqlite**: SQLite database
- **blake3**: Cryptographic hashing
- **rcgen**: Certificate generation
- **hmac + sha2**: Message authentication

## 🚀 Getting Started

### Prerequisites

- Flutter SDK (3.0+)
- Rust (1.70+)
- Cargo

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/pebble.git
cd pebble

# Install dependencies
flutter pub get

# Build Rust library
cd rust
cargo build --release
cd ..

# Run the app
flutter run
```

## 🧪 Testing

### Discovery Test (Same Machine)

```bash
cd rust

# Terminal 1
cargo run --release --bin test_discovery -- "Device-A"

# Terminal 2 (new terminal)
cargo run --release --bin test_discovery -- "Device-B"
```

### File Transfer Test

```bash
# Create test file
dd if=/dev/urandom of=/tmp/test_file.bin bs=1048576 count=10

# Terminal 1 - Receiver
cargo run --release --bin test_transfer -- receiver

# Terminal 2 - Sender
cargo run --release --bin test_transfer -- sender 127.0.0.1 /tmp/test_file.bin
```

See [rust/TEST_GUIDE.md](rust/TEST_GUIDE.md) for detailed testing instructions.

## 📁 Project Structure

```
pebble/
├── lib/                    # Flutter application code
├── rust/                   # Rust core library
│   ├── src/
│   │   ├── api/           # API modules
│   │   │   ├── db.rs      # SQLite database
│   │   │   ├── watcher.rs # File monitoring
│   │   │   ├── discovery.rs # Device discovery
│   │   │   ├── transfer.rs  # File transfer
│   │   │   ├── certificate.rs # TLS certificates
│   │   │   └── integrity.rs   # File hashing
│   │   └── bin/           # Test programs
│   │       ├── test_discovery.rs
│   │       └── test_transfer.rs
│   ├── Cargo.toml
│   └── TEST_GUIDE.md      # Testing documentation
├── android/               # Android platform code
├── ios/                   # iOS platform code
├── macos/                 # macOS platform code
└── README.md
```

## 🔐 Security Features

- ✅ **TLS 1.3**: Modern encryption protocol
- ✅ **Certificate Pinning**: Prevent MITM attacks
- ✅ **HMAC-SHA256**: Message authentication
- ✅ **BLAKE3**: Fast cryptographic hashing
- ✅ **Replay Protection**: Timestamp validation
- ✅ **Chunk Verification**: Per-chunk integrity checks

## 🗺️ Roadmap

### Phase 4: Conflict Resolution (Planned)
- [ ] Vector Clock for version control
- [ ] CRDT for conflict-free merging
- [ ] Manual conflict resolution UI

### Phase 5: Production Features (Planned)
- [ ] NAT Traversal (STUN/TURN)
- [ ] Multi-peer synchronization
- [ ] File compression (zstd)
- [ ] Access control lists
- [ ] Audit logging

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📧 Contact

For questions and support, please open an issue on GitHub.

---

**Built with ❤️ using Flutter and Rust**
