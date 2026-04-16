# Barrier macOS Native Frontend (最小桥接版)

本目录提供 SwiftUI/AppKit 入口样例，通过 `app-core-ffi` 调用共享 Rust Core。

## 目标

- 验证 macOS 前端可复用 `app-core`
- 保持与 Ubuntu 原生前端相同的核心命令语义
- 为后续完整 Xcode 工程迁移提供骨架

## 目录说明

- `BarrierMacApp.swift`: SwiftUI 最小 UI
- `BarrierCoreBridge.swift`: Swift 到 C ABI 的桥接
- `include/barrier_core.h`: C 头文件

## 构建 Rust FFI 库

```bash
cd app-core-ffi
cargo build --release
```

产物（示例）：

- `target/release/libbarrier_core_ffi.dylib`
- `target/release/libbarrier_core_ffi.a`

## Xcode 接入建议

1. 创建 macOS App 工程（SwiftUI 生命周期）
2. 将 `gui-macos/include/barrier_core.h` 配到 Bridging Header
3. 将 `libbarrier_core_ffi.dylib` 或静态库加入 Link Binary With Libraries
4. 加入 `BarrierMacApp.swift` / `BarrierCoreBridge.swift`
5. 运行并验证启动、停止、状态读取
