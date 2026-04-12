# Phase 2 输入捕获层 - 集成测试与优化指南

## 📦 项目结构

```
input-capture/
├── Cargo.toml              # 项目配置和依赖
├── src/
│   ├── lib.rs              # 主库入口和公共 API
│   └── platform/
│       ├── mod.rs          # 平台抽象层定义
│       ├── linux.rs        # Linux (X11) 实现
│       ├── windows.rs      # Windows (Win32) 实现
│       └── macos.rs        # macOS (Quartz) 实现
└── tests/
    └── integration_test.rs # 集成测试
```

## 🔧 构建要求

### Linux
```bash
# 安装 X11 开发库
sudo apt-get install libx11-dev libxi-dev libxtest-dev libxtst-dev

# 构建
cd input-capture
cargo build --release
```

### Windows
```powershell
# 需要 Visual Studio Build Tools 或 Visual Studio
# Windows SDK 已包含所需头文件

cargo build --release
```

### macOS
```bash
# 需要 Xcode Command Line Tools
xcode-select --install

cargo build --release
```

## 🧪 测试指南

### 单元测试（Mock 模式）
```bash
# 使用 mock 实现进行测试（不需要实际硬件权限）
cargo test --features test-mock -- --nocapture
```

### 集成测试（需要权限）

#### Linux
```bash
# 需要 X11 访问权限
cargo test -- --ignored --test-threads=1

# 注意：某些测试可能需要 root 权限或特殊 X11 配置
```

#### Windows
```powershell
# 需要管理员权限以进行全局钩子
Start-Process powershell -Verb RunAs -ArgumentList "cargo test -- --ignored"
```

#### macOS
```bash
# 需要在系统偏好设置中授予辅助功能权限
# 系统偏好设置 -> 安全性与隐私 -> 隐私 -> 辅助功能

cargo test -- --ignored
```

## 📊 测试结果验证

### 预期通过的测试

1. **test_create_input_capture** - 验证 InputCapture 实例创建
2. **test_initialize** - 验证平台初始化
3. **test_keyboard_injection** - 验证键盘事件注入
4. **test_mouse_injection** - 验证鼠标事件注入
5. **test_clipboard** - 验证剪贴板读写

### 测试注意事项

- 标记为 `#[ignore]` 的测试需要实际硬件权限
- 在 CI/CD 环境中建议使用 `test-mock` feature
- 实际设备测试应在隔离环境中进行

## 🚀 性能优化建议

### 1. 减少延迟

```rust
// 优化前：每次调用都获取平台实例
fn process_event() {
    let platform = get_platform().unwrap();
    platform.inject_keyboard(65, true);
}

// 优化后：缓存平台实例
struct InputHandler {
    platform: Box<dyn PlatformInput>,
}

impl InputHandler {
    fn new() -> Self {
        Self {
            platform: get_platform().unwrap(),
        }
    }
    
    fn process_event(&self) {
        self.platform.inject_keyboard(65, true).unwrap();
    }
}
```

### 2. 批量处理事件

```rust
// 对于高频事件（如鼠标移动），考虑批量处理
pub fn inject_mouse_batch(&self, events: &[(i16, i16)]) -> Result<(), PlatformError> {
    // 合并连续的小幅移动
    let mut total_dx = 0;
    let mut total_dy = 0;
    
    for &(dx, dy) in events {
        total_dx += dx;
        total_dy += dy;
    }
    
    if total_dx != 0 || total_dy != 0 {
        self.platform.inject_mouse_move(total_dx as i16, total_dy as i16)?;
    }
    
    Ok(())
}
```

### 3. 异步支持

```rust
// 添加 tokio 支持
use tokio::sync::mpsc;

pub async fn capture_events_async() -> mpsc::Receiver<InputEvent> {
    let (tx, rx) = mpsc::channel(100);
    
    tokio::spawn(async move {
        // 异步捕获逻辑
    });
    
    rx
}
```

## 🔗 与 Phase 1 协议层集成

```rust
use barrier_protocol::{KeyboardEvent, MouseEvent};
use barrier_input_capture::InputCapture;

pub struct BarrierServer {
    input_capture: InputCapture,
}

impl BarrierServer {
    pub fn handle_keyboard_event(&self, event: KeyboardEvent) {
        self.input_capture
            .inject_keyboard(event.key_code, event.pressed)
            .expect("Failed to inject keyboard event");
    }
    
    pub fn handle_mouse_event(&self, event: MouseEvent) {
        match event {
            MouseEvent::Move(dx, dy) => {
                self.input_capture.inject_mouse_move(dx, dy).unwrap();
            }
            MouseEvent::Button(button, pressed) => {
                self.input_capture.inject_mouse_button(button, pressed).unwrap();
            }
        }
    }
}
```

## 🔗 与 Phase 3 Tauri GUI 集成

在 Tauri 命令中调用输入捕获：

```rust
// tauri-gui/src-tauri/src/main.rs
use barrier_input_capture::InputCapture;

#[tauri::command]
fn inject_key(key_code: u16, pressed: bool) -> Result<(), String> {
    let capture = InputCapture::new().map_err(|e| e.to_string())?;
    capture.inject_keyboard(key_code, pressed)
           .map_err(|e| e.to_string())
}

#[tauri::command]
fn inject_mouse_move(dx: i16, dy: i16) -> Result<(), String> {
    let capture = InputCapture::new().map_err(|e| e.to_string())?;
    capture.inject_mouse_move(dx, dy)
           .map_err(|e| e.to_string())
}
```

## ⚠️ 安全注意事项

1. **权限管理**
   - Linux: 可能需要 `setuid` 或 X11 权限
   - Windows: 需要管理员权限进行全局钩子
   - macOS: 需要辅助功能权限

2. **防误触**
   ```rust
   // 实现热键屏蔽功能
   pub fn is_hotkey_shield_enabled() -> bool {
       // 检查是否启用了热键保护
       std::env::var("BARRIER_HOTKEY_SHIELD")
           .map(|v| v == "1")
           .unwrap_or(true)
   }
   ```

3. **日志记录**
   ```rust
   use log::{info, warn, error};
   
   fn inject_with_logging(key_code: u16, pressed: bool) {
       info!("Injecting key {} {}", key_code, if pressed { "press" } else { "release" });
       // ... 注入逻辑
   }
   ```

## 📈 基准测试

创建性能基准测试：

```rust
// benches/injection_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_keyboard_injection(c: &mut Criterion) {
    let capture = InputCapture::new().unwrap();
    
    c.bench_function("keyboard_injection", |b| {
        b.iter(|| {
            capture.inject_keyboard(black_box(65), black_box(true)).unwrap();
            capture.inject_keyboard(black_box(65), black_box(false)).unwrap();
        })
    });
}

criterion_group!(benches, benchmark_keyboard_injection);
criterion_main!(benches);
```

运行基准测试：
```bash
cargo bench
```

## 🐛 常见问题排查

### Linux 问题
- **X11 连接失败**: 检查 `DISPLAY` 环境变量
- **权限拒绝**: 尝试 `xhost +SI:localuser:$USER`

### Windows 问题
- **钩子失败**: 确保以管理员身份运行
- **UAC 阻止**: 调整 UAC 设置或使用 manifest

### macOS 问题
- **辅助功能权限**: 在系统偏好设置中重新授权
- **沙盒限制**: 禁用 App Sandbox 或使用 entitlements

## 📝 下一步行动

1. ✅ 完成平台特定实现
2. ✅ 添加集成测试
3. ⏳ 性能基准测试
4. ⏳ 与 Phase 1 和 Phase 3 完全集成
5. ⏳ 端到端测试

## 📚 参考资源

- [X11 Extension Library](https://www.x.org/releases/current/doc/libX11/libX11.html)
- [Windows Hooks](https://docs.microsoft.com/en-us/windows/win32/winmsg/about-hooks)
- [macOS Quartz Event Services](https://developer.apple.com/documentation/coregraphics/quartz_event_services)
