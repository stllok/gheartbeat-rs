use a2s::A2SClient;
use local_ip_address::local_ip;
use rglua::prelude::*;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, LazyLock, OnceLock,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use sysinfo::{Pid, Signal, System};

macro_rules! print_with_prefix {
    ($msg:expr) => {
        println!(
            "[gheartbeat-rs {}] {}",
            chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S"),
            $msg
        );
    };
}

#[derive(Debug)]
enum HealCheckMode {
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
    },
}
impl HealCheckMode {
    fn is_health(&self) -> bool {
        match self {
            HealCheckMode::TimerLegacy {
                threshold,
                last_timestamp,
                pause,
            } => {
                (get_current_time() - last_timestamp.load(Ordering::Relaxed) <= *threshold)
                    && !pause.load(Ordering::Relaxed)
            }
            HealCheckMode::A2S { retry_count, port } => {
                // Define timeout (as since localhost we can set the timeout to very fast)
                for i in 0..*retry_count {
                    if A2S_CLIENT.info(format!("{}:{port}", *LOCAL_IP)).is_ok() {
                        return true;
                    }
                    print_with_prefix!("Attempt ping [{i}/{retry_count}]");
                }

                false
            }
            HealCheckMode::RCON { retry_count, port } => todo!(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ResultError {
    #[error("You already hook the function!")]
    HookAlreadyExists,
    #[error("This function only available for timerLegacy health check!")]
    TimerLegacyHealCheckOnly,
    #[error("{0}")]
    UnclassifiedStatic(&'static str),
    #[error("{0}")]
    Unclassified(String),
}

// Global state
static IS_HOOKED: AtomicBool = AtomicBool::new(false);
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);
static PID: LazyLock<u32> = LazyLock::new(std::process::id);
// Global Timer based state
static GLOBAL_TIMER_STATE: OnceLock<(Arc<AtomicU64>, Arc<AtomicBool>)> = OnceLock::new();
// Global A2S state
static A2S_CLIENT: LazyLock<A2SClient> = LazyLock::new(|| A2SClient::new().unwrap());
static LOCAL_IP: LazyLock<String> = LazyLock::new(|| local_ip().unwrap().to_string());

#[inline(always)]
fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .expect("SystemTime before UNIX EPOCH!!!")
}

#[inline(always)]
fn kill_process() {
    if let Some(process) = System::new_all().process(Pid::from_u32(*PID)) {
        print_with_prefix!("SIGTERM request");
        process.kill_with(Signal::Term);
    }
    print_with_prefix!("Process exit");
    std::process::exit(0);
}

fn bg_check_health(interval: u64, healthcheck: HealCheckMode) {
    print_with_prefix!("Attached health check");
    print_with_prefix!("Check interval {interval} seconds");
    IS_HOOKED.store(true, Ordering::Relaxed);

    loop {
        thread::sleep(Duration::from_secs(interval));

        if DEBUG_MODE.load(Ordering::Relaxed) {
            print_with_prefix!("{healthcheck:?}");
        }

        if healthcheck.is_health() && IS_HOOKED.load(Ordering::Relaxed) {
            continue;
        } else {
            // Either server health check failed or HOOK released
            break;
        }
    }

    if IS_HOOKED.load(Ordering::Relaxed) {
        print_with_prefix!("Server health check failed");
        kill_process();
    } else {
        print_with_prefix!("Receive exit request");
    }
}

#[lua_function]
fn ping_alive(_l: LuaState) -> Result<i32, ResultError> {
    match GLOBAL_TIMER_STATE.get() {
        Some((last_ping, _)) => {
            if DEBUG_MODE.load(Ordering::Relaxed) {
                print_with_prefix!("Receive PING from game!");
            }

            last_ping.store(get_current_time(), Ordering::Relaxed);
            Ok(0)
        }
        None => Err(ResultError::TimerLegacyHealCheckOnly),
    }
}

#[lua_function]
fn hook_a2s_heartbeat(l: LuaState) -> Result<i32, ResultError> {
    if IS_HOOKED.load(Ordering::Relaxed) {
        return Err(ResultError::HookAlreadyExists);
    }

    printgm!(l, "[gHeartbeat] Acquire a2s heartbeat!");
    let (retry_count, port, debug_on) = (
        luaL_checkinteger(l, 1) as u8,
        luaL_checkinteger(l, 2) as u16,
        luaL_checkinteger(l, 3) as u8,
    );

    // set state
    DEBUG_MODE.store(debug_on.eq(&1), Ordering::Relaxed);

    // spawn bg
    thread::spawn(move || bg_check_health(10, HealCheckMode::A2S { retry_count, port }));

    printgm!(l, "[gHeartbeat] Success to hook!");

    Ok(0)
}

#[lua_function]
fn hook_legacy_timer_heartbeat(l: LuaState) -> Result<i32, ResultError> {
    if IS_HOOKED.load(Ordering::Relaxed) {
        return Err(ResultError::HookAlreadyExists);
    }

    printgm!(l, "[gHeartbeat] Acquire legacy timer heartbeat!");

    let (threshold, interval, debug_on) = (
        luaL_checkinteger(l, 1) as u64,
        luaL_checkinteger(l, 2) as u64,
        luaL_checkinteger(l, 3) as u8,
    );

    let last_timestamp = Arc::new(AtomicU64::new(get_current_time()));
    let pause = Arc::new(AtomicBool::new(false));

    // set state
    GLOBAL_TIMER_STATE
        .set((last_timestamp.clone(), pause.clone()))
        .map_err(|_| {
            ResultError::UnclassifiedStatic("Global state set already..? it should be a bug!")
        })?;
    DEBUG_MODE.store(debug_on.eq(&1), Ordering::Relaxed);

    // spawn bg
    thread::spawn(move || {
        bg_check_health(
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
fn manual_exit(l: LuaState) -> i32 {
    printgm!(l, "[gHeartbeat] Requesting exit...");
    kill_process();
    0
}

#[lua_function]
fn pause(l: LuaState) -> Result<i32, ResultError> {
    match GLOBAL_TIMER_STATE.get() {
        Some((_, pause)) => {
            let pause_val = luaL_checkinteger(l, 1) as u8;

            printgm!(l, "[gHeartbeat] Requesting pause...");
            pause.store(pause_val == 1, Ordering::Relaxed);

            Ok(0)
        }
        None => Err(ResultError::TimerLegacyHealCheckOnly),
    }
}

// Note that since this is #[gmod_open] the name of the function does not matter
// This is the same for #[gmod_close]
#[gmod_open]
fn open(l: LuaState) -> i32 {
    // Print to the gmod console
    printgm!(l, "[gHeartbeat] Initializing");

    #[cfg(debug_assertions)]
    printgm!(l, "[gHeartbeat DEBUG] PID: {}", *PID);

    // Create a library to organize all of our functions to export to gmod.
    let lib = reg! [
        "manual_exit" => manual_exit,
        "hook_legacy_timer_heartbeat" => hook_legacy_timer_heartbeat,
        "hook_a2s_heartbeat" => hook_a2s_heartbeat,
        "ping_alive"=> ping_alive,
        "pause" => pause
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
