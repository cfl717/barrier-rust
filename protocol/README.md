# Barrier Protocol Rust Implementation - Phase 1

## Overview

This crate implements the Barrier network protocol in Rust, providing a safe and efficient foundation for cross-platform mouse and keyboard sharing.

## Features

- ✅ **Protocol Message Types**: All Barrier message types (handshake, clipboard, input events)
- ✅ **Serialization/Deserialization**: Binary protocol with checksum verification
- ✅ **Async Network Layer**: Tokio-based async TCP client and server
- ✅ **Handshake Protocol**: Complete client/server handshake implementation
- ✅ **Clipboard Support**: Text, HTML, RTF, BMP, PNG formats
- ✅ **Input Events**: Mouse move, button, wheel; keyboard key down/up
- 🚧 **Platform Input Capture**: Stub implementations (Phase 2)
- 🚧 **GUI Integration**: Native frontends via shared `app-core` (Phase 3)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    barrier_protocol                          │
├─────────────────────────────────────────────────────────────┤
│  protocol/                                                   │
│    ├── message.rs      # Core message types & serialization │
│    ├── handshake.rs    # Client/server handshake            │
│    ├── events.rs       # Mouse/keyboard events              │
│    └── clipboard.rs    # Clipboard data exchange            │
├─────────────────────────────────────────────────────────────┤
│  network/                                                    │
│    ├── server.rs       # Async TCP server                   │
│    ├── client.rs       # Async TCP client                   │
│    └── connection.rs   # Connection wrapper                 │
├─────────────────────────────────────────────────────────────┤
│  platform/                                                   │
│    ├── linux.rs        # Linux input (X11/Wayland)          │
│    ├── windows.rs      # Windows input (Win32)              │
│    └── macos.rs        # macOS input (Cocoa)                │
└─────────────────────────────────────────────────────────────┘
```

## Installation

```bash
cargo add barrier_protocol
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
barrier_protocol = "0.1.0"
```

## Usage Examples

### Running a Server

```rust
use barrier_protocol::{BarrierServer, ServerConfig, ServerEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::default();
    let (mut server, mut event_rx) = BarrierServer::new(config);
    
    // Spawn event handler
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                ServerEvent::ClientConnected(client) => {
                    println!("Client connected: {}", client.screen_name);
                }
                _ => {}
            }
        }
    });
    
    server.run().await?;
    Ok(())
}
```

### Running a Client

```rust
use barrier_protocol::{BarrierClient, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig {
        server_addr: "192.168.1.100:24800".to_string(),
        screen_name: "my-client".to_string(),
        auto_reconnect: true,
        reconnect_interval: 5,
        ..Default::default()
    };
    
    let mut client = BarrierClient::new(config);
    client.run().await?;
    Ok(())
}
```

### Message Serialization

```rust
use barrier_protocol::{Message, MessageType, MouseMoveEvent, ModifierKeys};

// Create a mouse move event
let event = MouseMoveEvent::new(10, -5, ModifierKeys::CONTROL);
let msg = event.to_message();

// Serialize to binary
let bytes = msg.serialize()?;

// Deserialize from binary
let parsed = Message::from_bytes(&bytes)?;
let parsed_event = MouseMoveEvent::from_message(&parsed)?;
```

## Examples

Run the provided examples:

```bash
# Message serialization demo
cargo run --example message_demo

# Start a server
cargo run --example server

# Start a client
cargo run --example client
```

## Protocol Specification

### Message Format

```
┌─────────────┬─────────────┬─────────────┬─────────────┐
│  Size (4B)  │ Type (4B)   │  Data...    │  Checksum   │
└─────────────┴─────────────┴─────────────┴─────────────┘
```

- **Size**: Total size including type, data, and checksum (big-endian u32)
- **Type**: 4-character message type code (big-endian u32)
- **Data**: Variable-length payload
- **Checksum**: XOR of all data bytes (u32)

### Common Message Types

| Code | Name | Direction | Description |
|------|------|-----------|-------------|
| `CBYQ` | HELLO_SERVER | C→S | Client hello |
| `QBYC` | HELLO_CLIENT | S→C | Server hello |
| `CLAP` | CLIPBOARD | Both | Clipboard data |
| `CGRP` | CLIPBOARD_GRAB | Both | Clipboard grab notification |
| `CCLR` | CLIPBOARD_CLEAR | Both | Clipboard cleared |
| `CMOV` | MOUSE_MOVE | S→C | Mouse movement |
| `CBUT` | MOUSE_BUTTON | S→C | Mouse button press/release |
| `CKDn` | KEY_DOWN | S→C | Key pressed |
| `CKUp` | KEY_UP | S→C | Key released |
| `CINN` | CLIENT_ENTER | S→C | Client entered screen |
| `COUT` | CLIENT_LEAVE | S→C | Client left screen |

## Development Roadmap

### Phase 1: Protocol Scaffold (Current)
- [x] Core message types
- [x] Serialization/deserialization
- [x] Handshake protocol
- [x] Async network layer
- [x] Basic examples

### Phase 2: Input Capture
- [ ] Linux X11 input capture
- [ ] Linux Wayland input capture
- [ ] Windows Win32 hooks
- [ ] macOS Cocoa event tap
- [ ] Clipboard integration

### Phase 3: Native GUI
- [ ] Ubuntu GTK4/libadwaita frontend
- [ ] macOS SwiftUI/AppKit frontend
- [ ] Configuration UI
- [ ] Connection status display

### Phase 4: Production Ready
- [ ] Performance optimization
- [ ] Security hardening
- [ ] Comprehensive testing
- [ ] Documentation
- [ ] Packaging and distribution

## License

GPL-2.0 (same as original Barrier)

## Contributing

Contributions welcome! Please see the main repository for guidelines.
