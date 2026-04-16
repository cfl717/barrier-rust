use app_core::{BarrierCore, ClientConfig, ServerConfig};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct CoreHandle {
    runtime: Runtime,
    core: Arc<BarrierCore>,
}

fn read_cstr(input: *const c_char, field: &str) -> Result<String, String> {
    if input.is_null() {
        return Err(format!("{} 不能为空", field));
    }
    let value = unsafe { CStr::from_ptr(input) };
    value
        .to_str()
        .map(|v| v.to_string())
        .map_err(|_| format!("{} 不是有效 UTF-8 字符串", field))
}

fn to_c_string(payload: String) -> *mut c_char {
    match CString::new(payload) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => CString::new("内部错误：字符串包含非法 NUL 字节")
            .expect("static str to CString")
            .into_raw(),
    }
}

fn result_to_error_ptr(result: Result<(), String>) -> *mut c_char {
    match result {
        Ok(()) => std::ptr::null_mut(),
        Err(err) => to_c_string(err),
    }
}

#[no_mangle]
pub extern "C" fn barrier_core_new() -> *mut CoreHandle {
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };
    let core = Arc::new(BarrierCore::new());
    runtime.block_on(core.initialize_platform_features());
    let handle = CoreHandle { runtime, core };
    Box::into_raw(Box::new(handle))
}

#[no_mangle]
pub extern "C" fn barrier_core_free(handle: *mut CoreHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub extern "C" fn barrier_core_start_server(
    handle: *mut CoreHandle,
    port: u16,
    screen_name: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return to_c_string("core handle 为空".to_string());
    }
    let screen_name = match read_cstr(screen_name, "screen_name") {
        Ok(v) if !v.trim().is_empty() => v,
        Ok(_) => "server".to_string(),
        Err(err) => return to_c_string(err),
    };
    let config = ServerConfig {
        screen_name,
        port,
        max_clients: 10,
        listen_address: Some(format!("0.0.0.0:{port}")),
        enable_clipboard: true,
        enable_drag_drop: true,
    };
    let core_handle = unsafe { &mut *handle };
    let result = core_handle.runtime.block_on(core_handle.core.start_server(config));
    result_to_error_ptr(result)
}

#[no_mangle]
pub extern "C" fn barrier_core_start_client(
    handle: *mut CoreHandle,
    server_addr: *const c_char,
    client_name: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return to_c_string("core handle 为空".to_string());
    }
    let server_addr = match read_cstr(server_addr, "server_addr") {
        Ok(v) if !v.trim().is_empty() => v,
        Ok(_) => "localhost:24800".to_string(),
        Err(err) => return to_c_string(err),
    };
    let client_name = match read_cstr(client_name, "client_name") {
        Ok(v) if !v.trim().is_empty() => v,
        Ok(_) => "client-1".to_string(),
        Err(err) => return to_c_string(err),
    };
    let config = ClientConfig {
        server_addr,
        screen_name: client_name,
        auto_reconnect: true,
        reconnect_interval: 5,
        enable_clipboard: true,
        enable_drag_drop: true,
    };
    let core_handle = unsafe { &mut *handle };
    let result = core_handle.runtime.block_on(core_handle.core.start_client(config));
    result_to_error_ptr(result)
}

#[no_mangle]
pub extern "C" fn barrier_core_stop(handle: *mut CoreHandle) -> *mut c_char {
    if handle.is_null() {
        return to_c_string("core handle 为空".to_string());
    }
    let core_handle = unsafe { &mut *handle };
    let result = core_handle.runtime.block_on(core_handle.core.stop_service());
    result_to_error_ptr(result)
}

#[no_mangle]
pub extern "C" fn barrier_core_get_status_json(handle: *mut CoreHandle) -> *mut c_char {
    if handle.is_null() {
        return to_c_string(r#"{"error":"core handle 为空"}"#.to_string());
    }
    let core_handle = unsafe { &mut *handle };
    let payload = match core_handle.runtime.block_on(core_handle.core.get_status()) {
        Ok(status) => serde_json::to_string(&status).unwrap_or_else(|e| {
            format!(
                r#"{{"mode":"server","is_running":false,"connected_clients":0,"server_address":"","active_client":null,"log_messages":["序列化失败: {}"]}}"#,
                e
            )
        }),
        Err(err) => format!(
            r#"{{"mode":"server","is_running":false,"connected_clients":0,"server_address":"","active_client":null,"log_messages":["{}"]}}"#,
            err
        ),
    };
    to_c_string(payload)
}

#[no_mangle]
pub extern "C" fn barrier_core_get_settings_json(handle: *mut CoreHandle) -> *mut c_char {
    if handle.is_null() {
        return to_c_string(r#"{"error":"core handle 为空"}"#.to_string());
    }
    let core_handle = unsafe { &mut *handle };
    let payload = serde_json::to_string(&core_handle.runtime.block_on(core_handle.core.get_settings()))
        .unwrap_or_else(|e| format!(r#"{{"error":"配置序列化失败: {}"}}"#, e));
    to_c_string(payload)
}

#[no_mangle]
pub extern "C" fn barrier_core_free_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(value));
    }
}
