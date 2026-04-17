// 最小 SwiftUI 示例：通过 C ABI 调用 Rust 核心。
// 1) cargo build -p barrier_core_ffi --release
// 2) Xcode 新建 macOS App，加入本文件
// 3) Build Settings → Library Search Paths → $(PROJECT_DIR)/../../target/release（按实际路径调整）
// 4) Other Linker Flags → -lbarrier_core_ffi
// 5) Header Search Paths → app-core-ffi/include（用于 IDE，运行时可不编译头文件）
//
// 下列声明等价于 Bridging Header 导入 barrier_core_ffi.h

import AppKit
import SwiftUI

@_silgen_name("barrier_core_new")
func barrier_core_new() -> OpaquePointer?

@_silgen_name("barrier_core_free")
func barrier_core_free(_ handle: OpaquePointer?)

@_silgen_name("barrier_core_start_server")
func barrier_core_start_server(_ handle: OpaquePointer?, _ port: UInt16, _ screenName: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("barrier_core_start_client")
func barrier_core_start_client(_ handle: OpaquePointer?, _ serverAddr: UnsafePointer<CChar>?, _ clientName: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("barrier_core_stop")
func barrier_core_stop(_ handle: OpaquePointer?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("barrier_core_get_status_json")
func barrier_core_get_status_json(_ handle: OpaquePointer?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("barrier_core_free_string")
func barrier_core_free_string(_ value: UnsafeMutablePointer<CChar>?)

@main
struct BarrierMenuApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

private struct CoreStatusSnapshot: Decodable {
    let mode: String
    let is_running: Bool
    let connected_clients: Int
    let server_address: String
    let active_client: String?
    let log_messages: [String]
}

struct ContentView: View {
    @State private var isServerMode: Bool = true
    @State private var port: String = "24800"
    @State private var screenName: String = "server"
    @State private var serverAddr: String = "192.168.5.5:24800"
    @State private var clientName: String = "client-1"
    @State private var status: String = "未连接核心"
    @State private var statusSummary: String = "等待连接"
    @State private var logs: [String] = []
    @State private var autoRefreshEnabled: Bool = true
    private let refreshTimer = Timer.publish(every: 1.0, on: .main, in: .common).autoconnect()
    @State private var core: OpaquePointer?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Barrier（macOS 原生客户端）")
                .font(.title3)
                .bold()

            Picker("运行模式", selection: $isServerMode) {
                Text("服务端").tag(true)
                Text("客户端").tag(false)
            }
            .pickerStyle(.segmented)

            GroupBox("连接配置") {
                VStack(alignment: .leading, spacing: 10) {
                    if isServerMode {
                        HStack {
                            Text("监听端口")
                                .frame(width: 84, alignment: .trailing)
                            TextField("24800", text: $port)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 120)
                            Text("屏幕名称")
                                .frame(width: 84, alignment: .trailing)
                            TextField("server", text: $screenName)
                                .textFieldStyle(.roundedBorder)
                        }
                        Button("启动 Server") { startServer() }
                            .buttonStyle(.borderedProminent)
                    } else {
                        HStack {
                            Text("服务端地址")
                                .frame(width: 84, alignment: .trailing)
                            TextField("192.168.5.5:24800", text: $serverAddr)
                                .textFieldStyle(.roundedBorder)
                        }
                        HStack {
                            Text("客户端名称")
                                .frame(width: 84, alignment: .trailing)
                            TextField("client-1", text: $clientName)
                                .textFieldStyle(.roundedBorder)
                        }
                        Button("启动 Client") { startClient() }
                            .buttonStyle(.borderedProminent)
                    }
                }
                .padding(.top, 4)
            }

            GroupBox("运行状态") {
                VStack(alignment: .leading, spacing: 8) {
                    Text(statusSummary)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                    Text(status)
                        .font(.caption.monospaced())
                        .textSelection(.enabled)
                        .lineLimit(3)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.top, 4)
            }

            GroupBox("客户端诊断日志") {
                ScrollView {
                    VStack(alignment: .leading, spacing: 4) {
                        ForEach(Array(logs.enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.caption.monospaced())
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .textSelection(.enabled)
                        }
                    }
                    .padding(.vertical, 4)
                }
                .frame(minHeight: 170)
            }

            HStack {
                Button("停止运行") { stopCore() }
                Button("刷新状态") { refreshStatus() }
                Toggle("自动刷新", isOn: $autoRefreshEnabled)
                    .toggleStyle(.checkbox)
                Spacer()
                Button("复制诊断") { copyDiagnostics() }
            }
        }
        .padding(16)
        .frame(minWidth: 640, minHeight: 560)
        .onAppear {
            core = barrier_core_new()
            refreshStatus()
        }
        .onReceive(refreshTimer) { _ in
            if autoRefreshEnabled {
                refreshStatus()
            }
        }
        .onDisappear {
            if let h = core {
                barrier_core_free(h)
                core = nil
            }
        }
    }

    private func refreshStatus() {
        guard let h = core else { return }
        guard let json = barrier_core_get_status_json(h) else { return }
        let s = String(cString: json)
        barrier_core_free_string(json)
        status = s
        updateStatusSummaryAndLogs(from: s)
    }

    private func startServer() {
        guard let h = core else { return }
        let p = UInt16(port) ?? 24800
        let err: UnsafeMutablePointer<CChar>? = screenName.withCString { sn in
            barrier_core_start_server(h, p, sn)
        }
        if let err {
            let msg = String(cString: err)
            barrier_core_free_string(err)
            status = "错误: \(msg)"
        } else {
            refreshStatus()
        }
    }

    private func startClient() {
        guard let h = core else { return }
        let err: UnsafeMutablePointer<CChar>? = serverAddr.withCString { sa in
            clientName.withCString { cn in
                barrier_core_start_client(h, sa, cn)
            }
        }
        if let err {
            let msg = String(cString: err)
            barrier_core_free_string(err)
            status = "错误: \(msg)"
        } else {
            refreshStatus()
        }
    }

    private func stopCore() {
        guard let h = core else { return }
        let err = barrier_core_stop(h)
        if let err {
            let msg = String(cString: err)
            barrier_core_free_string(err)
            status = "错误: \(msg)"
        } else {
            refreshStatus()
        }
    }

    private func updateStatusSummaryAndLogs(from raw: String) {
        guard let data = raw.data(using: .utf8) else {
            statusSummary = "状态解析失败：非 UTF-8"
            return
        }
        do {
            let decoded = try JSONDecoder().decode(CoreStatusSnapshot.self, from: data)
            let active = decoded.active_client ?? "-"
            statusSummary = "mode=\(decoded.mode) | running=\(decoded.is_running) | clients=\(decoded.connected_clients) | active=\(active) | server=\(decoded.server_address)"
            logs = decoded.log_messages
        } catch {
            statusSummary = "状态解析失败：\(error.localizedDescription)"
        }
    }

    private func copyDiagnostics() {
        let content = """
        ===== Barrier Diagnostics =====
        \(statusSummary)

        ---- Raw Status JSON ----
        \(status)

        ---- Log Messages ----
        \(logs.joined(separator: "\n"))
        """
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(content, forType: .string)
    }
}
