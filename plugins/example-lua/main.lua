save_default_config()

local config = load_config() or {}
local prefix = config.prefix or "[ExampleLua]"

function on_load()
  log("example plugin loaded")
end

function on_enable()
  log("example plugin enabled; try /hello or /hi")
end

function on_disable()
  log("example plugin disabled")
end

register_command("hello", function(sender, args)
  local target = args[1] or sender.name
  broadcast("Lua says hello to " .. target)
  return prefix .. " Hello, " .. target .. "!"
end)
