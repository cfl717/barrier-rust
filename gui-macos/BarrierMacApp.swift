import SwiftUI

@main
struct BarrierMacApp: App {
    @StateObject private var core = BarrierCoreBridge()

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
        }
    }
}

struct ContentView: View {
    @ObservedObject var core: BarrierCoreBridge
    @State private var mode: String = "server"
    @State private var serverPort: String = "24800"
    @State private var serverScreenName: String = "server"
    @State private var serverAddress: String = "localhost:24800"
    @State private var clientName: String = "client-1"
    @State private var routes: [ClientRoute] = [
        ClientRoute(id: "route-0", name: "client-1", address: "192.168.1.101:24800", edge: "right")
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Barrier macOS")
                .font(.title2)
                .fontWeight(.semibold)

            // 模式选择
            Picker("模式", selection: $mode) {
                Text("Server（共享本机键鼠）").tag("server")
                Text("Client（接收远程键鼠）").tag("client")
            }
            .pickerStyle(.segmented)
            .onChange(of: mode) { _ in
                core.settings.last_mode = mode
            }

            // Server 配置面板
            if mode == "server" {
                GroupBox("Server 配置") {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("监听端口:")
                                .frame(width: 80, alignment: .leading)
                            TextField("24800", text: $serverPort)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)
                            Text("屏幕名称:")
                                .frame(width: 80, alignment: .leading)
                            TextField("server", text: $serverScreenName)
                                .textFieldStyle(.roundedBorder)
                        }

                        Divider()

                        Text("客户端映射:")
                            .fontWeight(.medium)

                        ForEach($routes) { $route in
                            HStack {
                                TextField("名称", text: $route.name)
                                    .textFieldStyle(.roundedBorder)
                                    .frame(width: 100)
                                TextField("地址:端口", text: $route.address)
                                    .textFieldStyle(.roundedBorder)
                                Picker("Edge", selection: $route.edge) {
                                    Text("← 左").tag("left")
                                    Text("→ 右").tag("right")
                                    Text("↑ 上").tag("top")
                                    Text("↓ 下").tag("bottom")
                                }
                                .frame(width: 80)
                                Button(action: {
                                    routes.removeAll { $0.id == route.id }
                                }) {
                                    Image(systemName: "minus.circle.fill")
                                        .foregroundColor(.red)
                                }
                                .buttonStyle(.borderless)
                            }
                        }

                        Button(action: {
                            let newId = "route-\(routes.count)"
                            routes.append(ClientRoute(id: newId, name: "", address: "", edge: "right"))
                        }) {
                            Label("添加客户端", systemImage: "plus.circle")
                        }
                    }
                    .padding(8)
                }
            }

            // Client 配置面板
            if mode == "client" {
                GroupBox("Client 配置") {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("Server 地址:")
                                .frame(width: 90, alignment: .leading)
                            TextField("192.168.1.100:24800", text: $serverAddress)
                                .textFieldStyle(.roundedBorder)
                        }
                        HStack {
                            Text("本机名称:")
                                .frame(width: 90, alignment: .leading)
                            TextField("client-1", text: $clientName)
                                .textFieldStyle(.roundedBorder)
                        }
                    }
                    .padding(8)
                }
            }

            // 操作按钮
            HStack(spacing: 12) {
                Button("启动") {
                    if mode == "server" {
                        let port = UInt16(serverPort) ?? 24800
                        core.startServer(port: port, screenName: serverScreenName)
                    } else {
                        core.startClient(serverAddr: serverAddress, clientName: clientName)
                    }
                }
                .buttonStyle(.borderedProminent)

                Button("停止") {
                    core.stop()
                }
                .buttonStyle(.bordered)

                Button("保存配置") {
                    syncSettingsToCore()
                    core.saveSettings()
                }
                .buttonStyle(.bordered)
            }

            // Server 模式下的切换按钮
            if mode == "server" {
                HStack(spacing: 12) {
                    Button("切换到首个客户端") {
                        if let first = routes.first, !first.name.isEmpty {
                            core.switchClient(name: first.name, edge: first.edge)
                        } else {
                            core.lastError = "没有配置客户端映射"
                        }
                    }
                    .buttonStyle(.bordered)

                    Button("切回本机") {
                        core.switchBack()
                    }
                    .buttonStyle(.bordered)
                }
            }

            // 状态显示
            GroupBox("状态") {
                VStack(alignment: .leading, spacing: 4) {
                    let activeStr = core.status.active_client ?? "无"
                    let hint: String = {
                        if mode == "server" && core.status.is_running {
                            if core.status.connected_clients > 0 && core.status.active_client != nil {
                                return " ✓ 输入已转发"
                            } else if core.status.connected_clients > 0 {
                                return " ⚠ 点击「切换到首个客户端」激活"
                            } else {
                                return " ○ 等待客户端连接"
                            }
                        } else if mode == "client" && core.status.is_running {
                            return " ○ 等待 Server 激活"
                        }
                        return ""
                    }()

                    Text("服务: \(core.status.is_running ? "运行中" : "已停止") | 客户端: \(core.status.connected_clients) | 激活: \(activeStr)\(hint)")
                        .font(.system(size: 12))

                    if !core.lastMessage.isEmpty {
                        Text(core.lastMessage)
                            .font(.system(size: 11))
                            .foregroundColor(.green)
                    }

                    if !core.lastError.isEmpty {
                        Text(core.lastError)
                            .font(.system(size: 11))
                            .foregroundColor(.red)
                    }
                }
                .padding(4)
            }

            // 日志
            GroupBox("日志") {
                ScrollViewReader { proxy in
                    ScrollView {
                        VStack(alignment: .leading, spacing: 2) {
                            ForEach(Array(core.status.log_messages.enumerated()), id: \.offset) { index, line in
                                Text(line)
                                    .font(.system(size: 11, design: .monospaced))
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .id(index)
                            }
                        }
                        .padding(4)
                    }
                    .onChange(of: core.status.log_messages.count) { _ in
                        if let lastIndex = core.status.log_messages.indices.last {
                            withAnimation {
                                proxy.scrollTo(lastIndex, anchor: .bottom)
                            }
                        }
                    }
                }
                .frame(minHeight: 150)
            }
        }
        .padding(16)
        .frame(minWidth: 700, minHeight: 600)
        .onAppear {
            loadSettingsToUI()
            startStatusPolling()
        }
    }

    private func loadSettingsToUI() {
        mode = core.settings.last_mode
        serverPort = String(core.settings.server_port)
        serverScreenName = core.settings.server_screen_name
        serverAddress = core.settings.server_address
        clientName = core.settings.client_name
        if !core.settings.client_routes.isEmpty {
            routes = core.settings.client_routes
        }
    }

    private func syncSettingsToCore() {
        core.settings.last_mode = mode
        core.settings.server_port = UInt16(serverPort) ?? 24800
        core.settings.server_screen_name = serverScreenName
        core.settings.server_address = serverAddress
        core.settings.client_name = clientName
        core.settings.client_routes = routes.filter { !$0.name.isEmpty }
    }

    private func startStatusPolling() {
        Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            core.refreshStatus()
        }
    }
}
