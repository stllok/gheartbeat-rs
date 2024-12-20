local ENABLE_EMPTY_WORKSHOP_RESTART = false

require("gheartbeat")

-- Stop server if workshop didn't mount (require auto restart enabled)
if #engine.GetAddons() == 0 and ENABLE_EMPTY_WORKSHOP_RESTART then
  print("Restart trigger due to empty workshop!")
  gheartbeat.manual_exit()
end

-- 5 seconds interval, game port for a2s (just use the following code)
gheartbeat.hook_a2s_heartbeat(5, string.Split(game.GetIPAddress(), ":")[2])
