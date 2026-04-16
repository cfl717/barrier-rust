# app-core

`app-core` 是 Barrier 的共享应用核心，负责统一管理：

- Server / Client 生命周期
- 运行状态与日志
- 屏幕切换与输入转发
- 运行环境能力检测（X11/Wayland）
- 配置持久化与启动恢复

## 主要 API

- `BarrierCore::start_server`
- `BarrierCore::start_client`
- `BarrierCore::stop_service`
- `BarrierCore::get_status`
- `BarrierCore::switch_client`
- `BarrierCore::switch_back_local`
- `BarrierCore::save_settings` / `reload_settings`
- `BarrierCore::restore_on_startup`

## 事件流

可通过 `subscribe_events()` 订阅 `CoreEvent`，用于原生 UI 无轮询刷新。
