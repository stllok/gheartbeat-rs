local ENABLE_EMPTY_WORKSHOP_RESTART = false

-- load modules
require("gheartbeat")

-- hook server to modules (In here we use 30 seconds for timeout and 5 seconds for interval)
gheartbeat.hook_legacy_timer_heartbeat(30, 5)

-- add hook to notify modules server is empty (so it won't stop the server when no one due to the game features)
hook.Add("PlayerDisconnected", "gHeartbeatEmptyNotice", function(_) gheartbeat.pause(math.min(1, player.GetCount())) end)

-- timer to ping alive
timer.Create("gHeartbeat_ping", 2, 0, gheartbeat.ping_alive)

-- Stop server if workshop didn't mount (require auto restart enabled)
if #engine.GetAddons() == 0 and ENABLE_EMPTY_WORKSHOP_RESTART then
  print("Restart trigger due to empty workshop!")
  gheartbeat.manual_exit()
end
