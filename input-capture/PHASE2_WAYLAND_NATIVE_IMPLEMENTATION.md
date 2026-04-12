# Phase 2: Wayland 原生支持实施计划

## 概述

Phase 2 将实现完整的原生 Wayland 输入捕获和注入功能，移除对外部工具和 X11 兼容层的依赖。

## 目标

1. **原生键盘捕获** - 使用 `wlr-input-inhibitor-unstable-v1` 协议
2. **原生鼠标捕获** - 使用相同的 inhibitor 协议
3. **原生虚拟输入** - 使用 `virtual-keyboard-unstable-v1` 和 `wlr-virtual-pointer-unstable-v1` 协议
4. **原生剪贴板** - 使用 Wayland 数据设备协议 (wlr-data-control)
5. **完整事件循环** - 集成 Wayland 事件处理

## 架构设计

### 核心组件

```
LinuxInput
├── DisplayServer (运行时检测)
├── X11Backend (可选，feature-gated)
└── WaylandBackend (新增)
    ├── WaylandConnection
    ├── RegistryHandler
    ├── InputInhibitorManager
    ├── VirtualKeyboardManager
    ├── VirtualPointerManager
    └── DataControlManager
```

### Wayland 协议依赖

| 功能 | 协议 | 状态 |
|------|------|------|
| 键盘/鼠标捕获 | wlr-input-inhibitor-unstable-v1 | 需要实现 |
| 虚拟键盘注入 | virtual-keyboard-unstable-v1 | 需要实现 |
| 虚拟指针注入 | wlr-virtual-pointer-unstable-v1 | 需要实现 |
| 剪贴板访问 | wlr-data-control-unstable-v1 | 需要实现 |

## 实施步骤

### 步骤 1: Wayland 连接和注册表处理

```rust
// 新增结构体
pub struct WaylandBackend {
    connection: Connection,
    registry: RegistryGlobalHandler,
    compositor: Option<wl_compositor::WlCompositor>,
    seat: Option<wl_seat::WlSeat>,
    input_inhibitor_manager: Option<zwlr_input_inhibitor_manager_v1::ZwlrInputInhibitorManagerV1>,
    virtual_keyboard_manager: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,
    virtual_pointer_manager: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
    data_control_manager: Option<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1>,
}
```

**关键任务:**
- 建立 Wayland 连接
- 实现 RegistryGlobalHandler trait
- 绑定所需的全局对象
- 检查协议支持情况

### 步骤 2: 输入捕获实现

```rust
impl WaylandBackend {
    pub fn capture_keyboard(&mut self) -> Result<(), PlatformError> {
        // 使用 wlr-input-inhibitor-unstable-v1
        let inhibitor = self.input_inhibitor_manager
            .as_ref()
            .ok_or(PlatformError::NotSupported)?
            .get_inhibitor();
        
        // 存储 inhibitor 以防止被释放
        self.active_inhibitor = Some(inhibitor);
        Ok(())
    }
    
    pub fn release_keyboard_capture(&mut self) {
        // 销毁 inhibitor 以释放捕获
        self.active_inhibitor.take();
    }
}
```

**注意事项:**
- 需要 compositor 支持 wlr-input-inhibitor 协议
- 某些 compositor 可能限制后台应用的捕获
- 需要用户授权

### 步骤 3: 虚拟输入设备实现

```rust
impl WaylandBackend {
    pub fn create_virtual_keyboard(&mut self) -> Result<VirtualKeyboard, PlatformError> {
        let keyboard_manager = self.virtual_keyboard_manager
            .as_ref()
            .ok_or(PlatformError::NotSupported)?;
        
        let keyboard = keyboard_manager.create_virtual_keyboard(&self.seat);
        Ok(VirtualKeyboard { inner: keyboard })
    }
    
    pub fn inject_key(&mut self, key_code: u32, state: KeyState) -> Result<(), PlatformError> {
        if let Some(kbd) = &self.virtual_keyboard {
            kbd.key(0, key_code, state);
            kbd.commit();
            Ok(())
        } else {
            Err(PlatformError::NotInitialized)
        }
    }
}
```

### 步骤 4: 剪贴板实现

```rust
impl WaylandBackend {
    pub fn get_clipboard(&self) -> Result<String, PlatformError> {
        use std::os::unix::net::UnixStream;
        
        let data_device = self.data_control_manager
            .as_ref()?
            .get_data_device(&self.seat);
        
        let data_source = data_device.data_offer();
        // 实现 MIME type 协商和数据读取
        // ...
    }
    
    pub fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
        // 创建 data source
        // 提供 text/plain MIME type
        // 监听 selection 事件
        // ...
    }
}
```

### 步骤 5: 事件循环集成

```rust
use std::os::unix::io::AsRawFd;
use nix::poll::{poll, PollFd, POLLIN};

impl WaylandBackend {
    pub fn dispatch_events(&self) -> Result<(), PlatformError> {
        let fd = self.connection.as_raw_fd();
        let mut poll_fds = [PollFd::new(fd, POLLIN)];
        
        poll(&mut poll_fds, 100)?; // 100ms timeout
        
        if poll_fds[0].revents().unwrap().contains(POLLIN) {
            self.connection.dispatch_pending()?;
        }
        
        Ok(())
    }
}
```

## 代码结构变更

### 修改 `src/platform/linux.rs`

添加新的 WaylandBackend 模块:

```rust
#[cfg(feature = "wayland")]
mod wayland_backend {
    use wayland_client::*;
    use wayland_protocols_wlr::input_inhibitor::v1::client::*;
    use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::*;
    // ...
    
    pub struct WaylandBackend {
        // 字段定义
    }
    
    impl WaylandBackend {
        pub fn new() -> Result<Self, PlatformError> { }
        pub fn capture_keyboard(&self) -> Result<(), PlatformError> { }
        pub fn inject_keyboard(&self, key: u32, pressed: bool) -> Result<(), PlatformError> { }
        // ... 其他方法
    }
}
```

### 更新 `LinuxInput` 结构

```rust
pub struct LinuxInput {
    display_server: DisplayServer,
    
    #[cfg(feature = "x11")]
    x11_backend: Option<X11Backend>,
    
    #[cfg(feature = "wayland")]
    wayland_backend: Option<WaylandBackend>,
    
    initialized: bool,
    // ...
}
```

### 更新 trait 实现

```rust
impl PlatformInput for LinuxInput {
    fn initialize(&self) -> Result<(), PlatformError> {
        match self.display_server {
            DisplayServer::Wayland => {
                #[cfg(feature = "wayland")]
                {
                    self.wayland_backend = Some(WaylandBackend::new()?);
                }
            }
            DisplayServer::X11 => {
                #[cfg(feature = "x11")]
                {
                    self.x11_backend = Some(X11Backend::new()?);
                }
            }
        }
        Ok(())
    }
    
    fn capture_keyboard(&self) -> Result<(), PlatformError> {
        #[cfg(feature = "wayland")]
        if let Some(backend) = &self.wayland_backend {
            return backend.capture_keyboard();
        }
        
        #[cfg(feature = "x11")]
        if let Some(backend) = &self.x11_backend {
            return backend.capture_keyboard();
        }
        
        Err(PlatformError::NotSupported)
    }
    
    // ... 其他方法类似
}
```

## Compositor 兼容性

### 支持的 Compositors

| Compositor | Input Inhibitor | Virtual Keyboard | Virtual Pointer | Data Control |
|------------|-----------------|------------------|-----------------|--------------|
| sway ✅ | ✓ | ✓ | ✓ | ✓ |
| KDE Plasma ✅ | ✓ | ✓ | ✓ | ✓ |
| GNOME ❌ | ✗ | ✗ | ✗ | ✗ |
| wlroots-based ✅ | ✓ | ✓ | ✓ | ✓ |
| River ✅ | ✓ | ✓ | ✓ | ✓ |

**注意:** GNOME 默认不支持这些协议，需要特殊处理或使用 X11 fallback。

## 构建配置

### Cargo.toml 更新

```toml
[features]
default = ["x11"]
x11 = ["dep:x11rb", "dep:nix"]
wayland = [
    "dep:wayland-client",
    "dep:wayland-protocols", 
    "dep:wayland-protocols-wlr",
    "dep:wayland-protocols-misc"  # 新增
]
wayland-native = ["wayland"]  # 纯 Wayland 模式，无 X11
```

### 新增依赖

```toml
wayland-protocols-misc = { version = "0.3", features = ["client"], optional = true }
```

## 测试计划

### 单元测试

```rust
#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "wayland")]
    fn test_wayland_connection() {
        // 测试 Wayland 连接建立
    }
    
    #[test]
    #[cfg(feature = "wayland")]
    fn test_protocol_availability() {
        // 测试所需协议是否可用
    }
}
```

### 集成测试

1. **Sway WM 测试**: 在 Sway 中测试完整功能
2. **KDE 测试**: 在 KDE Plasma 下测试
3. **Fallback 测试**: 在不支持协议的 compositor 上测试 X11 fallback

## 错误处理

```rust
#[derive(Debug, thiserror::Error)]
pub enum WaylandError {
    #[error("Wayland connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Protocol not supported: {0}")]
    ProtocolNotSupported(String),
    
    #[error("Permission denied by compositor")]
    PermissionDenied,
    
    #[error("Compositor does not support required protocols")]
    CompositorIncompatible,
}
```

## 安全考虑

1. **权限控制**: 输入捕获需要用户明确授权
2. **沙盒环境**: 在 Flatpak/Snap 中需要额外权限
3. **隐私保护**: 确保捕获的输入数据不被泄露

## 性能优化

1. **批处理事件**: 减少 Wayland 协议调用次数
2. **异步处理**: 使用 async/await 处理事件循环
3. **连接复用**: 保持单一 Wayland 连接

## 时间估算

| 任务 | 预计时间 |
|------|----------|
| Wayland 连接和注册表 | 4 小时 |
| 输入捕获实现 | 6 小时 |
| 虚拟输入设备 | 8 小时 |
| 剪贴板实现 | 6 小时 |
| 事件循环集成 | 4 小时 |
| 测试和调试 | 8 小时 |
| 文档和清理 | 4 小时 |
| **总计** | **40 小时** |

## 后续工作 (Phase 3)

1. 性能基准测试和优化
2. 更多 compositor 兼容性测试
3. 添加配置选项
4. CI/CD 集成测试
5. 发布文档和用户指南

## 参考资源

- [wlr-input-inhibitor-unstable-v1](https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-input-inhibitor-unstable-v1.xml)
- [virtual-keyboard-unstable-v1](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/unstable/virtual-keyboard/virtual-keyboard-unstable-v1.xml)
- [wlr-virtual-pointer-unstable-v1](https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-virtual-pointer-unstable-v1.xml)
- [wlr-data-control-unstable-v1](https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-data-control-unstable-v1.xml)
- [wayland-rs 示例](https://github.com/Smithay/wayland-rs)
