use std::collections::HashMap;
use tracing::info;

/// Result of executing a command.
pub struct CommandResult {
    /// Message to send back to the command sender.
    pub response: Option<String>,
    /// Action to perform on the server.
    pub action: CommandAction,
}

/// Server-side action triggered by a command.
#[derive(Debug)]
pub enum CommandAction {
    /// No action, just a text response.
    None,
    /// Teleport the player to coordinates.
    Teleport { x: f32, y: f32, z: f32 },
    /// Change the player's gamemode.
    SetGamemode { mode: i32 },
    /// Broadcast a message to all players.
    Broadcast { message: String },
    /// Stop the server.
    Stop,
    /// Set world time.
    SetTime { time: i32 },
    /// Kill the player.
    Kill,
}

/// Definition of a command.
pub struct CommandDef {
    pub name: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    handler: fn(&[&str], &CommandContext) -> CommandResult,
}

/// Context passed to command handlers.
pub struct CommandContext {
    pub player_name: String,
}

/// Registry of all server commands.
pub struct CommandRegistry {
    commands: HashMap<String, CommandDef>,
}

impl CommandRegistry {
    /// Create a new registry with all built-in commands.
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register(&mut self, def: CommandDef) {
        self.commands.insert(format!("/{}", def.name), def);
    }

    /// Execute a command string (e.g., "/tp 10 20 30").
    pub fn execute(&self, command: &str, ctx: &CommandContext) -> CommandResult {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts.first().copied().unwrap_or("");

        if let Some(def) = self.commands.get(cmd) {
            info!("[CMD] {} executed: {}", ctx.player_name, command);
            (def.handler)(&parts, ctx)
        } else {
            CommandResult {
                response: Some(format!("Unknown command: {}. Type /help for help.", cmd)),
                action: CommandAction::None,
            }
        }
    }

    /// Get all command definitions (for AvailableCommands packet).
    pub fn all_commands(&self) -> Vec<(&str, &str)> {
        self.commands
            .values()
            .map(|def| (def.name, def.description))
            .collect()
    }

    fn register_builtins(&mut self) {
        self.register(CommandDef {
            name: "help",
            description: "Show available commands",
            usage: "/help",
            handler: |_, _| {
                CommandResult {
                    response: Some(
                        "Commands: /help, /list, /tp <x> <y> <z>, /gamemode <mode>, /say <msg>, /time set <val>, /kill, /stop, /ping, /seed"
                            .to_string(),
                    ),
                    action: CommandAction::None,
                }
            },
        });

        self.register(CommandDef {
            name: "list",
            description: "List online players",
            usage: "/list",
            handler: |_, _| CommandResult {
                response: Some("Use the player list (tab) to see online players.".to_string()),
                action: CommandAction::None,
            },
        });

        self.register(CommandDef {
            name: "tp",
            description: "Teleport to coordinates",
            usage: "/tp <x> <y> <z>",
            handler: |parts, ctx| {
                if parts.len() >= 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        info!("[CMD] {} teleported to {}, {}, {}", ctx.player_name, x, y, z);
                        return CommandResult {
                            response: Some(format!("Teleported to {}, {}, {}", x, y, z)),
                            action: CommandAction::Teleport { x, y, z },
                        };
                    }
                }
                CommandResult {
                    response: Some("Usage: /tp <x> <y> <z>".to_string()),
                    action: CommandAction::None,
                }
            },
        });

        self.register(CommandDef {
            name: "gamemode",
            description: "Change game mode",
            usage: "/gamemode <0-3|survival|creative|adventure|spectator>",
            handler: |parts, ctx| {
                if parts.len() < 2 {
                    return CommandResult {
                        response: Some("Usage: /gamemode <0-3|survival|creative|adventure|spectator>".to_string()),
                        action: CommandAction::None,
                    };
                }
                let mode = match parts[1] {
                    "0" | "survival" | "s" => 0,
                    "1" | "creative" | "c" => 1,
                    "2" | "adventure" | "a" => 2,
                    "3" | "spectator" | "sp" => 3,
                    _ => {
                        return CommandResult {
                            response: Some("Invalid gamemode. Use 0-3 or survival/creative/adventure/spectator.".to_string()),
                            action: CommandAction::None,
                        };
                    }
                };
                let name = match mode {
                    0 => "Survival",
                    1 => "Creative",
                    2 => "Adventure",
                    3 => "Spectator",
                    _ => "Unknown",
                };
                info!("[CMD] {} changed gamemode to {}", ctx.player_name, name);
                CommandResult {
                    response: Some(format!("Gamemode set to {}", name)),
                    action: CommandAction::SetGamemode { mode },
                }
            },
        });

        self.register(CommandDef {
            name: "say",
            description: "Broadcast a message",
            usage: "/say <message>",
            handler: |parts, ctx| {
                if parts.len() < 2 {
                    return CommandResult {
                        response: Some("Usage: /say <message>".to_string()),
                        action: CommandAction::None,
                    };
                }
                let message = parts[1..].join(" ");
                let broadcast = format!("[{}] {}", ctx.player_name, message);
                info!("[SAY] {}", broadcast);
                CommandResult {
                    response: Some(format!("Broadcast: {}", message)),
                    action: CommandAction::Broadcast { message: broadcast },
                }
            },
        });

        self.register(CommandDef {
            name: "time",
            description: "Set world time",
            usage: "/time set <value>",
            handler: |parts, ctx| {
                if parts.len() >= 3 && parts[1] == "set" {
                    if let Ok(time) = parts[2].parse::<i32>() {
                        info!("[CMD] {} set time to {}", ctx.player_name, time);
                        return CommandResult {
                            response: Some(format!("Time set to {}", time)),
                            action: CommandAction::SetTime { time },
                        };
                    }
                }
                CommandResult {
                    response: Some("Usage: /time set <value>".to_string()),
                    action: CommandAction::None,
                }
            },
        });

        self.register(CommandDef {
            name: "kill",
            description: "Kill yourself",
            usage: "/kill",
            handler: |_, ctx| {
                info!("[CMD] {} killed themselves", ctx.player_name);
                CommandResult {
                    response: Some("You died!".to_string()),
                    action: CommandAction::Kill,
                }
            },
        });

        self.register(CommandDef {
            name: "stop",
            description: "Stop the server",
            usage: "/stop",
            handler: |_, ctx| {
                info!("[CMD] Server stop requested by {}", ctx.player_name);
                CommandResult {
                    response: Some("Server shutting down...".to_string()),
                    action: CommandAction::Stop,
                }
            },
        });

        self.register(CommandDef {
            name: "seed",
            description: "Display world seed",
            usage: "/seed",
            handler: |_, _| CommandResult {
                response: Some("Seed: 0 (flat world)".to_string()),
                action: CommandAction::None,
            },
        });

        self.register(CommandDef {
            name: "ping",
            description: "Check server connection",
            usage: "/ping",
            handler: |_, _| CommandResult {
                response: Some("Pong!".to_string()),
                action: CommandAction::None,
            },
        });

        self.register(CommandDef {
            name: "difficulty",
            description: "Set game difficulty",
            usage: "/difficulty <0-3|peaceful|easy|normal|hard>",
            handler: |parts, ctx| {
                if parts.len() < 2 {
                    return CommandResult {
                        response: Some("Usage: /difficulty <0-3|peaceful|easy|normal|hard>".to_string()),
                        action: CommandAction::None,
                    };
                }
                let name = match parts[1] {
                    "0" | "peaceful" | "p" => "Peaceful",
                    "1" | "easy" | "e" => "Easy",
                    "2" | "normal" | "n" => "Normal",
                    "3" | "hard" | "h" => "Hard",
                    _ => {
                        return CommandResult {
                            response: Some("Invalid difficulty.".to_string()),
                            action: CommandAction::None,
                        };
                    }
                };
                info!("[CMD] {} set difficulty to {}", ctx.player_name, name);
                CommandResult {
                    response: Some(format!("Difficulty set to {}", name)),
                    action: CommandAction::None,
                }
            },
        });
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
