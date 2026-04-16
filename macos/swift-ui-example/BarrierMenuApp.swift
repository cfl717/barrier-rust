// 最小 SwiftUI 示例：通过 C ABI 调用 Rust 核心。
// 1) cargo build -p barrier_core_ffi --release
// 2) Xcode 新建 macOS App，加入本文件
// 3) Build Settings → Library Search Paths → $(PROJECT_DIR)/../../target/release（按实际路径调整）
// 4) Other Linker Flags → -lbarrier_core_ffi
// 5) Header Search Paths → app-core-ffi/include（用于 IDE，运行时可不编译头文件）
//
// 下列声明等价于 Bridging Header 导入 barrier_core_ffi.h

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

struct ContentView: View {
    @State private var isServerMode: Bool = true
    @State private var port: String = "24800"
    @State private var screenName: String = "server"
    @State private var serverAddr: String = "192.168.5.5:24800"
    @State private var clientName: String = "client-1"
    @State private var status: String = "未连接核心"
    @State private var core: OpaquePointer?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Barrier（macOS 原生壳示例）")
                .font(.title2)
                .bold()

            Picker("运行模式", selection: $isServerMode) {
                Text("服务端 (Server)").tag(true)
                Text("客户端 (Client)").tag(false)
            }
            .pickerStyle(SegmentedPickerStyle())

            Divider()

            if isServerMode {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("监听端口:")
                            .frame(width: 80, alignment: .trailing)
                        TextField("例如 24800", text: $port)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .frame(width: 100)
                    }
                    HStack {
                        Text("屏幕名称:")
                            .frame(width: 80, alignment: .trailing)
                        TextField("例如 server", text: $screenName)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }
                    Button("启动 Server") { startServer() }
                        .buttonStyle(.borderedProminent)
                        .padding(.top, 8)
                }
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("服务端 IP:")
                            .frame(width: 80, alignment: .trailing)
                        TextField("例如 192.168.5.5:24800", text: $serverAddr)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }
                    HStack {
                        Text("客户端名称:")
                            .frame(width: 80, alignment: .trailing)
                        TextField("例如 client-1", text: $clientName)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }
                    Button("启动 Client") { startClient() }
                        .buttonStyle(.borderedProminent)
                        .padding(.top, 8)
                }
            }

            Divider()

            HStack {
                Button("停止运行") { stopCore() }
                Spacer()
                Button("刷新状态") { refreshStatus() }
            }

            Text(status)
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(.top, 4)
        }
        .padding()
        .frame(minWidth: 400, minHeight: 300)
        .onAppear {
            core = barrier_core_new()
            refreshStatus()
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
}
