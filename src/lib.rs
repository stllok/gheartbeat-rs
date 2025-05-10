mod errors;
mod heartbeat;
mod lua_bindings;
mod state;
mod util;

use rglua::prelude::*;
use state::IS_HOOKED; // IS_HOOKED is used in open/close
use std::sync::atomic::Ordering; // Ordering is used for IS_HOOKED

// Note that since this is #[gmod_open] the name of the function does not matter
// This is the same for #[gmod_close]
#[gmod_open]
fn open(l: LuaState) -> i32 {
    // Print to the gmod console
    printgm!(l, "[gHeartbeat] Initializing");

    #[cfg(debug_assertions)]
    printgm!(l, "[gHeartbeat DEBUG] PID: {}", *state::PID);

    // Create a library to organize all of our functions to export to gmod.
    let lib = reg![
        "manual_exit" => lua_bindings::manual_exit,
        "hook_legacy_timer_heartbeat" => lua_bindings::hook_legacy_timer_heartbeat,
        "hook_a2s_heartbeat" => lua_bindings::hook_a2s_heartbeat,
        "hook_rcon_heartbeat" => lua_bindings::hook_rcon_heartbeat,
        "ping_alive" => lua_bindings::ping_alive,
        "pause" => lua_bindings::pause,
        "set_debug" => lua_bindings::set_debug
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
