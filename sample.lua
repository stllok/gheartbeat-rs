-- load modules
require("gheartbeat")

-- hook server to modules (In here we use 30 seconds for timeout and 5 seconds for interval)
gheartbeat.hook_heartbeat(30, 5)

-- add hook to notify modules server is empty (so it won't stop the server when no one due to the game features)
hook.Add("PlayerDisconnected", "gHeartbeatEmptyNotice", function(_) if player.GetCount() <= 1 then gheartbeat.server_empty_signal() end end)

-- timer to ping alive
timer.Create("gHeartbeat_ping", 2, 0, gheartbeat.ping_alive)
