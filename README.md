# gHeartBeat-rs

[![Rust Library Build](https://github.com/stllok/gheartbeat-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/stllok/gheartbeat-rs/actions/workflows/rust.yml)

A simple health checking library for Garry's mod.

## Detection method

- [x] Time based detection
- [x] A2S query
- [ ] RCON query

## How to use

1. Download dll from latest **Actions** build with correct version.
2. Put dll into `garrysmod/lua/bin` folder.
3. Copy `sample.lua` into `garrysmod/lua/autorun/server` folder.
4. Adjust `sample.lua` to your needs.
5. Restart server.

## API

```lua
-- Interface to stop the server
manual_exit()
-- toggle debug print
set_debug(0 or 1)
-- hook gHeartBeat with a2s method
hook_a2s_heartbeat(interval: number, string.Split(game.GetIPAddress(), ":")[2])
-- hook gHeartBeat with legacy method
hook_legacy_timer_heartbeat(interval: number, timeout: number)
-- toggle gHeartBeat to do health check (legacy only)
pause(0 or 1)
-- tell gHeartBeat server is health, will set pause to false (legacy only)
ping_alive()
```
