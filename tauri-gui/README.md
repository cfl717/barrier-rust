# Phase 3: Tauri GUI Integration

## Overview
This phase implements the modern web-based GUI for Barrier using Tauri (Rust backend + React frontend).

## Project Structure
```
tauri-gui/
├── src/                    # React frontend source
│   ├── components/         # Reusable UI components
│   ├── pages/              # Page components
│   ├── styles/             # CSS stylesheets
│   ├── App.tsx             # Main application component
│   └── main.tsx            # Entry point
├── src-tauri/              # Tauri Rust backend
│   ├── src/
│   │   └── main.rs         # Rust entry point & commands
│   ├── Cargo.toml          # Rust dependencies
│   ├── build.rs            # Build script
│   ├── tauri.conf.json     # Tauri configuration
│   └── icons/              # Application icons
├── package.json            # Node.js dependencies
├── tsconfig.json           # TypeScript configuration
├── vite.config.ts          # Vite build configuration
└── index.html              # HTML entry point
```

## Features Implemented

### Frontend (React + TypeScript)
- ✅ Modern responsive UI with gradient theme
- ✅ Server/Client mode selection
- ✅ Real-time status monitoring
- ✅ Activity log panel
- ✅ Server address configuration for client mode
- ✅ Start/Stop controls with visual feedback
- ✅ Auto-refresh status every 2 seconds

### Backend (Rust + Tauri)
- ✅ IPC command handlers:
  - `start_server` - Start barrier server
  - `start_client` - Start barrier client
  - `stop_service` - Stop running service
  - `get_status` - Get current service status
- ✅ Shared state management with Tokio mutex
- ✅ Background task spawning for server/client
- ✅ Logging integration with env_logger
- ✅ Clipboard and drag-drop support flags

### Configuration
- ✅ Tauri security allowlist configured
- ✅ File system access enabled
- ✅ Clipboard API access enabled
- ✅ Dialog APIs enabled
- ✅ Window configuration (1024x768, resizable)

## Dependencies

### Frontend
- React 18.2
- TypeScript 5.3
- Vite 5.0
- @tauri-apps/api 1.5

### Backend
- Tauri 1.5
- Tokio (async runtime)
- Serde (serialization)
- barrier-protocol (Phase 1 module)
- env_logger

## Development Commands

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## IPC API Reference

### `start_server(config: ServerConfig)`
Start the barrier server.

**Parameters:**
- `address`: Listen address (default: "0.0.0.0:24800")
- `enable_clipboard`: Enable clipboard sharing
- `enable_drag_drop`: Enable file drag-drop

### `start_client(config: ClientConfig)`
Start the barrier client.

**Parameters:**
- `server_address`: Server IP:port
- `client_name`: Client identifier
- `enable_clipboard`: Enable clipboard sharing
- `enable_drag_drop`: Enable file drag-drop

### `stop_service()`
Stop the currently running server or client.

### `get_status() -> AppState`
Get current service status.

**Returns:**
- `mode`: "server" or "client"
- `is_running`: Boolean status
- `connected_clients`: Number of connected clients (server mode)
- `server_address`: Current address
- `log_messages`: Recent log entries

## Security Considerations

1. **CSP**: Content Security Policy should be configured for production
2. **Allowlist**: Only necessary APIs are enabled
3. **Input Validation**: All IPC inputs should be validated
4. **File Access**: Scoped to user's home directory

## Next Steps (Phase 4)

1. **Complete Platform Implementations**
   - Linux X11 input capture
   - Windows global hooks
   - macOS CGEventTap

2. **Enhanced GUI Features**
   - Configuration editor
   - Screen layout designer
   - Connection encryption setup
   - Advanced logging viewer

3. **Performance Optimization**
   - Input latency measurement
   - Network compression
   - Frame rate optimization

4. **Testing**
   - Cross-platform testing
   - Performance benchmarks
   - Security audit

## Known Limitations

- Platform input capture not yet implemented (stub only)
- No encryption configured (TLS pending)
- Default drop directory uses hardcoded path
- No system tray integration yet
- No auto-start on boot

## Build Requirements

- Node.js 18+
- Rust 1.70+
- Tauri CLI: `cargo install tauri-cli`
- Platform-specific:
  - Linux: `libwebkit2gtk-4.0-dev`, `build-essential`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`
  - Windows: WebView2, Visual Studio C++ tools
  - macOS: Xcode command line tools
