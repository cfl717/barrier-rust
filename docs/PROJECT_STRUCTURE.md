# Barrier Rust 项目结构整理

## 当前主线结构

```text
/workspace/
├── README.md
├── Cargo.toml
├── docs/
│   ├── PROJECT_STRUCTURE.md
│   └── native-migration/api-contract.md
├── protocol/               # 协议与网络通信
├── input-capture/          # 输入捕获与注入
├── app-core/               # 共享应用核心
├── app-core-ffi/           # macOS 桥接层（C ABI）
├── gui-gtk/                # Ubuntu 原生前端（GTK4 + libadwaita）
└── gui-macos/              # macOS 原生前端骨架（SwiftUI / AppKit）
```

## 模块职责

### `protocol/`

- Barrier 消息协议
- TCP 服务端 / 客户端
- 握手流程
- 事件序列化与反序列化

### `input-capture/`

- 平台输入捕获
- 键鼠注入
- 与协议层的事件桥接

### `app-core/`

- 服务生命周期管理
- 运行状态与日志
- 路由切换与转发
- 能力探测
- 配置持久化与启动恢复

### `gui-gtk/`

- Ubuntu 原生桌面 UI
- 直接进程内调用 `app-core`

### `app-core-ffi/`

- 向 Swift / C 暴露共享核心 API
- 作为 macOS 前端的稳定桥接边界

### `gui-macos/`

- SwiftUI / AppKit 最小前端骨架
- 通过 `app-core-ffi` 复用共享核心

## 技术栈

- 后端：Rust、Tokio、Serde、log
- Ubuntu 前端：GTK4、libadwaita
- macOS 前端：SwiftUI / AppKit
- 跨前端共享层：`app-core` / `app-core-ffi`

## 当前状态

- `protocol`：可用
- `app-core`：已抽离
- `gui-gtk`：MVP 已接入
- `gui-macos`：桥接骨架已接入
- 旧 WebView 方案：已从主线移除

## 构建入口

```bash
cargo test -p app-core
```

```bash
cd gui-gtk
cargo run
```
