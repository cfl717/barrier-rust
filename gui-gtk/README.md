# Barrier GTK Native Frontend

Ubuntu 优先的原生前端实现，基于 `GTK4 + libadwaita`，通过 `app-core` 复用共享业务逻辑。

## 功能范围（MVP）

- Server / Client 模式切换
- 服务启停
- 状态显示（运行态、连接数、当前激活客户端）
- 客户端映射编辑（文本格式）
- 日志面板
- 配置保存与启动恢复（通过 `app-core` 配置接口）

## 本地依赖

```bash
sudo apt update
sudo apt install -y libgtk-4-dev libadwaita-1-dev pkg-config
```

## 运行

```bash
cd gui-gtk
cargo run
```
