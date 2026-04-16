import Foundation
import SwiftUI

struct CoreStatus: Codable {
    let mode: String
    let is_running: Bool
    let connected_clients: Int
    let server_address: String
    let active_client: String?
    let log_messages: [String]
}

struct ClientRoute: Codable, Identifiable {
    var id: String
    var name: String
    var address: String
    var edge: String
}

struct AppSettings: Codable {
    var startup_restore: Bool
    var last_mode: String
    var server_port: UInt16
    var server_screen_name: String
    var server_address: String
    var client_name: String
    var pointer_lock_enabled: Bool
    var client_routes: [ClientRoute]
}

final class BarrierCoreBridge: ObservableObject {
    @Published var status: CoreStatus = CoreStatus(
        mode: "server",
        is_running: false,
        connected_clients: 0,
        server_address: "",
        active_client: nil,
        log_messages: []
    )
    @Published var settings: AppSettings = AppSettings(
        startup_restore: true,
        last_mode: "server",
        server_port: 24800,
        server_screen_name: "server",
        server_address: "localhost:24800",
        client_name: "client-1",
        pointer_lock_enabled: true,
        client_routes: []
    )
    @Published var lastError: String = ""
    @Published var lastMessage: String = ""

    private var handle: UnsafeMutableRawPointer?

    init() {
        handle = barrier_core_new()
        refreshStatus()
        loadSettings()
    }

    deinit {
        if let handle {
            barrier_core_free(handle)
        }
    }

    func startServer(port: UInt16, screenName: String) {
        guard let handle else { return }
        let errorPtr = screenName.withCString { ptr in
            barrier_core_start_server(handle, port, ptr)
        }
        consumeErrorPointer(errorPtr)
        refreshStatus()
    }

    func startClient(serverAddr: String, clientName: String) {
        guard let handle else { return }
        let errorPtr = serverAddr.withCString { serverPtr in
            clientName.withCString { clientPtr in
                barrier_core_start_client(handle, serverPtr, clientPtr)
            }
        }
        consumeErrorPointer(errorPtr)
        refreshStatus()
    }

    func stop() {
        guard let handle else { return }
        let errorPtr = barrier_core_stop(handle)
        consumeErrorPointer(errorPtr)
        refreshStatus()
    }

    func switchClient(name: String, edge: String) {
        guard let handle else { return }
        let resultPtr = name.withCString { namePtr in
            edge.withCString { edgePtr in
                barrier_core_switch_client(handle, namePtr, edgePtr)
            }
        }
        consumeResultPointer(resultPtr)
        refreshStatus()
    }

    func switchBack() {
        guard let handle else { return }
        let resultPtr = barrier_core_switch_back(handle)
        consumeResultPointer(resultPtr)
        refreshStatus()
    }

    func refreshStatus() {
        guard let handle else { return }
        guard let raw = barrier_core_get_status_json(handle) else { return }
        let payload = String(cString: raw)
        barrier_core_free_string(raw)
        guard let data = payload.data(using: .utf8) else { return }
        do {
            let decoded = try JSONDecoder().decode(CoreStatus.self, from: data)
            status = decoded
        } catch {
            lastError = "状态解析失败: \(error.localizedDescription)"
        }
    }

    func loadSettings() {
        guard let handle else { return }
        guard let raw = barrier_core_get_settings_json(handle) else { return }
        let payload = String(cString: raw)
        barrier_core_free_string(raw)
        guard let data = payload.data(using: .utf8) else { return }
        do {
            let decoded = try JSONDecoder().decode(AppSettings.self, from: data)
            settings = decoded
        } catch {
            lastError = "配置解析失败: \(error.localizedDescription)"
        }
    }

    func saveSettings() {
        guard let handle else { return }
        do {
            let jsonData = try JSONEncoder().encode(settings)
            guard let jsonStr = String(data: jsonData, encoding: .utf8) else { return }
            let errorPtr = jsonStr.withCString { ptr in
                barrier_core_save_settings_json(handle, ptr)
            }
            consumeErrorPointer(errorPtr)
            if lastError.isEmpty {
                lastMessage = "配置已保存"
            }
        } catch {
            lastError = "配置序列化失败: \(error.localizedDescription)"
        }
    }

    private func consumeErrorPointer(_ pointer: UnsafeMutablePointer<CChar>?) {
        guard let pointer else {
            lastError = ""
            return
        }
        lastError = String(cString: pointer)
        barrier_core_free_string(pointer)
    }

    private func consumeResultPointer(_ pointer: UnsafeMutablePointer<CChar>?) {
        guard let pointer else {
            lastError = ""
            lastMessage = ""
            return
        }
        let result = String(cString: pointer)
        barrier_core_free_string(pointer)
        if result.hasPrefix("OK:") {
            lastError = ""
            lastMessage = String(result.dropFirst(3))
        } else {
            lastError = result
            lastMessage = ""
        }
    }
}
