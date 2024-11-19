# gHeartBeat-rs

[![Rust Library Build](https://github.com/stllok/gheartbeat-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/stllok/gheartbeat-rs/actions/workflows/rust.yml)

A simple health checking library for Garry's mod.

## Detection method

- [x] Time based detection
- [ ] A2S query
- [ ] RCON query

## How to use

1. Download dll from latest **Actions** build with correct version.
2. Put dll into `garrysmod/lua/bin` folder.
3. Copy `sample.lua` into `garrysmod/lua/autorun/server` folder.
4. Adjust `sample.lua` to your needs.
5. Restart server.
