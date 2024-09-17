use rglua::prelude::*;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        OnceLock,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use sysinfo::{Pid, Signal, System};

#[derive(Debug, thiserror::Error)]
enum ResultError {
    #[error("You already hook the function!")]
    HookAlreadyExists,
}

static LAST_HEARTBEAT: AtomicU64 = AtomicU64::new(0);
static IS_NO_PLAYER: AtomicBool = AtomicBool::new(true);
static IS_HOOKED: AtomicBool = AtomicBool::new(false);
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);
static PID: OnceLock<u32> = OnceLock::new();

#[inline(always)]
fn get_pid() -> u32 {
    *PID.get_or_init(std::process::id)
}

#[inline(always)]
fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .expect("SystemTime before UNIX EPOCH!!!")
}

#[inline(always)]
fn is_health(threshold: u64) -> bool {
    get_current_time() - LAST_HEARTBEAT.load(Ordering::Relaxed) <= threshold
}

#[inline(always)]
fn ping() {
    LAST_HEARTBEAT.store(get_current_time(), Ordering::Relaxed);
}

#[inline(always)]
fn kill_process() {
    if let Some(process) = System::new_all().process(Pid::from_u32(get_pid())) {
        println!("[gHeartbeat] Kill with Signal 9");
        process.kill_with(Signal::Kill);
    }
    println!("[gHeartbeat] Kill process exit");
    std::process::exit(0);
}

fn bg_check_health(threshold: u64, interval: u64) {
    println!("[gHeartbeat] Attached health check");
    println!("[gHeartbeat] Automatic stop server if {threshold} seconds no response");
    println!("[gHeartbeat] Check interval {interval} seconds");

    while IS_HOOKED.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_secs(interval));

        if DEBUG_MODE.load(Ordering::Relaxed) {
            println!(
                "[gHeartbeat DEBUG] current time: {}, recorded time: {}, duration: {}, empty server: {}",
                get_current_time(),
                LAST_HEARTBEAT.load(Ordering::Relaxed),
                get_current_time() - LAST_HEARTBEAT.load(Ordering::Relaxed),
                IS_NO_PLAYER.load(Ordering::Relaxed)
            );
        }

        if !is_health(threshold) {
            println!(
                "[gHeartbeat {}] Detected server no response within {threshold} seconds, stopping!!!",
                chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S")
            );
            kill_process();
        }

        // mark as ping when no player so it won't stop the server when no one inside
        if IS_NO_PLAYER.load(Ordering::Relaxed) {
            ping();
        }
    }
    println!("[gHeartbeat] Dropping background thread");
}

#[lua_function]
fn ping_alive(_l: LuaState) -> i32 {
    if DEBUG_MODE.load(Ordering::Relaxed) {
        println!(
            "[gHeartbeat DEBUG {}] Receive PING from game!",
            chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
    }

    ping();

    // as server responsed, server must contain player inside
    IS_NO_PLAYER.store(false, Ordering::Relaxed);
    0
}

#[lua_function]
fn server_empty_signal(_l: LuaState) -> i32 {
    println!("[gHeartbeat] Receive server empty signal from game!");

    IS_NO_PLAYER.store(true, Ordering::Relaxed);
    0
}

#[lua_function]
fn hook_heartbeat(l: LuaState) -> Result<i32, ResultError> {
    if IS_HOOKED.load(Ordering::Relaxed) {
        return Err(ResultError::HookAlreadyExists);
    }

    printgm!(l, "[gHeartbeat] Hooking to the modules!");
    IS_HOOKED.store(true, Ordering::Relaxed);
    LAST_HEARTBEAT.store(get_current_time(), Ordering::Relaxed);

    let (threshold, interval, debug_on) = (
        luaL_checkinteger(l, 1) as u64,
        luaL_checkinteger(l, 2) as u64,
        luaL_checkinteger(l, 3) as u8,
    );

    if debug_on == 1 {
        DEBUG_MODE.store(true, Ordering::Relaxed);
    }
    thread::spawn(move || bg_check_health(threshold, interval));
    printgm!(l, "[gHeartbeat] Success to hook!");

    Ok(0)
}

#[lua_function]
fn manual_exit(l: LuaState) -> i32 {
    printgm!(l, "[gHeartbeat] Requesting exit...");
    kill_process();
    0
}

// Note that since this is #[gmod_open] the name of the function does not matter
// This is the same for #[gmod_close]
#[gmod_open]
fn open(l: LuaState) -> i32 {
    // Print to the gmod console
    printgm!(l, "[gHeartbeat] Initializing");

    #[cfg(debug_assertions)]
    printgm!(l, "[gHeartbeat DEBUG] PID: {}", get_pid());

    // get PID at start
    get_pid();

    // Create a library to organize all of our functions to export to gmod.
    let lib = reg! [
        "manual_exit" => manual_exit,
        "hook_heartbeat" => hook_heartbeat,
        "server_empty_signal" => server_empty_signal,
        "ping_alive"=> ping_alive
    ];

    // Register our functions in ``_G.gheartbeat``
    // This WILL NOT overwrite _G.gheartbeat if it already exists (which it should..)
    luaL_register(l, cstr!("gheartbeat"), lib.as_ptr());
    1
}

#[gmod_close]
fn close(l: LuaState) -> i32 {
    IS_HOOKED.store(false, Ordering::Relaxed);
    printgm!(l, "[gHeartbeat] Releasing");
    0
}
