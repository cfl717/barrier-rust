# Barrier Rust 项目 README

[![License: GPL-2.0](https://img.shields.io/badge/License-GPL--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-1.5+-blue.svg)](https://tauri.app)
[![Status](https://img.shields.io/badge/Status-Alpha-yellow.svg)]()

## 🖥️ Barrier - 跨平台键鼠共享工具 (Rust 重写版)

使用 Rust + Tauri 重写的 Barrier，实现跨多台计算机共享鼠标和键盘。

### ✨ 特性

- 🔒 **内存安全**: 利用 Rust 所有权系统消除内存泄漏
- ⚡ **低延迟**: 异步架构保持毫秒级响应
- 🎨 **现代化 GUI**: React + Tauri 构建的精美界面
- 📋 **剪贴板共享**: 支持文本、HTML、图片多格式
- 🖱️ **文件拖放**: 跨设备拖放文件传输
- 🐧 **跨平台**: Linux, Windows, macOS 完整支持

---

## 📁 项目结构

```
/workspace/
├── README.md                          # 本文件
├── docs/PROJECT_STRUCTURE.md               # 详细项目结构文档
├── docs/GUI_ALTERNATIVES_EVALUATION.md     # GUI 方案评估报告
├── docs/BARRIER_RUST_REWRITE_COMPLETE.md   # 完整重构实施报告
├── docs/PHASE2_INTEGRATION_TESTING.md      # Phase 2 测试与优化指南
│
├── protocol/          # Phase 1: 协议层 ✅
│   ├── Cargo.toml
│   ├── src/
│   │   ├── protocol/                  # Barrier 协议实现
│   │   ├── network/                   # TCP 服务器/客户端
│   │   └── platform/                  # 平台抽象层
│   └── examples/                      # 可运行示例
│
├── input-capture/              # Phase 2: 输入捕获层 ✅
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                     # 公共 API
│   │   └── platform/                  # 平台特定实现
│   │       ├── linux.rs               # Linux (X11)
│   │       ├── windows.rs             # Windows (Win32)
│   │       └── macos.rs               # macOS (Quartz)
│   └── tests/                         # 集成测试
│
└── tauri-gui/                  # Phase 3: Tauri GUI ✅
    ├── src/                           # React 前端
    ├── src-tauri/                     # Rust 后端
    └── package.json
```

---

## 🚀 快速开始

### 前置要求

- **Rust** 1.70+
- **Node.js** 18+
- **系统依赖**:
  - Linux: `libwebkit2gtk-4.0-dev`, `libssl-dev`, `libgtk-3-dev`
  - Windows: WebView2, Visual Studio C++ 工具
  - macOS: Xcode 命令行工具

### 安装系统依赖 (Linux)

```bash
sudo apt update
sudo apt install -y \
    libwebkit2gtk-4.0-dev \
    build-essential \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

### 克隆项目

```bash
git clone <repository-url>
cd barrier-rust
```

---

## 📦 运行方式

### 方式 1: 仅协议层 (CLI)

适合开发和测试协议功能：

```bash
cd protocol

# 编译
cargo build --release

# 运行服务器
cargo run --example server

# 运行客户端
cargo run --example client

# 运行消息演示
cargo run --example message_demo

# 运行测试
cargo test
```

### 方式 2: 完整 GUI 应用

```bash
cd tauri-gui

# 安装前端依赖
npm install

# 开发模式 (热重载)
npm run tauri dev

# 生产构建
npm run tauri build
```

构建完成后，安装包位于：
- Linux: `src-tauri/target/release/bundle/deb/*.deb`
- Windows: `src-tauri/target/release/bundle/msi/*.msi`
- macOS: `src-tauri/target/release/bundle/dmg/*.dmg`

---

## 💻 使用指南

### 服务器端配置

1. 选择 **Server** 模式
2. 点击 **Start Server**
3. 记录显示的 IP 地址 (如 `192.168.1.100:24800`)

### 客户端配置

1. 选择 **Client** 模式
2. 输入服务器地址 (如 `192.168.1.100:24800`)
3. 点击 **Start Client**

### 屏幕布局

当前版本支持基础连接，屏幕布局编辑器将在后续版本中添加。

---

## 🛠️ 开发指南

### 代码结构

#### 协议层 (`protocol/src/`)

| 模块 | 描述 | 代码行数 |
|------|------|----------|
| `protocol/message.rs` | 消息序列化/反序列化 | 250 |
| `protocol/handshake.rs` | 握手协议 | 210 |
| `protocol/events.rs` | 输入事件定义 | 363 |
| `protocol/clipboard.rs` | 剪贴板支持 | 222 |
| `network/server.rs` | TCP 服务器 | 198 |
| `network/client.rs` | TCP 客户端 | 228 |
| `platform/linux.rs` | Linux 平台实现 | 83 |
| `platform/windows.rs` | Windows 平台实现 | 68 |
| `platform/macos.rs` | macOS 平台实现 | 68 |

#### GUI 层 (`tauri-gui/`)

| 组件 | 技术 | 描述 |
|------|------|------|
| 前端 | React + TypeScript | 用户界面 |
| 后端 | Rust + Tauri | IPC 命令处理 |
| 样式 | CSS | 自定义主题 |

### 添加新功能

1. **协议扩展**: 在 `protocol/src/protocol/` 中添加新消息类型
2. **平台实现**: 在 `protocol/src/platform/` 中实现平台特定功能
3. **GUI 更新**: 在 `tauri-gui/src/` 中更新 React 组件
4. **IPC 命令**: 在 `tauri-gui/src-tauri/src/main.rs` 中添加新命令

### 代码规范

```bash
# Rust 格式化
cargo fmt

# Rust Lint
cargo clippy

# TypeScript 检查
cd tauri-gui
npx tsc --noEmit
npx eslint src/
```

---

## 📊 性能指标

| 指标 | 目标 | 当前状态 |
|------|------|----------|
| 输入延迟 | <5ms | 待测试 |
| 内存占用 | <50MB | 待测试 |
| 启动时间 | <2s | 待测试 |
| 二进制大小 | <10MB | ~8MB |

---

## 🔒 安全性

### 已实现
- ✅ 内存安全 (Rust 所有权系统)
- ✅ 线程安全 (Tokio Mutex)
- ✅ 输入验证 (IPC 参数检查)

### 计划中
- ⏳ TLS 加密通信
- ⏳ 身份认证机制
- ⏳ 访问控制列表

---

## 🗺️ 路线图

### Phase 1: 协议层脚手架 ✅
- [x] 消息协议实现
- [x] 网络通信层
- [x] 平台抽象层
- [x] 单元测试

### Phase 2: 输入捕获层 ✅
- [x] Linux X11 输入捕获 (467 行)
- [x] Windows 全局钩子 (336 行)
- [x] macOS CGEventTap (308 行)
- [x] 剪贴板同步
- [x] 平台抽象层
- [x] 集成测试框架
- [x] 文档和示例

### Phase 3: Tauri GUI ✅
- [x] 基础界面
- [x] IPC 通信
- [ ] 系统托盘
- [ ] 屏幕布局编辑器
- [ ] 配置文件管理

### Phase 4: 生产就绪 📋
- [ ] TLS 加密
- [ ] 性能优化
- [ ] 安装包构建
- [ ] 自动更新
- [ ] 完整文档

---

## 🤝 贡献指南

### 开发流程

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

### 提交信息规范

遵循 [约定式提交](https://www.conventionalcommits.org/):

- `feat:` 新功能
- `fix:` 修复 bug
- `docs:` 文档更新
- `style:` 代码格式
- `refactor:` 重构
- `test:` 测试相关
- `chore:` 构建/工具

---

## 📄 许可证

GPL-2.0 License - 与原 Barrier 项目保持一致

---

## 🙏 致谢

- 原 [Barrier](https://github.com/debauchee/barrier) 团队的基础工作
- [Tauri](https://tauri.app) 团队的优秀框架
- [React](https://react.dev) 和 [Vite](https://vitejs.dev) 团队的工具链
- Rust 社区的生态支持

---

## 📞 联系方式

- **Issues**: [GitHub Issues](link-to-issues)
- **Discussions**: [GitHub Discussions](link-to-discussions)

---

**版本**: 3.0.0-alpha.1  
**状态**: 开发中  
**最后更新**: 2024
