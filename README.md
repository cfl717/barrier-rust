# Barrier Rust 项目 README

[![License: GPL-2.0](https://img.shields.io/badge/License-GPL--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/Status-Alpha-yellow.svg)]()

## Barrier

使用 Rust 重写的 Barrier，目标是通过共享 Rust Core + 平台原生前端，实现跨多台计算机共享鼠标和键盘。

## 当前架构

- `protocol/`: Barrier 协议与网络通信
- `input-capture/`: 输入捕获与平台注入
- `app-core/`: 共享应用核心
- `gui-gtk/`: Ubuntu 原生前端（GTK4 + libadwaita）
- `app-core-ffi/`: macOS 桥接层（C ABI）
- `gui-macos/`: macOS 原生前端骨架（SwiftUI / AppKit）

## 快速开始

### 前置要求

- Rust 1.70+
- Linux 原生前端依赖：

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libgtk-4-dev \
    libadwaita-1-dev \
    libssl-dev
```

### 运行协议层示例

```bash
cd protocol
cargo run --example server
```

```bash
cd protocol
cargo run --example client
```

### 运行 Ubuntu 原生前端

```bash
cd gui-gtk
cargo run
```

### 构建 macOS 桥接库

```bash
cd app-core-ffi
cargo build --release
```

## 开发说明

- 所有 GUI 业务逻辑统一收敛到 `app-core`
- Ubuntu 前端直接进程内调用 `app-core`
- macOS 前端通过 `app-core-ffi` 调用共享核心
- `docs/native-migration/api-contract.md` 定义跨前端共享契约

## 测试

```bash
cargo test -p app-core
```

```bash
cargo test -p protocol
```

## 状态

- 协议层：可用
- 共享核心：已抽离
- Ubuntu 原生前端：MVP 已接入
- macOS 原生前端：桥接骨架已接入

## 许可证

GPL-2.0 License
