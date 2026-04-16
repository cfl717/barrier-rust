# Barrier Rust 重写实施路线图

## 项目概述

将 Barrier（跨平台键鼠共享工具）从 C++ 重写为 Rust + 原生双前端架构，实现：
- **内存安全**：消除内存泄漏和数据竞争
- **现代化 GUI**：基于平台原生技术的桌面界面
- **低延迟**：保持原有性能水平
- **易维护**：清晰的代码结构和完善的文档

---

## Phase 1: 协议层脚手架 ✅ (已完成)

### 完成内容

#### 1. 核心协议模块 (`src/protocol/`)
- **message.rs** (250 行)
  - MessageType 类型定义和转换
  - Message 结构体（序列化/反序列化）
  - 所有 Barrier 消息类型常量（CBYQ, QBYC, CLAP, CMOV, etc.）
  - XOR 校验和计算
  - 单元测试覆盖

- **handshake.rs** (210 行)
  - 客户端握手流程 (`client_handshake`)
  - 服务端握手流程 (`server_handshake`)
  - HandshakeState 状态机
  - HandshakeResult 结果结构
  - 异步读写支持

- **events.rs** (363 行)
  - MouseMoveEvent（鼠标移动）
  - MouseButtonEvent（鼠标按键）
  - KeyEvent（键盘事件）
  - ClientEnterEvent（屏幕进入）
  - ModifierKeys（修饰键）
  - 完整的序列化/反序列化
  - 单元测试

- **clipboard.rs** (222 行)
  - ClipboardFormat（文本/HTML/RTF/BMP/PNG）
  - ClipboardData 数据结构
  - 消息创建和解析函数
  - 多格式支持

#### 2. 网络层模块 (`src/network/`)
- **connection.rs** (123 行)
  - Connection 结构体封装 TCP 流
  - send/receive 方法
  - 握手状态管理

- **server.rs** (198 行)
  - BarrierServer 主服务器
  - ServerConfig 配置
  - ClientInfo 客户端信息
  - ServerEvent 事件系统
  - 异步客户端处理
  - 并发连接管理

- **client.rs** (228 行)
  - BarrierClient 客户端
  - ClientConfig 配置
  - ClientState 状态机
  - 自动重连机制
  - run 循环

#### 3. 平台抽象层 (`src/platform/`)
- **mod.rs** (84 行)
  - PlatformInput trait 定义
  - PlatformError 错误类型
  - get_platform() 工厂函数

- **linux.rs** (83 行)
  - LinuxInput 结构体
  - Trait 方法桩实现
  - X11/Wayland 待实现标记

- **windows.rs** (68 行)
  - WindowsInput 结构体
  - Win32 API 待实现标记

- **macos.rs** (68 行)
  - MacOsInput 结构体
  - Cocoa 待实现标记

#### 4. 示例程序 (`examples/`)
- **server.rs** (60 行)
  - 完整服务器运行示例
  - 事件处理演示
  - 日志输出

- **client.rs** (32 行)
  - 完整客户端运行示例
  - 自动重连演示

- **message_demo.rs** (76 行)
  - 消息序列化演示
  - 鼠标事件示例
  - 剪贴板数据示例
  - 往返验证

#### 5. 文档和配置
- **Cargo.toml**: 依赖配置（tokio, serde, log, thiserror 等）
- **README.md**: 完整使用文档、架构图、协议说明
- **lib.rs**: 模块导出和架构说明

### 代码统计
| 类别 | 文件数 | 代码行数 |
|------|--------|----------|
| 协议层 | 5 | 1,059 |
| 网络层 | 4 | 560 |
| 平台层 | 4 | 303 |
| 示例 | 3 | 168 |
| **总计** | **16** | **2,090** |

### 测试覆盖
- 消息序列化测试 ✅
- 握手流程测试 ✅
- 输入事件测试 ✅
- 剪贴板测试 ✅
- 服务器/客户端创建测试 ✅

---

## Phase 2: 输入捕获层 (计划中)

### 目标
实现各平台的底层输入捕获和注入功能

### Linux (X11/Wayland)
- [ ] X11 键盘钩子（XGrabKeyboard）
- [ ] X11 鼠标钩子（XGrabPointer）
- [ ] XTest 事件注入
- [ ] X11 剪贴板（XSelection）
- [ ] Wayland 支持（xdg-desktop-portal）
- [ ] 文件拖放（XDnD 协议集成）

### Windows
- [ ] 全局键盘钩子（SetWindowsHookEx）
- [ ] 全局鼠标钩子
- [ ] SendInput 事件注入
- [ ] 剪贴板 API（OpenClipboard/SetClipboardData）
- [ ] 文件拖放（OLE IDropTarget）

### macOS
- [ ] CGEventTap 事件捕获
- [ ] CGEventPost 事件注入
- [ ] NSPasteboard 剪贴板
- [ ] 辅助功能权限检查

### 预计工作量
- 开发时间：8-10 周
- 代码量：~2,000 行
- 测试用例：50+

---

## Phase 3: 原生 GUI (计划中)

### 前端技术栈
- **Ubuntu**: GTK4 + libadwaita
- **macOS**: SwiftUI + AppKit
- **共享核心**: `app-core`
- **桥接层**: `app-core-ffi`

### 后端集成
- **直接调用**: Ubuntu 前端进程内调用 Rust Core
- **FFI 桥接**: macOS 通过 C ABI 调用共享核心
- **系统托盘**: 后台运行支持

### 功能页面
1. **主界面**
   - 服务器/客户端切换
   - 屏幕布局配置（可视化拖拽）
   - 连接状态显示

2. **设置页面**
   - 网络配置（端口、加密）
   - 热键配置
   - 剪贴板选项
   - 文件传输选项

3. **日志页面**
   - 实时日志流
   - 日志级别过滤
   - 导出功能

### 预计工作量
- 开发时间：6-8 周
- 代码量：~3,000 行（Rust + Swift）

---

## Phase 4: 生产就绪 (计划中)

### 性能优化
- [ ] 基准测试和性能分析
- [ ] 事件处理延迟优化（目标 <5ms）
- [ ] 内存使用优化
- [ ] 网络缓冲区调优

### 安全加固
- [ ] TLS 加密传输
- [ ] 认证机制（密码/证书）
- [ ] 输入验证
- [ ] 审计日志

### 测试完善
- [ ] 单元测试覆盖率 >80%
- [ ] 集成测试
- [ ] 跨平台测试矩阵
- [ ] 压力测试

### 文档
- [ ] API 文档（rustdoc）
- [ ] 用户手册
- [ ] 开发者指南
- [ ] 故障排除指南

### 打包分发
- [ ] Windows MSI/EXE
- [ ] macOS DMG/App Bundle
- [ ] Linux DEB/RPM/Flatpak
- [ ] 自动更新机制

### 预计工作量
- 开发时间：6-8 周

---

## 总体时间线

| 阶段 | 内容 | 工期 | 累计 |
|------|------|------|------|
| Phase 1 | 协议层脚手架 | 2 周 | 2 周 |
| Phase 2 | 输入捕获层 | 10 周 | 12 周 |
| Phase 3 | 原生 GUI | 8 周 | 20 周 |
| Phase 4 | 生产就绪 | 8 周 | 28 周 |

**总工期**: 约 7 个月（28 周）

---

## 团队配置建议

- **Rust 开发者**: 2-3 名（系统编程经验）
- **桌面前端开发者**: 1-2 名（GTK / SwiftUI）
- **测试工程师**: 1 名（兼职）
- **技术负责人**: 1 名（架构设计 + 代码审查）

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Wayland 兼容性 | 高 | 优先支持 X11，Wayland 作为实验性特性 |
| macOS 权限问题 | 中 | 提前研究辅助功能 API，准备用户引导 |
| 性能不达标 | 中 | 早期基准测试，保留关键路径 C 优化 |
| 学习曲线 | 低 | Rust 培训，代码审查，结对编程 |

---

## 成功标准

1. **功能完整性**: 100% 兼容原有 Barrier 功能
2. **性能**: 延迟 ≤ 原 C++ 版本
3. **稳定性**: 无内存泄漏，崩溃率 <0.1%
4. **用户体验**: GUI 响应流畅，配置简单
5. **代码质量**: 测试覆盖率 >80%，文档完善

---

## 下一步行动

1. ✅ Phase 1 完成（当前）
2. 📋 启动 Phase 2：Linux X11 输入捕获
3. 📋 添加更多协议消息类型支持（屏幕形状、心跳等）
4. 📋 建立 CI/CD 流水线
5. 📋 编写 Phase 2 详细设计文档

---

*文档生成时间：2024*
*版本：1.0*
