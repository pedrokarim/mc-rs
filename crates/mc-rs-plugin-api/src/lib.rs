//! Plugin API: traits, events, and server API for plugin authors.
//!
//! This crate defines the interface that plugin runtimes (Lua, WASM) implement.
//! It has no dependency on mc-rs-server or mc-rs-proto.

// ─── Types ───────────────────────────────────────────────────────────────────

/// Information about an online player, passed to plugins in events.
#[derive(Debug, Clone)]
pub struct PluginPlayer {
    pub name: String,
    pub uuid: String,
    pub runtime_id: u64,
    pub position: (f32, f32, f32),
    pub gamemode: i32,
    pub health: f32,
}

/// Block position for plugin events (decoupled from mc-rs-proto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginBlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Cause of damage for PlayerDamage events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageCause {
    Attack,
    Fall,
    Drowning,
    Lava,
    Fire,
    Suffocation,
    Starvation,
    Void,
    Other,
}

/// Log level for plugin logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// Result of dispatching an event to a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// Continue normal handling.
    Continue,
    /// Event was cancelled by this plugin.
    Cancelled,
}

// ─── Events ──────────────────────────────────────────────────────────────────

/// All events that plugins can listen to.
#[derive(Debug, Clone)]
pub enum PluginEvent {
    // --- Player events (8) ---
    PlayerJoin {
        player: PluginPlayer,
    },
    PlayerQuit {
        player: PluginPlayer,
    },
    PlayerChat {
        player: PluginPlayer,
        message: String,
    },
    PlayerCommand {
        player: PluginPlayer,
        command: String,
        args: Vec<String>,
    },
    PlayerMove {
        player: PluginPlayer,
        from: (f32, f32, f32),
        to: (f32, f32, f32),
    },
    PlayerDeath {
        player: PluginPlayer,
        message: String,
    },
    PlayerDamage {
        player: PluginPlayer,
        damage: f32,
        cause: DamageCause,
    },
    PlayerRespawn {
        player: PluginPlayer,
    },

    // --- Block events (2) ---
    BlockBreak {
        player: PluginPlayer,
        position: PluginBlockPos,
        block_id: u32,
    },
    BlockPlace {
        player: PluginPlayer,
        position: PluginBlockPos,
        block_id: u32,
    },

    // --- Entity events (3) ---
    MobSpawn {
        mob_type: String,
        runtime_id: u64,
        position: (f32, f32, f32),
    },
    MobDeath {
        mob_type: String,
        runtime_id: u64,
        killer_runtime_id: Option<u64>,
    },
    EntityDamage {
        runtime_id: u64,
        damage: f32,
        attacker_runtime_id: Option<u64>,
    },

    // --- World events (2) ---
    WeatherChange {
        raining: bool,
        thundering: bool,
    },
    TimeChange {
        new_time: i64,
    },

    // --- Server events (2) ---
    ServerStarted,
    ServerStopping,
}

impl PluginEvent {
    /// Whether this event type can be cancelled by a plugin.
    pub fn is_cancellable(&self) -> bool {
        matches!(
            self,
            PluginEvent::PlayerChat { .. }
                | PluginEvent::PlayerCommand { .. }
                | PluginEvent::PlayerMove { .. }
                | PluginEvent::PlayerDamage { .. }
                | PluginEvent::BlockBreak { .. }
                | PluginEvent::BlockPlace { .. }
                | PluginEvent::MobSpawn { .. }
                | PluginEvent::EntityDamage { .. }
                | PluginEvent::WeatherChange { .. }
                | PluginEvent::TimeChange { .. }
        )
    }
}

// ─── Plugin trait ────────────────────────────────────────────────────────────

/// Metadata about a plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

/// The Plugin trait: implemented by Lua/WASM runtimes or built-in plugins.
pub trait Plugin: Send {
    /// Return plugin metadata.
    fn info(&self) -> PluginInfo;

    /// Called when the plugin is loaded. Use `api` to register commands, schedule tasks.
    fn on_enable(&mut self, api: &mut dyn ServerApi);

    /// Called when the plugin is unloaded.
    fn on_disable(&mut self) {}

    /// Called for every dispatched event. Return `Cancelled` to cancel cancellable events.
    fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult {
        let _ = (event, api);
        EventResult::Continue
    }

    /// Called when a scheduled task fires.
    fn on_task(&mut self, task_id: u32, api: &mut dyn ServerApi) {
        let _ = (task_id, api);
    }

    /// Called when a plugin-registered command is executed. Return a response message.
    fn on_command(
        &mut self,
        command: &str,
        args: &[String],
        sender: &str,
        api: &mut dyn ServerApi,
    ) -> Option<String> {
        let _ = (command, args, sender, api);
        None
    }

    /// Return a default config as JSON. If `Some`, the plugin gets a config file.
    fn default_config(&self) -> Option<serde_json::Value> {
        None
    }

    /// Called with the loaded config (from `plugins/<name>/config.json`).
    fn load_config(&mut self, _config: serde_json::Value) {}
}

// ─── Server API ──────────────────────────────────────────────────────────────

/// Safe read/write access to server state, passed to plugins during callbacks.
///
/// Read methods return data immediately. Write methods are deferred (applied
/// after the plugin callback returns).
pub trait ServerApi {
    // --- Players ---
    fn online_players(&self) -> Vec<PluginPlayer>;
    fn get_player(&self, name: &str) -> Option<PluginPlayer>;
    fn send_message(&mut self, player_name: &str, message: &str);
    fn broadcast_message(&mut self, message: &str);
    fn kick_player(&mut self, player_name: &str, reason: &str);
    fn set_player_health(&mut self, player_name: &str, health: f32);
    fn set_player_food(&mut self, player_name: &str, food: i32);
    fn teleport_player(&mut self, player_name: &str, x: f32, y: f32, z: f32);

    // --- World ---
    fn get_time(&self) -> i64;
    fn set_time(&mut self, time: i64);
    fn is_raining(&self) -> bool;

    // --- Entities ---
    fn spawn_mob(&mut self, mob_type: &str, x: f32, y: f32, z: f32);
    fn remove_mob(&mut self, runtime_id: u64);

    // --- Server ---
    fn get_tick(&self) -> u64;
    fn log(&self, level: LogLevel, message: &str);

    // --- Scheduler ---
    fn schedule_delayed(&mut self, plugin_name: &str, delay_ticks: u64, task_id: u32);
    fn schedule_repeating(
        &mut self,
        plugin_name: &str,
        delay_ticks: u64,
        interval_ticks: u64,
        task_id: u32,
    );
    fn cancel_task(&mut self, plugin_name: &str, task_id: u32);

    // --- Commands ---
    fn register_command(&mut self, name: &str, description: &str, plugin_name: &str);
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_player() -> PluginPlayer {
        PluginPlayer {
            name: "TestPlayer".into(),
            uuid: "00000000-0000-0000-0000-000000000001".into(),
            runtime_id: 1,
            position: (0.5, 65.62, 0.5),
            gamemode: 0,
            health: 20.0,
        }
    }

    // Minimal ServerApi implementation for testing.
    struct MockApi {
        messages: Vec<(String, String)>,
        broadcasts: Vec<String>,
        commands: Vec<(String, String)>,
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                messages: Vec::new(),
                broadcasts: Vec::new(),
                commands: Vec::new(),
            }
        }
    }

    impl ServerApi for MockApi {
        fn online_players(&self) -> Vec<PluginPlayer> {
            vec![test_player()]
        }
        fn get_player(&self, name: &str) -> Option<PluginPlayer> {
            if name == "TestPlayer" {
                Some(test_player())
            } else {
                None
            }
        }
        fn send_message(&mut self, player_name: &str, message: &str) {
            self.messages
                .push((player_name.to_string(), message.to_string()));
        }
        fn broadcast_message(&mut self, message: &str) {
            self.broadcasts.push(message.to_string());
        }
        fn kick_player(&mut self, _player_name: &str, _reason: &str) {}
        fn set_player_health(&mut self, _player_name: &str, _health: f32) {}
        fn set_player_food(&mut self, _player_name: &str, _food: i32) {}
        fn teleport_player(&mut self, _player_name: &str, _x: f32, _y: f32, _z: f32) {}
        fn get_time(&self) -> i64 {
            6000
        }
        fn set_time(&mut self, _time: i64) {}
        fn is_raining(&self) -> bool {
            false
        }
        fn spawn_mob(&mut self, _mob_type: &str, _x: f32, _y: f32, _z: f32) {}
        fn remove_mob(&mut self, _runtime_id: u64) {}
        fn get_tick(&self) -> u64 {
            100
        }
        fn log(&self, _level: LogLevel, _message: &str) {}
        fn schedule_delayed(&mut self, _plugin_name: &str, _delay_ticks: u64, _task_id: u32) {}
        fn schedule_repeating(
            &mut self,
            _plugin_name: &str,
            _delay_ticks: u64,
            _interval_ticks: u64,
            _task_id: u32,
        ) {
        }
        fn cancel_task(&mut self, _plugin_name: &str, _task_id: u32) {}
        fn register_command(&mut self, name: &str, description: &str, _plugin_name: &str) {
            self.commands
                .push((name.to_string(), description.to_string()));
        }
    }

    // A simple test plugin.
    struct HelloPlugin {
        greet_on_join: bool,
    }

    impl HelloPlugin {
        fn new() -> Self {
            Self {
                greet_on_join: true,
            }
        }
    }

    impl Plugin for HelloPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                name: "HelloPlugin".into(),
                version: "1.0.0".into(),
                description: "Greets players on join".into(),
                author: "Test".into(),
            }
        }

        fn on_enable(&mut self, api: &mut dyn ServerApi) {
            api.register_command("greet", "Greet a player", "HelloPlugin");
            api.log(LogLevel::Info, "HelloPlugin enabled!");
        }

        fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult {
            match event {
                PluginEvent::PlayerJoin { player } if self.greet_on_join => {
                    api.send_message(&player.name, &format!("Welcome, {}!", player.name));
                    EventResult::Continue
                }
                PluginEvent::PlayerChat { message, .. } if message.contains("bad") => {
                    EventResult::Cancelled
                }
                _ => EventResult::Continue,
            }
        }

        fn on_command(
            &mut self,
            command: &str,
            args: &[String],
            sender: &str,
            api: &mut dyn ServerApi,
        ) -> Option<String> {
            if command == "greet" {
                let target = args.first().map(|s| s.as_str()).unwrap_or(sender);
                api.send_message(target, "Hello from plugin!");
                Some(format!("Greeted {target}"))
            } else {
                None
            }
        }

        fn default_config(&self) -> Option<serde_json::Value> {
            Some(serde_json::json!({ "greet_on_join": true }))
        }

        fn load_config(&mut self, config: serde_json::Value) {
            if let Some(v) = config.get("greet_on_join").and_then(|v| v.as_bool()) {
                self.greet_on_join = v;
            }
        }
    }

    #[test]
    fn plugin_info() {
        let plugin = HelloPlugin::new();
        let info = plugin.info();
        assert_eq!(info.name, "HelloPlugin");
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn plugin_on_enable_registers_command() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        plugin.on_enable(&mut api);
        assert_eq!(api.commands.len(), 1);
        assert_eq!(api.commands[0].0, "greet");
    }

    #[test]
    fn plugin_greets_on_join() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let event = PluginEvent::PlayerJoin {
            player: test_player(),
        };
        let result = plugin.on_event(&event, &mut api);
        assert_eq!(result, EventResult::Continue);
        assert_eq!(api.messages.len(), 1);
        assert_eq!(api.messages[0].0, "TestPlayer");
        assert!(api.messages[0].1.contains("Welcome"));
    }

    #[test]
    fn plugin_cancels_bad_chat() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let event = PluginEvent::PlayerChat {
            player: test_player(),
            message: "this is bad word".into(),
        };
        let result = plugin.on_event(&event, &mut api);
        assert_eq!(result, EventResult::Cancelled);
    }

    #[test]
    fn plugin_allows_good_chat() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let event = PluginEvent::PlayerChat {
            player: test_player(),
            message: "hello everyone".into(),
        };
        let result = plugin.on_event(&event, &mut api);
        assert_eq!(result, EventResult::Continue);
    }

    #[test]
    fn plugin_command_greet() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let response = plugin.on_command("greet", &["Alice".into()], "Bob", &mut api);
        assert_eq!(response, Some("Greeted Alice".into()));
        assert_eq!(api.messages.len(), 1);
        assert_eq!(api.messages[0].0, "Alice");
    }

    #[test]
    fn plugin_config_roundtrip() {
        let mut plugin = HelloPlugin::new();
        assert!(plugin.greet_on_join);

        let config = serde_json::json!({ "greet_on_join": false });
        plugin.load_config(config);
        assert!(!plugin.greet_on_join);

        let default = plugin.default_config().unwrap();
        assert_eq!(default["greet_on_join"], true);
    }

    #[test]
    fn event_cancellable_flags() {
        assert!(PluginEvent::PlayerChat {
            player: test_player(),
            message: String::new()
        }
        .is_cancellable());
        assert!(PluginEvent::BlockBreak {
            player: test_player(),
            position: PluginBlockPos { x: 0, y: 0, z: 0 },
            block_id: 0,
        }
        .is_cancellable());
        assert!(PluginEvent::PlayerDamage {
            player: test_player(),
            damage: 0.0,
            cause: DamageCause::Attack,
        }
        .is_cancellable());
        assert!(!PluginEvent::PlayerJoin {
            player: test_player()
        }
        .is_cancellable());
        assert!(!PluginEvent::PlayerQuit {
            player: test_player()
        }
        .is_cancellable());
        assert!(!PluginEvent::ServerStarted.is_cancellable());
        assert!(!PluginEvent::ServerStopping.is_cancellable());
        assert!(!PluginEvent::MobDeath {
            mob_type: String::new(),
            runtime_id: 0,
            killer_runtime_id: None,
        }
        .is_cancellable());
    }
}
