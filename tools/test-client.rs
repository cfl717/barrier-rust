#!/usr/bin/env -S cargo +nightly -Zscript
//! 简单测试客户端，用于验证 Server-Client 连接
//! 用法: cargo run --example test-client -- [server_addr] [client_name]

use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;

const BARRIER_VERSION: &str = "Barrier 1.0";

fn main() {
    let args: Vec<String> = env::args().collect();
    let server_addr = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1:24800");
    let client_name = args.get(2).map(|s| s.as_str()).unwrap_or("test-client");

    println!("连接到 Server: {}", server_addr);
    println!("客户端名称: {}", client_name);

    let mut stream = match TcpStream::connect(server_addr) {
        Ok(s) => {
            println!("✓ TCP 连接成功");
            s
        }
        Err(e) => {
            eprintln!("✗ 连接失败: {}", e);
            return;
        }
    };

    // 发送 Client Hello: CBYQ + payload
    let hello_payload = format!("{}\n{}\n", BARRIER_VERSION, client_name);
    let msg_type = b"CBYQ";
    let payload_len = hello_payload.len() as u32;
    
    let mut packet = Vec::new();
    packet.extend_from_slice(&payload_len.to_be_bytes());
    packet.extend_from_slice(msg_type);
    packet.extend_from_slice(hello_payload.as_bytes());

    println!("发送 CBYQ: {:?}", hello_payload.trim());
    if let Err(e) = stream.write_all(&packet) {
        eprintln!("✗ 发送失败: {}", e);
        return;
    }
    stream.flush().unwrap();

    // 读取 Server 响应
    let mut len_buf = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut len_buf) {
        eprintln!("✗ 读取响应长度失败: {}", e);
        return;
    }
    let resp_len = u32::from_be_bytes(len_buf) as usize;
    
    let mut resp_buf = vec![0u8; resp_len];
    if let Err(e) = stream.read_exact(&mut resp_buf) {
        eprintln!("✗ 读取响应内容失败: {}", e);
        return;
    }

    let msg_type = String::from_utf8_lossy(&resp_buf[0..4]);
    let payload = String::from_utf8_lossy(&resp_buf[4..]);
    println!("✓ 收到响应: {} - {:?}", msg_type, payload.trim());

    if msg_type == "QBYC" {
        println!("✓ 握手成功！客户端已注册为: {}", client_name);
        println!("\n保持连接中... 按 Ctrl+C 退出");
        
        // 保持连接，等待 Server 消息
        loop {
            let mut len_buf = [0u8; 4];
            match stream.read_exact(&mut len_buf) {
                Ok(_) => {
                    let msg_len = u32::from_be_bytes(len_buf) as usize;
                    let mut msg_buf = vec![0u8; msg_len];
                    if stream.read_exact(&mut msg_buf).is_ok() {
                        let msg_type = String::from_utf8_lossy(&msg_buf[0..4.min(msg_buf.len())]);
                        println!("收到消息: {} ({}字节)", msg_type, msg_len);
                        
                        if msg_type == "CINN" {
                            println!("  → CLIENT_ENTER: 输入已切换到此客户端");
                        } else if msg_type == "COUT" {
                            println!("  → CLIENT_LEAVE: 输入已切回 Server");
                        } else if msg_type == "CMOV" {
                            println!("  → MOUSE_MOVE");
                        } else if msg_type == "CBUT" {
                            println!("  → MOUSE_BUTTON");
                        } else if msg_type == "CKDn" || msg_type == "CKUp" {
                            println!("  → KEY_EVENT");
                        }
                    }
                }
                Err(_) => {
                    println!("连接已断开");
                    break;
                }
            }
        }
    } else {
        eprintln!("✗ 握手失败，收到意外响应: {}", msg_type);
    }
}
