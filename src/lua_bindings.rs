use crate::errors::ResultError;
use crate::heartbeat::HealCheckMode;
use crate::state::DEBUG_MODE;
use crate::state::GLOBAL_TIMER_STATE;
use crate::state::IS_HOOKED;
use crate::util::get_current_time;
use crate::util::kill_process;
use rglua::prelude::*;
use std::ffi::CStr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::thread;

#[lua_function]
pub fn ping_alive(_l: LuaState) -> Result<i32, ResultError> {
    GLOBAL_TIMER_STATE
        .get()
        .map(|(last_ping, pause)| {
            if DEBUG_MODE.load(Ordering::Relaxed) {
                let now = chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S");
                println!("[gHeartbeat DEBUG {}] Receive PING from game!", now);
            }

            last_ping.store(get_current_time(), Ordering::Relaxed);
            pause.store(false, Ordering::Relaxed);
            0
        })
        .ok_or(ResultError::TimerLegacyHealCheckOnly)
}

#[lua_function]
pub fn hook_a2s_heartbeat(l: LuaState) -> Result<i32, ResultError> {
    if IS_HOOKED.load(Ordering::Relaxed) {
        return Err(ResultError::HookAlreadyExists);
    }

    printgm!(l, "[gHeartbeat] Acquire a2s heartbeat!");
    let retry_count = luaL_checkinteger(l, 1) as u8;
    let port = luaL_checkinteger(l, 2) as u16;

    thread::spawn(move || {
        crate::heartbeat::bg_check_health(10, HealCheckMode::A2S { retry_count, port })
    });

    printgm!(l, "[gHeartbeat] Success to hook!");
    Ok(0)
}

#[lua_function]
pub fn hook_rcon_heartbeat(l: LuaState) -> Result<i32, ResultError> {
    if IS_HOOKED.load(Ordering::Relaxed) {
        return Err(ResultError::HookAlreadyExists);
    }

    printgm!(l, "[gHeartbeat] Acquire RCON heartbeat!");
    let retry_count = luaL_checkinteger(l, 1) as u8;
    let port = luaL_checkinteger(l, 2) as u16;

    let password_ptr = luaL_checkstring(l, 3);
    if password_ptr.is_null() {
        return Err(ResultError::UnclassifiedStatic("RCON password cannot be null"));
    }
    
    // 安全地将C字符串转换为Rust字符串
    let password = unsafe { CStr::from_ptr(password_ptr) }.to_string_lossy().into_owned();

    printgm!(
        l,
        "[gHeartbeat] RCON Params: Port {}, Retries {}, Password Length: {}",
        port,
        retry_count,
        password.len()
    );

    thread::spawn(move || {
        crate::heartbeat::bg_check_health(
            15,
            HealCheckMode::RCON {
                retry_count,
                port,
                password,
            },
        )
    });

    printgm!(l, "[gHeartbeat] Success to hook RCON heartbeat!");
    Ok(0)
}

#[lua_function]
pub fn hook_legacy_timer_heartbeat(l: LuaState) -> Result<i32, ResultError> {
    if IS_HOOKED.load(Ordering::Relaxed) {
        return Err(ResultError::HookAlreadyExists);
    }

    printgm!(l, "[gHeartbeat] Acquire legacy timer heartbeat!");
    let threshold = luaL_checkinteger(l, 1) as u64;
    let interval = luaL_checkinteger(l, 2) as u64;

    let last_timestamp = Arc::new(AtomicU64::new(get_current_time()));
    let pause = Arc::new(AtomicBool::new(true));

    GLOBAL_TIMER_STATE
        .set((last_timestamp.clone(), pause.clone()))
        .map_err(|_| ResultError::UnclassifiedStatic(
            "Global state set already..? it should be a bug!"
        ))?;

    thread::spawn(move || {
        crate::heartbeat::bg_check_health(
            interval,
            HealCheckMode::TimerLegacy {
                threshold,
                last_timestamp,
                pause,
            },
        )
    });

    printgm!(l, "[gHeartbeat] Success to hook!");
    Ok(0)
}

#[lua_function]
pub fn set_debug(l: LuaState) -> i32 {
    let debug_value = luaL_checkinteger(l, 1) == 1;
    DEBUG_MODE.store(debug_value, Ordering::Relaxed);
    0
}

#[lua_function]
pub fn manual_exit(l: LuaState) -> i32 {
    printgm!(l, "[gHeartbeat] Requesting exit...");
    kill_process();
}

#[lua_function]
pub fn pause(l: LuaState) -> Result<i32, ResultError> {
    GLOBAL_TIMER_STATE
        .get()
        .map(|(_, pause)| {
            let pause_val = luaL_checkinteger(l, 1) == 1;
            printgm!(l, "[gHeartbeat] Requesting pause...");
            pause.store(pause_val, Ordering::Relaxed);
            0
        })
        .ok_or(ResultError::TimerLegacyHealCheckOnly)
}
