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

final class BarrierCoreBridge: ObservableObject {
    @Published var status: CoreStatus = CoreStatus(
        mode: "server",
        is_running: false,
        connected_clients: 0,
        server_address: "",
        active_client: nil,
        log_messages: []
    )
    @Published var lastError: String = ""

    private var handle: UnsafeMutableRawPointer?

    init() {
        handle = barrier_core_new()
        refreshStatus()
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

    private func consumeErrorPointer(_ pointer: UnsafeMutablePointer<CChar>?) {
        guard let pointer else {
            lastError = ""
            return
        }
        lastError = String(cString: pointer)
        barrier_core_free_string(pointer)
    }
}
