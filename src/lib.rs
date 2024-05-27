use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rglua::prelude::*;

static LAST_HEARTBEAT: AtomicU64 = AtomicU64::new(0);
static IS_HOOKED: AtomicBool = AtomicBool::new(false);

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

fn bg_check_health(threshold: u64, interval: u64) {
    println!("[gHeartbeat] Attached health check");
    println!("[gHeartbeat] Automatic stop server if {threshold} seconds no response");
    println!("[gHeartbeat] Check interval {interval} seconds");

    while IS_HOOKED.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_secs(interval));

        #[cfg(debug_assertions)]
        println!(
            "[gHeartbeat DEBUG] current time: {}, recorded time: {}",
            get_current_time(),
            LAST_HEARTBEAT.load(Ordering::Relaxed)
        );

        if !is_health(threshold) {
            println!(
                "[gHeartbeat] Detected server no response within {threshold} seconds, stopping!!!"
            );
            std::process::exit(0);
        }
    }
}

#[lua_function]
fn ping_alive(_l: LuaState) -> i32 {
    #[cfg(debug_assertions)]
    println!("[gHeartbeat DEBUG] Receive PING from game!",);

    LAST_HEARTBEAT.store(get_current_time(), Ordering::Relaxed);
    0
}

#[lua_function]
fn hook_heartbeat(l: LuaState) -> i32 {
    if IS_HOOKED.load(Ordering::Relaxed) {
        printgm!(l, "[gHeartbeat] You already hook the function!");
        return 0;
    }
    printgm!(l, "[gHeartbeat] Hooking to the modules!");
    IS_HOOKED.store(true, Ordering::Relaxed);
    LAST_HEARTBEAT.store(get_current_time(), Ordering::Relaxed);

    let threshold = luaL_checkinteger(l, 1) as u64;
    let interval = luaL_checkinteger(l, 2) as u64;
    thread::spawn(move || bg_check_health(threshold, interval));
    printgm!(l, "[gHeartbeat] Success to hook!");

    0
}

#[lua_function]
fn exit(l: LuaState) -> i32 {
    printgm!(l, "[gHeartbeat] Requesting exit...");
    std::process::exit(0);
    0
}

// Note that since this is #[gmod_open] the name of the function does not matter
// This is the same for #[gmod_close]
#[gmod_open]
fn open(l: LuaState) -> i32 {
    // Print to the gmod console
    printgm!(l, "[gHeartbeat] Initializing");

    // Create a library to organize all of our functions to export to gmod.
    let lib = reg! [
        "exit" => exit,
        "ping_alive"=> ping_alive,
        "hook_heartbeat" => hook_heartbeat
    ];

    // Register our functions in ``_G.math``
    // This WILL NOT overwrite _G.math if it already exists (which it should..)
    luaL_register(l, cstr!("gheartbeat"), lib.as_ptr());
    1
}

#[gmod_close]
fn close(l: LuaState) -> i32 {
    IS_HOOKED.store(false, Ordering::Relaxed);
    printgm!(l, "[gHeartbeat] Releasing");
    0
}
