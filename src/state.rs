use a2s::A2SClient;
use local_ip_address::local_ip;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, LazyLock, OnceLock};

// Global state
pub static IS_HOOKED: AtomicBool = AtomicBool::new(false);
pub static DEBUG_MODE: AtomicBool = AtomicBool::new(false);
pub static PID: LazyLock<u32> = LazyLock::new(std::process::id);

// Global Timer based state
pub static GLOBAL_TIMER_STATE: OnceLock<(Arc<AtomicU64>, Arc<AtomicBool>)> = OnceLock::new();

// Global A2S state
pub static A2S_CLIENT: LazyLock<A2SClient> = LazyLock::new(|| A2SClient::new().unwrap());
pub static LOCAL_IP: LazyLock<String> = LazyLock::new(|| local_ip().unwrap().to_string());
