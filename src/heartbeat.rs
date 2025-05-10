use crate::state::{A2S_CLIENT, DEBUG_MODE, IS_HOOKED, LOCAL_IP};
use crate::util::get_current_time;
use crate::util::kill_process;
use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64}};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum HealCheckMode {
    TimerLegacy {
        threshold: u64,
        last_timestamp: Arc<AtomicU64>,
        pause: Arc<AtomicBool>,
    },
    A2S {
        retry_count: u8,
        port: u16,
    },
    RCON {
        retry_count: u8,
        port: u16,
        password: String,
    },
}

impl HealCheckMode {
    fn check_a2s_health(retry_count: u8, port: u16) -> bool {
        let address = format!("{}:{port}", *LOCAL_IP);
        for i in 0..retry_count {
            if A2S_CLIENT.info(&address).is_ok() {
                return true;
            }
            if DEBUG_MODE.load(Ordering::Relaxed) {
                println!("[gHeartbeat DEBUG] A2S attempt [{i}/{retry_count}] failed for {address}");
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
    }
    
    fn check_rcon_health(retry_count: u8, port: u16, password: &str) -> bool {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        
        const REQUEST_ID: i32 = 12345;
        const SERVERDATA_AUTH: i32 = 3;
        const SERVERDATA_AUTH_RESPONSE: i32 = 2;
        
        let addr = format!("{}:{}", *LOCAL_IP, port);
        
        for i in 0..retry_count {
            match TcpStream::connect(&addr) {
                Ok(mut stream) => {
                    // 设置读写超时
                    if let Err(_) = stream.set_read_timeout(Some(Duration::from_secs(5))) {
                        if DEBUG_MODE.load(Ordering::Relaxed) {
                            println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] failed to set read timeout for {addr}");
                        }
                        continue;
                    }
                    
                    if let Err(_) = stream.set_write_timeout(Some(Duration::from_secs(5))) {
                        if DEBUG_MODE.load(Ordering::Relaxed) {
                            println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] failed to set write timeout for {addr}");
                        }
                        continue;
                    }

                    // 准备RCON认证数据包
                    let password_bytes = password.as_bytes();
                    let body_len = password_bytes.len() + 1 + 1;
                    let packet_size = 4 + 4 + body_len as i32;

                    let mut request_packet = Vec::with_capacity(4 + packet_size as usize);
                    request_packet.extend_from_slice(&packet_size.to_le_bytes());
                    request_packet.extend_from_slice(&REQUEST_ID.to_le_bytes());
                    request_packet.extend_from_slice(&SERVERDATA_AUTH.to_le_bytes());
                    request_packet.extend_from_slice(password_bytes);
                    request_packet.push(0);
                    request_packet.push(0);

                    // 发送认证请求
                    if let Err(_) = stream.write_all(&request_packet) {
                        if DEBUG_MODE.load(Ordering::Relaxed) {
                            println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] failed to write auth packet to {addr}");
                        }
                        continue;
                    }

                    // 读取响应大小
                    let mut response_size_bytes = [0u8; 4];
                    if let Err(_) = stream.read_exact(&mut response_size_bytes) {
                        if DEBUG_MODE.load(Ordering::Relaxed) {
                            println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] failed to read response size from {addr}");
                        }
                        continue;
                    }
                    
                    let response_size = i32::from_le_bytes(response_size_bytes);
                    if response_size < 10 {
                        if DEBUG_MODE.load(Ordering::Relaxed) {
                            println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] received undersized response ({} bytes) from {addr}", response_size);
                        }
                        continue;
                    }
                    
                    // 读取响应内容
                    let mut response_payload = vec![0u8; response_size as usize]; 
                    if let Err(_) = stream.read_exact(&mut response_payload) {
                        if DEBUG_MODE.load(Ordering::Relaxed) {
                            println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] failed to read response payload from {addr}");
                        }
                        continue;
                    }

                    // 解析响应
                    let response_id = i32::from_le_bytes(response_payload[0..4].try_into().unwrap_or_default());
                    let response_type = i32::from_le_bytes(response_payload[4..8].try_into().unwrap_or_default());
                    
                    if response_type == SERVERDATA_AUTH_RESPONSE && response_id == REQUEST_ID {
                        if DEBUG_MODE.load(Ordering::Relaxed) {
                            println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] auth successful for {addr}");
                        }
                        return true;
                    } else if DEBUG_MODE.load(Ordering::Relaxed) {
                        println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] auth failed for {addr}. ID: {response_id}, Type: {response_type} (Expected ID: {REQUEST_ID}, Expected Type: {SERVERDATA_AUTH_RESPONSE})");
                    }
                }
                Err(e) => {
                    if DEBUG_MODE.load(Ordering::Relaxed) {
                        println!("[gHeartbeat DEBUG] RCON attempt [{i}/{retry_count}] connection to {addr} failed: {e}");
                    }
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
        false
    }

    pub fn is_health(&self) -> bool {
        if DEBUG_MODE.load(Ordering::Relaxed) {
            println!("[gHeartbeat DEBUG] Acquire check with {self:?}");
        }
        match self {
            HealCheckMode::TimerLegacy {
                threshold,
                last_timestamp,
                pause,
            } => {
                let current_time = get_current_time();
                let last_ping_time = last_timestamp.load(Ordering::Relaxed);
                let is_within_threshold = current_time - last_ping_time <= *threshold;
                let is_not_paused = !pause.load(Ordering::Relaxed);
                
                is_within_threshold && is_not_paused
            }
            HealCheckMode::A2S { retry_count, port } => {
                Self::check_a2s_health(*retry_count, *port)
            }
            HealCheckMode::RCON { retry_count, port, password } => {
                Self::check_rcon_health(*retry_count, *port, password)
            }
        }
    }
}

pub fn bg_check_health(interval: u64, healthcheck: HealCheckMode) {
    println!("[gHeartbeat] Attached health check");
    println!("[gHeartbeat] Check interval {interval} seconds");
    IS_HOOKED.store(true, Ordering::Relaxed);

    // 循环直到健康检查失败或钩子被移除
    while healthcheck.is_health() && IS_HOOKED.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_secs(interval));
    }

    // 根据钩子状态决定退出原因
    if IS_HOOKED.load(Ordering::Relaxed) {
        println!("[gHeartbeat] Server health check failed");
        kill_process();
    } else {
        println!("[gHeartbeat] Receive exit request");
    }
}
