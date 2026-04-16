# Barrier Native Migration API Contract

本文档冻结 UI 无关的应用服务契约，供 `gui-gtk`、`gui-macos` 及其桥接层共享。

## 1. 核心入口

- Rust 类型：`app_core::BarrierCore`
- 生命周期：
  - `new()`：初始化内存状态并加载持久化配置
  - `initialize_platform_features()`：启动平台特性（Linux 全局输入转发）
  - `restore_on_startup()`：根据配置恢复上次运行模式

## 2. 命令契约

### 服务控制

- `start_server(config: ServerConfig) -> Result<(), String>`
  - 失败场景：端口占用、权限不足、监听失败
- `start_client(config: ClientConfig) -> Result<(), String>`
  - 失败场景：目标服务不可达、握手失败
- `stop_service() -> Result<(), String>`

### 状态读取

- `get_status() -> Result<AppState, String>`
- `get_runtime_capabilities() -> RuntimeCapabilities`

### 屏幕切换与事件转发

- `switch_client(request: SwitchClientRequest) -> Result<String, String>`
- `switch_back_local(edge: String) -> Result<String, String>`
- `relay_mouse_move(dx: i16, dy: i16) -> Result<(), String>`
- `relay_key_event(key_code: u16, pressed: bool) -> Result<(), String>`
- `relay_mouse_button(button: u8, pressed: bool) -> Result<(), String>`

### 配置管理

- `get_settings() -> AppSettings`
- `save_settings(settings: AppSettings) -> Result<(), String>`
- `reload_settings() -> Result<AppSettings, String>`

## 3. 数据模型

### `AppState`

- `mode: String`：`server` / `client`
- `is_running: bool`
- `connected_clients: u32`
- `server_address: String`
- `active_client: Option<String>`
- `log_messages: Vec<String>`

### `RuntimeCapabilities`

- `session_type: String`
- `has_x11_display: bool`
- `has_wayland_display: bool`
- `input_backend: String`：`x11` / `wayland-limited` / `none`
- `global_input_available: bool`
- `recommendations: Vec<String>`
- `blocking_issues: Vec<String>`

### `SwitchClientRequest`

- `edge: String`
- `client_name: String`
- `client_address: Option<String>`
- `cursor_x: i16`
- `cursor_y: i16`

### `AppSettings`

- `startup_restore: bool`
- `last_mode: String`
- `server_port: u16`
- `server_screen_name: String`
- `server_address: String`
- `client_name: String`
- `pointer_lock_enabled: bool`
- `client_routes: Vec<ClientRoute>`

## 4. 状态机约束

### 模式转换

- `idle -> server_running`
- `idle -> client_running`
- `server_running -> idle`
- `client_running -> idle`
- `server_running -> client_running`（隐式先停服务端）
- `client_running -> server_running`（隐式先停客户端）

### 切换约束

- `switch_client` 仅 `server_running` 可调用
- `relay_*` 仅在 `server_running + active_client != None` 可调用
- `switch_back_local` 仅在 `active_client != None` 可调用

## 5. 错误语义

- 网络错误：保持原始上下文，如 `无法连接服务端（host:port）: ...`
- 状态错误：清晰返回前置条件，如 `服务端未运行`
- 输入错误：如 `客户端名称不能为空`
- 端口冲突：固定文案指引用户改端口或释放占用

所有错误均可直接用于 GUI 提示，不要求前端二次解析错误码。

## 6. 线程与并发模型

- `BarrierCore` 内部通过 `Arc<Mutex<BarrierRuntimeState>>` 序列化状态修改。
- Server/Client 长任务运行在 Tokio 后台任务。
- Linux 全局输入使用独立线程 + 独立 Tokio runtime，避免阻塞 UI 线程。

## 7. 事件订阅

- `subscribe_events() -> broadcast::Receiver<CoreEvent>`
- 事件类型：
  - `status(AppState)`
  - `log(String)`
  - `error(String)`
  - `settings(AppSettings)`

用于原生前端状态同步；也可以直接调用 `get_status` 做主动轮询。
