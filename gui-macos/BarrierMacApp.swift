import SwiftUI

@main
struct BarrierMacApp: App {
    @StateObject private var core = BarrierCoreBridge()
    @State private var mode: String = "server"
    @State private var serverPort: String = "24800"
    @State private var serverAddress: String = "localhost:24800"
    @State private var clientName: String = "client-1"
    @State private var serverScreenName: String = "server"

    var body: some Scene {
        WindowGroup {
            VStack(alignment: .leading, spacing: 12) {
                Text("Barrier macOS 原生前端（最小桥接版）")
                    .font(.title3)

                Picker("模式", selection: $mode) {
                    Text("Server").tag("server")
                    Text("Client").tag("client")
                }
                .pickerStyle(.segmented)

                HStack {
                    TextField("Server 端口", text: $serverPort)
                    TextField("Server 名称", text: $serverScreenName)
                }
                HStack {
                    TextField("Server 地址", text: $serverAddress)
                    TextField("Client 名称", text: $clientName)
                }

                HStack {
                    Button("启动") {
                        if mode == "server" {
                            let port = UInt16(serverPort) ?? 24800
                            core.startServer(port: port, screenName: serverScreenName)
                        } else {
                            core.startClient(serverAddr: serverAddress, clientName: clientName)
                        }
                    }
                    Button("停止") {
                        core.stop()
                    }
                    Button("刷新状态") {
                        core.refreshStatus()
                    }
                }

                Text("运行状态: \(core.status.is_running ? "running" : "stopped") | mode=\(core.status.mode)")
                Text("地址: \(core.status.server_address)")
                Text("连接客户端数: \(core.status.connected_clients)")
                if !core.lastError.isEmpty {
                    Text("错误: \(core.lastError)")
                        .foregroundColor(.red)
                }

                Text("日志")
                    .font(.headline)
                ScrollView {
                    Text(core.status.log_messages.joined(separator: "\n"))
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .font(.system(size: 12, design: .monospaced))
                }
            }
            .padding(16)
            .frame(minWidth: 900, minHeight: 620)
        }
    }
}
