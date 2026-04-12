# Phase 1: Wayland Support - 基础架构重构

## 概述

本文档描述了为项目添加 Linux Wayland 支持的第一阶段实现。Phase 1 主要完成基础架构重构，为后续的原生 Wayland 实现奠定基础。

## 已完成的工作

### 1. Cargo.toml 配置更新

**新增 Features:**
- `x11` (默认启用): X11 支持
- `wayland`: Wayland 支持（当前为预留，Phase 2 实现原生功能）

**新增依赖:**
```toml
wayland-client = { version = "0.31", optional = true }
wayland-protocols = { version = "0.31", features = ["client", "staging", "unstable"], optional = true }
wayland-protocols-wlr = { version = "0.2", features = ["client"], optional = true }
```

### 2. DisplayServer 运行时检测

新增 `DisplayServer` 枚举和检测逻辑：

```rust
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

impl DisplayServer {
    pub fn detect() -> Self {
        // 优先检测 Wayland
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return DisplayServer::Wayland;
        }
        
        // 回退到 X11
        if std::env::var("DISPLAY").is_ok() {
            return DisplayServer::X11;
        }
        
        DisplayServer::Unknown
    }
}
```

**检测优先级:**
1. `WAYLAND_DISPLAY` 环境变量 → Wayland
2. `DISPLAY` 环境变量 → X11
3. 均不存在 → Unknown

### 3. LinuxInput 结构体重构

**主要变更:**
- 添加 `display_server: DisplayServer` 字段用于运行时检测
- 保留 `#[cfg(feature)]` 条件编译的 X11/Wayland 连接字段
- 添加 `is_wayland()` 和 `is_x11()` 辅助方法

### 4. 输入操作方法重构

所有 PlatformInput trait 方法现在采用以下模式：

```rust
fn capture_keyboard(&self) -> Result<(), PlatformError> {
    #[cfg(feature = "wayland")]
    if self.is_wayland() {
        log::warn!("Wayland keyboard capture not yet implemented (Phase 1)");
        return self.capture_keyboard_x11_fallback();
    }
    
    #[cfg(feature = "x11")]
    if self.is_x11() {
        return self.capture_keyboard_x11();
    }
    
    Err(PlatformError::NotSupported)
}
```

**重构的方法:**
- `capture_keyboard()`
- `inject_keyboard()`
- `capture_mouse()`
- `inject_mouse_move()`
- `inject_mouse_button()`
- `get_clipboard()`
- `set_clipboard()`

### 5. X11 实现提取

将原有内联实现提取为独立方法：
- `capture_keyboard_x11()`
- `inject_keyboard_x11()`
- `capture_mouse_x11()`
- `inject_mouse_move_x11()`
- `inject_mouse_button_x11()`

**技术改进:**
- 从旧的 `x11` crate 迁移到现代的 `x11rb` crate
- 使用更安全的 Rust API 替代 unsafe 代码
- 改进错误处理和日志记录

### 6. Wayland 回退机制

Phase 1 中，Wayland 环境下使用以下策略：

**有 X11 特性时:**
- Wayland 会话自动回退到 X11 实现
- 通过 X11 兼容层提供基本功能

**无 X11 特性时:**
- 剪贴板：继续使用 `wl-paste`/`wl-copy` 命令行工具
- 输入捕获/注入：返回 `NotSupported` 错误

### 7. 剪贴板优化

**新逻辑:**
```rust
fn get_clipboard(&self) -> Result<String, PlatformError> {
    // 1. Wayland 环境优先尝试 wl-paste
    #[cfg(feature = "wayland")]
    if self.is_wayland() {
        // 尝试 wl-paste
    }
    
    // 2. X11 环境尝试 xclip
    #[cfg(feature = "x11")]
    {
        // 尝试 xclip
    }
    
    // 3. 全部失败返回错误
}
```

## 架构优势

### 1. 运行时自适应
- 自动检测显示服务器类型
- 无需手动配置
- 支持混合环境（X11 + Wayland）

### 2. 渐进式开发
- Phase 1: 基础架构 + X11 回退
- Phase 2: 原生 Wayland 实现
- 向后兼容现有功能

### 3. 特性门控
- 用户可根据需求选择特性
- 减少不必要的依赖
- 灵活的构建配置

## 使用方法

### 默认构建（仅 X11）
```bash
cargo build
```

### 启用 Wayland 支持
```bash
cargo build --features wayland
```

### 同时启用 X11 和 Wayland
```bash
cargo build --features "x11,wayland"
```

### 仅 Wayland（无 X11 回退）
```bash
cargo build --no-default-features --features wayland
```

## Phase 1 限制

### 已知问题

1. **Wayland 非原生实现**
   - 输入捕获/注入依赖 X11 回退
   - 纯 Wayland 环境功能受限

2. **剪贴板依赖外部工具**
   - 需要安装 `xclip` 或 `wl-clipboard`
   - 非原生协议实现

3. **权限要求**
   - X11 回退仍需适当权限
   - 某些 Wayland compositor 可能限制 X11 兼容层

### 待改进项（Phase 2）

- [ ] 原生 Wayland 输入捕获（`wlr-input-inhibitor-unstable-v1`）
- [ ] 原生 Wayland 虚拟指针（`wlr-virtual-pointer-unstable-v1`）
- [ ] 原生 Wayland 虚拟键盘（`virtual-keyboard-unstable-v1`）
- [ ] 移除对外部剪贴板工具的依赖
- [ ] 完整的 Wayland 事件循环集成

## 测试建议

### 1. X11 环境测试
```bash
# 确保在 X11 会话中
echo $DISPLAY  # 应显示 :0 或类似
cargo test --features x11
```

### 2. Wayland 环境测试（带 X11 回退）
```bash
# 在 Wayland 会话中
echo $WAYLAND_DISPLAY  # 应显示 wayland-0 或类似
cargo test --features "x11,wayland"
```

### 3. 纯 Wayland 测试
```bash
cargo test --no-default-features --features wayland
```

## 下一步计划（Phase 2）

Phase 2 将实现完整的原生 Wayland 支持：

1. **Wayland 连接管理**
   - 实现 `wl_display` 连接
   - 注册全局对象
   - 事件循环集成

2. **输入捕获协议**
   - `wlr-input-inhibitor-unstable-v1`
   - 全局键盘/鼠标钩子

3. **输入注入协议**
   - `wlr-virtual-pointer-unstable-v1`
   - `virtual-keyboard-unstable-v1`

4. **测试与优化**
   - 多 compositor 测试（GNOME, KDE, Sway）
   - 性能优化
   - 错误处理完善

## 兼容性说明

### 支持的显示服务器

| 显示服务器 | Phase 1 支持 | Phase 2 目标 |
|-----------|-------------|-------------|
| X11       | ✅ 完整支持  | ✅ 保持支持  |
| Wayland + X11 回退 | ⚠️ 通过 X11 兼容层 | ✅ 原生支持 |
| 纯 Wayland | ❌ 部分功能受限 | ✅ 完整支持 |

### Compositor 兼容性

Phase 2 将测试以下 compositor：
- GNOME (Mutter)
- KDE Plasma (KWin)
- Sway
- Weston
- Hyprland

## 故障排除

### 常见问题

**Q: Wayland 下无法捕获输入**
A: Phase 1 中这是预期行为。如果未启用 X11 特性，Wayland 输入捕获将返回 `NotSupported`。请在 Phase 2 完成后重试。

**Q: 剪贴板操作失败**
A: 确保已安装相应工具：
- X11: `xclip` 或 `xsel`
- Wayland: `wl-clipboard`

**Q: 检测到错误的显示服务器**
A: 检查环境变量设置。某些情况下可能需要手动设置 `WAYLAND_DISPLAY` 或 `DISPLAY`。

## 总结

Phase 1 成功完成了 Wayland 支持的基础架构重构：

✅ 运行时显示服务器检测  
✅ 模块化代码结构  
✅ X11 实现现代化（迁移到 x11rb）  
✅ Wayland 回退机制  
✅ 剪贴板优化  
✅ 灵活的特性门控  

这为 Phase 2 的原生 Wayland 实现奠定了坚实基础。
