//! Plugin manager: loads, enables, and dispatches events to plugins.

use std::collections::HashMap;

use mc_rs_plugin_api::{EventResult, LogLevel, Plugin, PluginEvent, PluginPlayer, ServerApi};
use tracing::{debug, error, info, warn};

// ─── Types ───────────────────────────────────────────────────────────────────

/// A scheduled task owned by a plugin.
pub struct ScheduledTask {
    pub plugin_name: String,
    pub task_id: u32,
    pub remaining_ticks: u64,
    /// `None` = one-shot, `Some(n)` = repeating every `n` ticks.
    pub interval: Option<u64>,
}

/// Server state snapshot for plugin API reads (built before dispatch).
pub struct ServerSnapshot {
    pub players: Vec<PluginPlayer>,
    pub world_time: i64,
    pub current_tick: u64,
    pub is_raining: bool,
}

/// Deferred side-effect requested by a plugin during a callback.
#[allow(dead_code)]
pub enum PendingAction {
    SendMessage {
        player_name: String,
        message: String,
    },
    BroadcastMessage {
        message: String,
    },
    KickPlayer {
        player_name: String,
        reason: String,
    },
    SetPlayerHealth {
        player_name: String,
        health: f32,
    },
    SetPlayerFood {
        player_name: String,
        food: i32,
    },
    TeleportPlayer {
        player_name: String,
        x: f32,
        y: f32,
        z: f32,
    },
    SetTime {
        time: i64,
    },
    SpawnMob {
        mob_type: String,
        x: f32,
        y: f32,
        z: f32,
    },
    RemoveMob {
        runtime_id: u64,
    },
    RegisterCommand {
        name: String,
        description: String,
        plugin_name: String,
    },
    ScheduleTask {
        task: ScheduledTask,
    },
    CancelTask {
        plugin_name: String,
        task_id: u32,
    },
    Log {
        level: LogLevel,
        message: String,
    },
    ShowForm {
        player_name: String,
        form_id: u32,
        form_data: String,
        form_type: String,
    },
}

// ─── ServerApiImpl ───────────────────────────────────────────────────────────

/// Implements `ServerApi` using a snapshot for reads and accumulating PendingActions for writes.
struct ServerApiImpl<'a> {
    snapshot: &'a ServerSnapshot,
    actions: Vec<PendingAction>,
}

impl<'a> ServerApiImpl<'a> {
    fn new(snapshot: &'a ServerSnapshot) -> Self {
        Self {
            snapshot,
            actions: Vec::new(),
        }
    }

    fn take_actions(self) -> Vec<PendingAction> {
        self.actions
    }
}

impl ServerApi for ServerApiImpl<'_> {
    fn online_players(&self) -> Vec<PluginPlayer> {
        self.snapshot.players.clone()
    }

    fn get_player(&self, name: &str) -> Option<PluginPlayer> {
        self.snapshot
            .players
            .iter()
            .find(|p| p.name == name)
            .cloned()
    }

    fn send_message(&mut self, player_name: &str, message: &str) {
        self.actions.push(PendingAction::SendMessage {
            player_name: player_name.to_string(),
            message: message.to_string(),
        });
    }

    fn broadcast_message(&mut self, message: &str) {
        self.actions.push(PendingAction::BroadcastMessage {
            message: message.to_string(),
        });
    }

    fn kick_player(&mut self, player_name: &str, reason: &str) {
        self.actions.push(PendingAction::KickPlayer {
            player_name: player_name.to_string(),
            reason: reason.to_string(),
        });
    }

    fn set_player_health(&mut self, player_name: &str, health: f32) {
        self.actions.push(PendingAction::SetPlayerHealth {
            player_name: player_name.to_string(),
            health,
        });
    }

    fn set_player_food(&mut self, player_name: &str, food: i32) {
        self.actions.push(PendingAction::SetPlayerFood {
            player_name: player_name.to_string(),
            food,
        });
    }

    fn teleport_player(&mut self, player_name: &str, x: f32, y: f32, z: f32) {
        self.actions.push(PendingAction::TeleportPlayer {
            player_name: player_name.to_string(),
            x,
            y,
            z,
        });
    }

    fn get_time(&self) -> i64 {
        self.snapshot.world_time
    }

    fn set_time(&mut self, time: i64) {
        self.actions.push(PendingAction::SetTime { time });
    }

    fn is_raining(&self) -> bool {
        self.snapshot.is_raining
    }

    fn spawn_mob(&mut self, mob_type: &str, x: f32, y: f32, z: f32) {
        self.actions.push(PendingAction::SpawnMob {
            mob_type: mob_type.to_string(),
            x,
            y,
            z,
        });
    }

    fn remove_mob(&mut self, runtime_id: u64) {
        self.actions.push(PendingAction::RemoveMob { runtime_id });
    }

    fn get_tick(&self) -> u64 {
        self.snapshot.current_tick
    }

    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Info => info!("[plugin] {message}"),
            LogLevel::Warn => warn!("[plugin] {message}"),
            LogLevel::Error => error!("[plugin] {message}"),
            LogLevel::Debug => debug!("[plugin] {message}"),
        }
    }

    fn schedule_delayed(&mut self, plugin_name: &str, delay_ticks: u64, task_id: u32) {
        self.actions.push(PendingAction::ScheduleTask {
            task: ScheduledTask {
                plugin_name: plugin_name.to_string(),
                task_id,
                remaining_ticks: delay_ticks,
                interval: None,
            },
        });
    }

    fn schedule_repeating(
        &mut self,
        plugin_name: &str,
        delay_ticks: u64,
        interval_ticks: u64,
        task_id: u32,
    ) {
        self.actions.push(PendingAction::ScheduleTask {
            task: ScheduledTask {
                plugin_name: plugin_name.to_string(),
                task_id,
                remaining_ticks: delay_ticks,
                interval: Some(interval_ticks),
            },
        });
    }

    fn cancel_task(&mut self, plugin_name: &str, task_id: u32) {
        self.actions.push(PendingAction::CancelTask {
            plugin_name: plugin_name.to_string(),
            task_id,
        });
    }

    fn register_command(&mut self, name: &str, description: &str, plugin_name: &str) {
        self.actions.push(PendingAction::RegisterCommand {
            name: name.to_string(),
            description: description.to_string(),
            plugin_name: plugin_name.to_string(),
        });
    }

    fn show_simple_form(&mut self, player_name: &str, form_id: u32, json: &str) {
        self.actions.push(PendingAction::ShowForm {
            player_name: player_name.to_string(),
            form_id,
            form_data: json.to_string(),
            form_type: "simple".to_string(),
        });
    }

    fn show_modal_form(&mut self, player_name: &str, form_id: u32, json: &str) {
        self.actions.push(PendingAction::ShowForm {
            player_name: player_name.to_string(),
            form_id,
            form_data: json.to_string(),
            form_type: "modal".to_string(),
        });
    }

    fn show_custom_form(&mut self, player_name: &str, form_id: u32, json: &str) {
        self.actions.push(PendingAction::ShowForm {
            player_name: player_name.to_string(),
            form_id,
            form_data: json.to_string(),
            form_type: "custom".to_string(),
        });
    }
}

// ─── PluginManager ───────────────────────────────────────────────────────────

/// Manages all loaded plugins, their scheduled tasks, and command registrations.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    tasks: Vec<ScheduledTask>,
    /// Commands registered by plugins: command_name → plugin_name.
    pub plugin_commands: HashMap<String, String>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            tasks: Vec::new(),
            plugin_commands: HashMap::new(),
        }
    }

    /// Register a plugin (call before enable_all).
    #[allow(dead_code)]
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        info!("Registered plugin: {}", plugin.info().name);
        self.plugins.push(plugin);
    }

    /// Enable all registered plugins.
    #[allow(dead_code)]
    pub fn enable_all(&mut self, snapshot: &ServerSnapshot) {
        // First pass: enable all plugins, collect actions
        let mut all_actions = Vec::new();
        for plugin in &mut self.plugins {
            let mut api = ServerApiImpl::new(snapshot);
            plugin.on_enable(&mut api);
            all_actions.extend(api.take_actions());
        }
        // Apply register-command actions immediately
        self.apply_internal_actions(all_actions);
    }

    /// Disable all registered plugins.
    pub fn disable_all(&mut self) {
        for plugin in &mut self.plugins {
            plugin.on_disable();
        }
    }

    /// Dispatch an event to all plugins. Returns the combined result and pending actions.
    pub fn dispatch(
        &mut self,
        event: &PluginEvent,
        snapshot: &ServerSnapshot,
    ) -> (EventResult, Vec<PendingAction>) {
        let cancellable = event.is_cancellable();
        let mut all_actions = Vec::new();
        let mut final_result = EventResult::Continue;

        for plugin in &mut self.plugins {
            let mut api = ServerApiImpl::new(snapshot);
            let result = plugin.on_event(event, &mut api);
            all_actions.extend(api.take_actions());

            if cancellable && result == EventResult::Cancelled {
                final_result = EventResult::Cancelled;
                break; // Stop propagation
            }
        }

        (final_result, all_actions)
    }

    /// Tick the scheduler. Returns pending actions from fired tasks.
    pub fn tick_scheduler(&mut self, snapshot: &ServerSnapshot) -> Vec<PendingAction> {
        let mut all_actions = Vec::new();
        let mut fired: Vec<(String, u32)> = Vec::new();

        // Decrement and collect fired tasks
        for task in &mut self.tasks {
            if task.remaining_ticks > 0 {
                task.remaining_ticks -= 1;
            }
            if task.remaining_ticks == 0 {
                fired.push((task.plugin_name.clone(), task.task_id));
                if let Some(interval) = task.interval {
                    task.remaining_ticks = interval;
                }
            }
        }

        // Remove one-shot tasks that fired
        self.tasks
            .retain(|t| t.remaining_ticks > 0 || t.interval.is_some());

        // Call on_task for each fired task
        for (plugin_name, task_id) in fired {
            if let Some(plugin) = self
                .plugins
                .iter_mut()
                .find(|p| p.info().name == plugin_name)
            {
                let mut api = ServerApiImpl::new(snapshot);
                plugin.on_task(task_id, &mut api);
                all_actions.extend(api.take_actions());
            }
        }

        all_actions
    }

    /// Handle a plugin-registered command. Returns (response_message, pending_actions).
    pub fn handle_command(
        &mut self,
        command: &str,
        args: &[String],
        sender: &str,
        snapshot: &ServerSnapshot,
    ) -> (Option<String>, Vec<PendingAction>) {
        let plugin_name = match self.plugin_commands.get(command) {
            Some(name) => name.clone(),
            None => return (None, Vec::new()),
        };

        if let Some(plugin) = self
            .plugins
            .iter_mut()
            .find(|p| p.info().name == plugin_name)
        {
            let mut api = ServerApiImpl::new(snapshot);
            let response = plugin.on_command(command, args, sender, &mut api);
            (response, api.take_actions())
        } else {
            (None, Vec::new())
        }
    }

    /// Load configs for all plugins from disk.
    #[allow(dead_code)]
    pub fn load_configs(&mut self) {
        for plugin in &mut self.plugins {
            let info = plugin.info();
            if let Some(default_config) = plugin.default_config() {
                let plugin_dir = std::path::PathBuf::from(format!("plugins/{}", info.name));
                let config_path = plugin_dir.join("config.json");

                let config = if config_path.exists() {
                    match std::fs::read_to_string(&config_path) {
                        Ok(data) => match serde_json::from_str(&data) {
                            Ok(v) => v,
                            Err(e) => {
                                warn!("Failed to parse config for {}: {e}", info.name);
                                default_config.clone()
                            }
                        },
                        Err(e) => {
                            warn!("Failed to read config for {}: {e}", info.name);
                            default_config.clone()
                        }
                    }
                } else {
                    // Write default config
                    std::fs::create_dir_all(&plugin_dir).ok();
                    if let Ok(json) = serde_json::to_string_pretty(&default_config) {
                        std::fs::write(&config_path, json).ok();
                    }
                    default_config
                };

                plugin.load_config(config);
            }
        }
    }

    /// Apply internal actions (RegisterCommand, ScheduleTask, CancelTask) immediately.
    #[allow(dead_code)]
    fn apply_internal_actions(&mut self, actions: Vec<PendingAction>) {
        for action in actions {
            match action {
                PendingAction::RegisterCommand {
                    name, plugin_name, ..
                } => {
                    self.plugin_commands.insert(name, plugin_name);
                }
                PendingAction::ScheduleTask { task } => {
                    self.tasks.push(task);
                }
                PendingAction::CancelTask {
                    plugin_name,
                    task_id,
                } => {
                    self.tasks
                        .retain(|t| !(t.plugin_name == plugin_name && t.task_id == task_id));
                }
                PendingAction::Log { level, message } => match level {
                    LogLevel::Info => info!("[plugin] {message}"),
                    LogLevel::Warn => warn!("[plugin] {message}"),
                    LogLevel::Error => error!("[plugin] {message}"),
                    LogLevel::Debug => debug!("[plugin] {message}"),
                },
                _ => {} // Other actions need async context, ignored here
            }
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use mc_rs_plugin_api::{DamageCause, PluginInfo};

    fn empty_snapshot() -> ServerSnapshot {
        ServerSnapshot {
            players: Vec::new(),
            world_time: 6000,
            current_tick: 100,
            is_raining: false,
        }
    }

    fn snapshot_with_player() -> ServerSnapshot {
        ServerSnapshot {
            players: vec![PluginPlayer {
                name: "Alice".into(),
                uuid: "uuid-1".into(),
                runtime_id: 1,
                position: (0.5, 65.62, 0.5),
                gamemode: 0,
                health: 20.0,
            }],
            world_time: 6000,
            current_tick: 100,
            is_raining: false,
        }
    }

    /// A test plugin that cancels PlayerDamage and greets on join.
    struct TestPlugin {
        enabled: bool,
    }

    impl TestPlugin {
        fn new() -> Self {
            Self { enabled: false }
        }
    }

    impl Plugin for TestPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                name: "TestPlugin".into(),
                version: "0.1.0".into(),
                description: "Test".into(),
                author: "Test".into(),
            }
        }

        fn on_enable(&mut self, api: &mut dyn ServerApi) {
            self.enabled = true;
            api.register_command("test", "A test command", "TestPlugin");
        }

        fn on_disable(&mut self) {
            self.enabled = false;
        }

        fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult {
            match event {
                PluginEvent::PlayerDamage { .. } => EventResult::Cancelled,
                PluginEvent::PlayerJoin { player } => {
                    api.send_message(&player.name, "Welcome!");
                    EventResult::Continue
                }
                _ => EventResult::Continue,
            }
        }

        fn on_task(&mut self, task_id: u32, api: &mut dyn ServerApi) {
            api.broadcast_message(&format!("Task {task_id} fired!"));
        }

        fn on_command(
            &mut self,
            _command: &str,
            args: &[String],
            _sender: &str,
            _api: &mut dyn ServerApi,
        ) -> Option<String> {
            Some(format!("Test OK: {} args", args.len()))
        }
    }

    #[test]
    fn enable_all_calls_on_enable() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new()));
        mgr.enable_all(&empty_snapshot());
        assert!(mgr.plugin_commands.contains_key("test"));
    }

    #[test]
    fn dispatch_cancels_damage() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new()));
        mgr.enable_all(&empty_snapshot());

        let event = PluginEvent::PlayerDamage {
            player: PluginPlayer {
                name: "Alice".into(),
                uuid: "u1".into(),
                runtime_id: 1,
                position: (0.0, 0.0, 0.0),
                gamemode: 0,
                health: 20.0,
            },
            damage: 5.0,
            cause: DamageCause::Attack,
        };
        let (result, _) = mgr.dispatch(&event, &snapshot_with_player());
        assert_eq!(result, EventResult::Cancelled);
    }

    #[test]
    fn dispatch_continues_for_join() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new()));
        mgr.enable_all(&empty_snapshot());

        let event = PluginEvent::PlayerJoin {
            player: PluginPlayer {
                name: "Alice".into(),
                uuid: "u1".into(),
                runtime_id: 1,
                position: (0.0, 0.0, 0.0),
                gamemode: 0,
                health: 20.0,
            },
        };
        let (result, actions) = mgr.dispatch(&event, &snapshot_with_player());
        assert_eq!(result, EventResult::Continue);
        // Should have a SendMessage action
        assert!(actions.iter().any(|a| matches!(a, PendingAction::SendMessage { player_name, .. } if player_name == "Alice")));
    }

    #[test]
    fn dispatch_no_plugins_continues() {
        let mut mgr = PluginManager::new();
        let event = PluginEvent::ServerStarted;
        let (result, actions) = mgr.dispatch(&event, &empty_snapshot());
        assert_eq!(result, EventResult::Continue);
        assert!(actions.is_empty());
    }

    #[test]
    fn scheduler_fires_delayed_task() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new()));
        mgr.enable_all(&empty_snapshot());

        mgr.tasks.push(ScheduledTask {
            plugin_name: "TestPlugin".into(),
            task_id: 42,
            remaining_ticks: 2,
            interval: None,
        });

        // Tick 1: not yet
        let actions = mgr.tick_scheduler(&empty_snapshot());
        assert!(actions.is_empty());
        assert_eq!(mgr.tasks.len(), 1);

        // Tick 2: fires and removes
        let actions = mgr.tick_scheduler(&empty_snapshot());
        assert!(!actions.is_empty());
        assert!(mgr.tasks.is_empty()); // one-shot removed
    }

    #[test]
    fn scheduler_fires_repeating_task() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new()));
        mgr.enable_all(&empty_snapshot());

        mgr.tasks.push(ScheduledTask {
            plugin_name: "TestPlugin".into(),
            task_id: 10,
            remaining_ticks: 1,
            interval: Some(3),
        });

        // Tick 1: fires
        let actions = mgr.tick_scheduler(&empty_snapshot());
        assert!(!actions.is_empty());
        // Task should still exist with reset remaining
        assert_eq!(mgr.tasks.len(), 1);
        assert_eq!(mgr.tasks[0].remaining_ticks, 3);
    }

    #[test]
    fn handle_command_routes_to_plugin() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new()));
        mgr.enable_all(&empty_snapshot());

        let (response, _actions) = mgr.handle_command(
            "test",
            &["a".into(), "b".into()],
            "Alice",
            &empty_snapshot(),
        );
        assert_eq!(response, Some("Test OK: 2 args".into()));
    }

    #[test]
    fn handle_unknown_command_returns_none() {
        let mut mgr = PluginManager::new();
        let (response, actions) = mgr.handle_command("unknown", &[], "Alice", &empty_snapshot());
        assert!(response.is_none());
        assert!(actions.is_empty());
    }

    #[test]
    fn server_api_impl_reads_snapshot() {
        let snapshot = snapshot_with_player();
        let api = ServerApiImpl::new(&snapshot);
        assert_eq!(api.online_players().len(), 1);
        assert!(api.get_player("Alice").is_some());
        assert!(api.get_player("Bob").is_none());
        assert_eq!(api.get_time(), 6000);
        assert_eq!(api.get_tick(), 100);
        assert!(!api.is_raining());
    }

    #[test]
    fn server_api_impl_accumulates_actions() {
        let snapshot = empty_snapshot();
        let mut api = ServerApiImpl::new(&snapshot);
        api.send_message("Alice", "Hello");
        api.broadcast_message("Hi all");
        api.set_time(12000);
        let actions = api.take_actions();
        assert_eq!(actions.len(), 3);
    }
}
