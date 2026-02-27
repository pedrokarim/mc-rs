//! WASM plugin runtime using wasmtime.

mod host_functions;
mod manifest;

pub use manifest::WasmPluginManifest;

use mc_rs_plugin_api::*;
use std::path::Path;
use tracing::{error, info, warn};
use wasmtime::{
    Config, Engine, Instance, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

// ─── Action descriptors (accumulated by host functions) ─────────────────────

/// An action queued by a WASM plugin during a callback.
/// These are flushed through the ServerApi after each guest call.
pub(crate) enum WasmAction {
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
    Log {
        level: LogLevel,
        message: String,
    },
    RegisterCommand {
        name: String,
        description: String,
    },
    ScheduleDelayed {
        delay_ticks: u64,
        task_id: u32,
    },
    ScheduleRepeating {
        delay_ticks: u64,
        interval_ticks: u64,
        task_id: u32,
    },
    CancelTask {
        task_id: u32,
    },
}

// ─── Host data (stored in wasmtime::Store) ──────────────────────────────────

pub(crate) struct WasmHostData {
    pub actions: Vec<WasmAction>,
    pub cached_time: i64,
    pub cached_tick: u64,
    pub cached_is_raining: bool,
    pub cached_players_json: String,
    pub plugin_name: String,
    limits: StoreLimits,
}

impl WasmHostData {
    fn new(plugin_name: String, max_memory_pages: u64) -> Self {
        Self {
            actions: Vec::new(),
            cached_time: 0,
            cached_tick: 0,
            cached_is_raining: false,
            cached_players_json: "[]".to_string(),
            plugin_name,
            limits: StoreLimitsBuilder::new()
                .memory_size(max_memory_pages as usize * 65536)
                .build(),
        }
    }

    // Methods called by host functions:
    pub fn send_message(&mut self, player_name: String, message: String) {
        self.actions.push(WasmAction::SendMessage {
            player_name,
            message,
        });
    }
    pub fn broadcast_message(&mut self, message: String) {
        self.actions.push(WasmAction::BroadcastMessage { message });
    }
    pub fn kick_player(&mut self, player_name: String, reason: String) {
        self.actions.push(WasmAction::KickPlayer {
            player_name,
            reason,
        });
    }
    pub fn set_player_health(&mut self, player_name: String, health: f32) {
        self.actions.push(WasmAction::SetPlayerHealth {
            player_name,
            health,
        });
    }
    pub fn set_player_food(&mut self, player_name: String, food: i32) {
        self.actions
            .push(WasmAction::SetPlayerFood { player_name, food });
    }
    pub fn teleport_player(&mut self, player_name: String, x: f32, y: f32, z: f32) {
        self.actions.push(WasmAction::TeleportPlayer {
            player_name,
            x,
            y,
            z,
        });
    }
    pub fn set_time(&mut self, time: i64) {
        self.actions.push(WasmAction::SetTime { time });
    }
    pub fn spawn_mob(&mut self, mob_type: String, x: f32, y: f32, z: f32) {
        self.actions
            .push(WasmAction::SpawnMob { mob_type, x, y, z });
    }
    pub fn remove_mob(&mut self, runtime_id: u64) {
        self.actions.push(WasmAction::RemoveMob { runtime_id });
    }
    pub fn log(&mut self, level: LogLevel, message: String) {
        let prefixed = format!("[{}] {}", self.plugin_name, message);
        self.actions.push(WasmAction::Log {
            level,
            message: prefixed,
        });
    }
    pub fn register_command(&mut self, name: String, description: String) {
        self.actions
            .push(WasmAction::RegisterCommand { name, description });
    }
    pub fn schedule_delayed(&mut self, delay_ticks: u64, task_id: u32) {
        self.actions.push(WasmAction::ScheduleDelayed {
            delay_ticks,
            task_id,
        });
    }
    pub fn schedule_repeating(&mut self, delay_ticks: u64, interval_ticks: u64, task_id: u32) {
        self.actions.push(WasmAction::ScheduleRepeating {
            delay_ticks,
            interval_ticks,
            task_id,
        });
    }
    pub fn cancel_task(&mut self, task_id: u32) {
        self.actions.push(WasmAction::CancelTask { task_id });
    }
}

// ─── WasmPlugin ─────────────────────────────────────────────────────────────

pub struct WasmPlugin {
    manifest: WasmPluginManifest,
    store: Store<WasmHostData>,
    // Keep instance alive (it owns the WASM module's runtime state).
    #[allow(dead_code)]
    instance: Instance,
    memory: Memory,
    fn_malloc: TypedFunc<i32, i32>,
    #[allow(dead_code)]
    fn_free: TypedFunc<(i32, i32), ()>,
    fn_on_enable: TypedFunc<(), ()>,
    fn_on_disable: TypedFunc<(), ()>,
    fn_on_event: TypedFunc<(i32, i32), i32>,
    fn_on_task: TypedFunc<i32, ()>,
    fn_on_command: TypedFunc<(i32, i32), i32>,
    #[allow(dead_code)]
    fn_plugin_info: TypedFunc<(), i32>,
    #[allow(dead_code)]
    fn_default_config: TypedFunc<(), i32>,
    fn_load_config: TypedFunc<(i32, i32), ()>,
}

impl WasmPlugin {
    /// Write a string into guest memory via __malloc. Returns guest pointer.
    fn write_to_guest(&mut self, data: &str) -> Result<i32, wasmtime::Error> {
        let len = data.len() as i32;
        let ptr = self.fn_malloc.call(&mut self.store, len)?;
        let mem = self.memory.data_mut(&mut self.store);
        let start = ptr as usize;
        let end = start + data.len();
        if end <= mem.len() {
            mem[start..end].copy_from_slice(data.as_bytes());
        }
        Ok(ptr)
    }

    /// Read a length-prefixed string ([u32_le len][utf8 data]) from guest memory.
    fn read_length_prefixed(&self, ptr: i32) -> Option<String> {
        if ptr == 0 {
            return None;
        }
        let mem = self.memory.data(&self.store);
        let p = ptr as usize;
        if p + 4 > mem.len() {
            return None;
        }
        let len = u32::from_le_bytes(mem[p..p + 4].try_into().ok()?) as usize;
        if p + 4 + len > mem.len() {
            return None;
        }
        String::from_utf8(mem[p + 4..p + 4 + len].to_vec()).ok()
    }

    /// Cache snapshot data from the ServerApi into WasmHostData before a guest call.
    fn sync_snapshot(&mut self, api: &dyn ServerApi) {
        let data = self.store.data_mut();
        data.cached_time = api.get_time();
        data.cached_tick = api.get_tick();
        data.cached_is_raining = api.is_raining();
        let players = api.online_players();
        data.cached_players_json =
            serde_json::to_string(&players).unwrap_or_else(|_| "[]".to_string());
        data.actions.clear();
    }

    /// Reset fuel for a new guest call.
    fn refuel(&mut self, fuel: u64) {
        let _ = self.store.set_fuel(fuel);
    }

    /// Drain accumulated WasmActions and forward them through the ServerApi.
    fn flush_actions(&mut self, api: &mut dyn ServerApi) {
        let actions: Vec<WasmAction> = std::mem::take(&mut self.store.data_mut().actions);
        let plugin_name = self.manifest.name.clone();
        for action in actions {
            match action {
                WasmAction::SendMessage {
                    player_name,
                    message,
                } => api.send_message(&player_name, &message),
                WasmAction::BroadcastMessage { message } => api.broadcast_message(&message),
                WasmAction::KickPlayer {
                    player_name,
                    reason,
                } => api.kick_player(&player_name, &reason),
                WasmAction::SetPlayerHealth {
                    player_name,
                    health,
                } => api.set_player_health(&player_name, health),
                WasmAction::SetPlayerFood { player_name, food } => {
                    api.set_player_food(&player_name, food)
                }
                WasmAction::TeleportPlayer {
                    player_name,
                    x,
                    y,
                    z,
                } => api.teleport_player(&player_name, x, y, z),
                WasmAction::SetTime { time } => api.set_time(time),
                WasmAction::SpawnMob { mob_type, x, y, z } => api.spawn_mob(&mob_type, x, y, z),
                WasmAction::RemoveMob { runtime_id } => api.remove_mob(runtime_id),
                WasmAction::Log { level, message } => api.log(level, &message),
                WasmAction::RegisterCommand { name, description } => {
                    api.register_command(&name, &description, &plugin_name)
                }
                WasmAction::ScheduleDelayed {
                    delay_ticks,
                    task_id,
                } => api.schedule_delayed(&plugin_name, delay_ticks, task_id),
                WasmAction::ScheduleRepeating {
                    delay_ticks,
                    interval_ticks,
                    task_id,
                } => api.schedule_repeating(&plugin_name, delay_ticks, interval_ticks, task_id),
                WasmAction::CancelTask { task_id } => api.cancel_task(&plugin_name, task_id),
            }
        }
    }
}

// ─── Plugin trait implementation ────────────────────────────────────────────

impl Plugin for WasmPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: self.manifest.name.clone(),
            version: self.manifest.version.clone(),
            description: self.manifest.description.clone(),
            author: self.manifest.author.clone(),
        }
    }

    fn on_enable(&mut self, api: &mut dyn ServerApi) {
        self.sync_snapshot(api);
        self.refuel(self.manifest.fuel_on_enable);
        if let Err(e) = self.fn_on_enable.call(&mut self.store, ()) {
            error!("[wasm:{}] on_enable trapped: {e}", self.manifest.name);
            return;
        }
        self.flush_actions(api);
    }

    fn on_disable(&mut self) {
        self.refuel(self.manifest.fuel_on_enable);
        if let Err(e) = self.fn_on_disable.call(&mut self.store, ()) {
            error!("[wasm:{}] on_disable trapped: {e}", self.manifest.name);
        }
    }

    fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult {
        self.sync_snapshot(api);
        self.refuel(self.manifest.fuel_per_event);
        let json = match serde_json::to_string(event) {
            Ok(j) => j,
            Err(e) => {
                error!(
                    "[wasm:{}] failed to serialize event: {e}",
                    self.manifest.name
                );
                return EventResult::Continue;
            }
        };
        let ptr = match self.write_to_guest(&json) {
            Ok(p) => p,
            Err(e) => {
                error!("[wasm:{}] write_to_guest failed: {e}", self.manifest.name);
                return EventResult::Continue;
            }
        };
        let result = match self
            .fn_on_event
            .call(&mut self.store, (ptr, json.len() as i32))
        {
            Ok(code) => {
                if code == 1 {
                    EventResult::Cancelled
                } else {
                    EventResult::Continue
                }
            }
            Err(e) => {
                warn!("[wasm:{}] on_event trapped: {e}", self.manifest.name);
                EventResult::Continue
            }
        };
        self.flush_actions(api);
        result
    }

    fn on_task(&mut self, task_id: u32, api: &mut dyn ServerApi) {
        self.sync_snapshot(api);
        self.refuel(self.manifest.fuel_per_task);
        if let Err(e) = self.fn_on_task.call(&mut self.store, task_id as i32) {
            warn!(
                "[wasm:{}] on_task({task_id}) trapped: {e}",
                self.manifest.name
            );
        }
        self.flush_actions(api);
    }

    fn on_command(
        &mut self,
        command: &str,
        args: &[String],
        sender: &str,
        api: &mut dyn ServerApi,
    ) -> Option<String> {
        self.sync_snapshot(api);
        self.refuel(self.manifest.fuel_per_command);
        let input = serde_json::json!({
            "command": command,
            "args": args,
            "sender": sender,
        });
        let json = serde_json::to_string(&input).ok()?;
        let ptr = self.write_to_guest(&json).ok()?;
        let result_ptr = match self
            .fn_on_command
            .call(&mut self.store, (ptr, json.len() as i32))
        {
            Ok(p) => p,
            Err(e) => {
                warn!("[wasm:{}] on_command trapped: {e}", self.manifest.name);
                self.flush_actions(api);
                return None;
            }
        };
        self.flush_actions(api);
        self.read_length_prefixed(result_ptr)
    }

    fn default_config(&self) -> Option<serde_json::Value> {
        // WASM plugins use plugin.toml for config, not this method.
        None
    }

    fn load_config(&mut self, config: serde_json::Value) {
        self.refuel(self.manifest.fuel_on_enable);
        let json = match serde_json::to_string(&config) {
            Ok(j) => j,
            Err(_) => return,
        };
        let ptr = match self.write_to_guest(&json) {
            Ok(p) => p,
            Err(_) => return,
        };
        let _ = self
            .fn_load_config
            .call(&mut self.store, (ptr, json.len() as i32));
    }
}

// WasmPlugin is Send because wasmtime::Store<T> is Send when T: Send,
// and WasmHostData contains only owned data (Strings, Vecs, StoreLimits).
// wasmtime::Instance, Memory, TypedFunc are all Send when the Store is.
unsafe impl Send for WasmPlugin {}

// ─── Engine & loading ───────────────────────────────────────────────────────

/// Create a wasmtime Engine with fuel metering enabled.
pub fn create_engine() -> Engine {
    let mut config = Config::new();
    config.consume_fuel(true);
    Engine::new(&config).expect("failed to create wasmtime engine")
}

/// Load a single WASM plugin from a directory containing plugin.toml.
fn load_single_plugin(dir: &Path, engine: &Engine) -> Result<WasmPlugin, String> {
    let manifest_path = dir.join("plugin.toml");
    let toml_content =
        std::fs::read_to_string(&manifest_path).map_err(|e| format!("read plugin.toml: {e}"))?;
    let manifest =
        WasmPluginManifest::parse(&toml_content).map_err(|e| format!("parse plugin.toml: {e}"))?;

    let wasm_path = dir.join(&manifest.wasm_file);
    let module = Module::from_file(engine, &wasm_path)
        .map_err(|e| format!("load {}: {e}", manifest.wasm_file))?;

    let host_data = WasmHostData::new(manifest.name.clone(), manifest.max_memory_pages);
    let mut store = Store::new(engine, host_data);
    store.limiter(|data| &mut data.limits);
    store
        .set_fuel(manifest.fuel_on_enable)
        .map_err(|e| format!("set fuel: {e}"))?;

    let linker = host_functions::build_linker(engine).map_err(|e| format!("build linker: {e}"))?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("instantiate: {e}"))?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or("WASM module must export 'memory'")?;
    let fn_malloc = instance
        .get_typed_func::<i32, i32>(&mut store, "__malloc")
        .map_err(|e| format!("missing __malloc: {e}"))?;
    let fn_free = instance
        .get_typed_func::<(i32, i32), ()>(&mut store, "__free")
        .map_err(|e| format!("missing __free: {e}"))?;
    let fn_on_enable = instance
        .get_typed_func::<(), ()>(&mut store, "__on_enable")
        .map_err(|e| format!("missing __on_enable: {e}"))?;
    let fn_on_disable = instance
        .get_typed_func::<(), ()>(&mut store, "__on_disable")
        .map_err(|e| format!("missing __on_disable: {e}"))?;
    let fn_on_event = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "__on_event")
        .map_err(|e| format!("missing __on_event: {e}"))?;
    let fn_on_task = instance
        .get_typed_func::<i32, ()>(&mut store, "__on_task")
        .map_err(|e| format!("missing __on_task: {e}"))?;
    let fn_on_command = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "__on_command")
        .map_err(|e| format!("missing __on_command: {e}"))?;
    let fn_plugin_info = instance
        .get_typed_func::<(), i32>(&mut store, "__plugin_info")
        .map_err(|e| format!("missing __plugin_info: {e}"))?;
    let fn_default_config = instance
        .get_typed_func::<(), i32>(&mut store, "__default_config")
        .map_err(|e| format!("missing __default_config: {e}"))?;
    let fn_load_config = instance
        .get_typed_func::<(i32, i32), ()>(&mut store, "__load_config")
        .map_err(|e| format!("missing __load_config: {e}"))?;

    Ok(WasmPlugin {
        manifest,
        store,
        instance,
        memory,
        fn_malloc,
        fn_free,
        fn_on_enable,
        fn_on_disable,
        fn_on_event,
        fn_on_task,
        fn_on_command,
        fn_plugin_info,
        fn_default_config,
        fn_load_config,
    })
}

/// Scan a directory for WASM plugin subdirectories and load all valid plugins.
pub fn load_wasm_plugins(plugins_dir: &Path, engine: &Engine) -> Vec<Box<dyn Plugin>> {
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

    let entries = match std::fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(_) => return plugins,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("plugin.toml").exists() {
            continue;
        }
        match load_single_plugin(&path, engine) {
            Ok(plugin) => {
                info!(
                    "Loaded WASM plugin: {} v{}",
                    plugin.manifest.name, plugin.manifest.version
                );
                plugins.push(Box::new(plugin));
            }
            Err(e) => {
                error!("Failed to load WASM plugin from {}: {e}", path.display());
            }
        }
    }

    plugins
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_engine_succeeds() {
        // Verify engine creation with fuel metering does not panic.
        let _engine = create_engine();
    }

    #[test]
    fn host_data_accumulates_actions() {
        let mut hd = WasmHostData::new("test".to_string(), 256);
        hd.broadcast_message("hello".to_string());
        hd.send_message("player".to_string(), "msg".to_string());
        hd.set_time(1000);
        assert_eq!(hd.actions.len(), 3);
    }

    #[test]
    fn load_empty_dir() {
        let dir = std::env::temp_dir().join("mc-rs-wasm-test-empty");
        let _ = std::fs::create_dir_all(&dir);
        let engine = create_engine();
        let plugins = load_wasm_plugins(&dir, &engine);
        assert!(plugins.is_empty());
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_nonexistent_dir() {
        let dir = std::path::PathBuf::from("/tmp/mc-rs-wasm-nonexistent-12345");
        let engine = create_engine();
        let plugins = load_wasm_plugins(&dir, &engine);
        assert!(plugins.is_empty());
    }

    /// Helper: create a WasmPlugin from a WAT string and manifest for testing.
    fn make_wat_plugin(engine: &Engine, wat: &str, name: &str) -> WasmPlugin {
        let module = Module::new(engine, wat).expect("WAT compilation failed");
        let manifest = WasmPluginManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            wasm_file: "test.wasm".to_string(),
            fuel_per_event: 1_000_000,
            fuel_per_command: 1_000_000,
            fuel_per_task: 500_000,
            fuel_on_enable: 5_000_000,
            max_memory_pages: 256,
        };
        let host_data = WasmHostData::new(manifest.name.clone(), manifest.max_memory_pages);
        let mut store = Store::new(engine, host_data);
        store.limiter(|data| &mut data.limits);
        store.set_fuel(manifest.fuel_on_enable).unwrap();

        let linker = host_functions::build_linker(engine).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let memory = instance.get_memory(&mut store, "memory").unwrap();
        let fn_malloc = instance
            .get_typed_func::<i32, i32>(&mut store, "__malloc")
            .unwrap();
        let fn_free = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "__free")
            .unwrap();
        let fn_on_enable = instance
            .get_typed_func::<(), ()>(&mut store, "__on_enable")
            .unwrap();
        let fn_on_disable = instance
            .get_typed_func::<(), ()>(&mut store, "__on_disable")
            .unwrap();
        let fn_on_event = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "__on_event")
            .unwrap();
        let fn_on_task = instance
            .get_typed_func::<i32, ()>(&mut store, "__on_task")
            .unwrap();
        let fn_on_command = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "__on_command")
            .unwrap();
        let fn_plugin_info = instance
            .get_typed_func::<(), i32>(&mut store, "__plugin_info")
            .unwrap();
        let fn_default_config = instance
            .get_typed_func::<(), i32>(&mut store, "__default_config")
            .unwrap();
        let fn_load_config = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "__load_config")
            .unwrap();

        WasmPlugin {
            manifest,
            store,
            instance,
            memory,
            fn_malloc,
            fn_free,
            fn_on_enable,
            fn_on_disable,
            fn_on_event,
            fn_on_task,
            fn_on_command,
            fn_plugin_info,
            fn_default_config,
            fn_load_config,
        }
    }

    /// Minimal WAT module with all required exports (does nothing).
    const MINIMAL_WAT: &str = r#"
        (module
            (memory (export "memory") 1)
            (global $heap_ptr (mut i32) (i32.const 1024))

            (func (export "__malloc") (param $size i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $heap_ptr))
                (global.set $heap_ptr (i32.add (global.get $heap_ptr) (local.get $size)))
                (local.get $ptr)
            )
            (func (export "__free") (param $ptr i32) (param $size i32))
            (func (export "__plugin_info") (result i32) (i32.const 0))
            (func (export "__on_enable"))
            (func (export "__on_disable"))
            (func (export "__on_event") (param $ptr i32) (param $len i32) (result i32) (i32.const 0))
            (func (export "__on_task") (param $task_id i32))
            (func (export "__on_command") (param $ptr i32) (param $len i32) (result i32) (i32.const 0))
            (func (export "__default_config") (result i32) (i32.const 0))
            (func (export "__load_config") (param $ptr i32) (param $len i32))
        )
    "#;

    #[test]
    fn minimal_wat_plugin_loads() {
        let engine = create_engine();
        let plugin = make_wat_plugin(&engine, MINIMAL_WAT, "test-wat");

        let info = plugin.info();
        assert_eq!(info.name, "test-wat");
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn wat_plugin_on_event_returns_continue() {
        let engine = create_engine();
        let mut plugin = make_wat_plugin(&engine, MINIMAL_WAT, "evt-test");

        let mut api = MockApi::new();
        let event = PluginEvent::ServerStarted;
        let result = plugin.on_event(&event, &mut api);
        assert_eq!(result, EventResult::Continue);
    }

    #[test]
    fn wat_plugin_on_enable_disable() {
        let engine = create_engine();
        let mut plugin = make_wat_plugin(&engine, MINIMAL_WAT, "lifecycle-test");

        let mut api = MockApi::new();
        plugin.on_enable(&mut api);
        plugin.on_disable();
        // Should not panic.
    }

    #[test]
    fn wat_plugin_on_command_returns_none() {
        let engine = create_engine();
        let mut plugin = make_wat_plugin(&engine, MINIMAL_WAT, "cmd-test");

        let mut api = MockApi::new();
        let result = plugin.on_command("test", &["arg1".into()], "player", &mut api);
        // The minimal WAT returns 0 from __on_command, which read_length_prefixed treats as None.
        assert!(result.is_none());
    }

    #[test]
    fn wat_plugin_on_task() {
        let engine = create_engine();
        let mut plugin = make_wat_plugin(&engine, MINIMAL_WAT, "task-test");

        let mut api = MockApi::new();
        plugin.on_task(42, &mut api);
        // Should not panic.
    }

    #[test]
    fn flush_actions_forwards_to_api() {
        let mut hd = WasmHostData::new("test".into(), 256);
        hd.send_message("Alice".into(), "Hello".into());
        hd.broadcast_message("World".into());
        hd.set_time(6000);

        let mut api = MockApi::new();
        let plugin_name = hd.plugin_name.clone();
        let actions: Vec<WasmAction> = std::mem::take(&mut hd.actions);
        for action in actions {
            match action {
                WasmAction::SendMessage {
                    player_name,
                    message,
                } => api.send_message(&player_name, &message),
                WasmAction::BroadcastMessage { message } => api.broadcast_message(&message),
                WasmAction::SetTime { time } => api.set_time(time),
                WasmAction::RegisterCommand { name, description } => {
                    api.register_command(&name, &description, &plugin_name)
                }
                _ => {}
            }
        }

        assert_eq!(api.messages.len(), 1);
        assert_eq!(api.messages[0], ("Alice".to_string(), "Hello".to_string()));
        assert_eq!(api.broadcasts, vec!["World".to_string()]);
        assert_eq!(api.time_set, Some(6000));
    }

    // Minimal ServerApi implementation for testing.
    struct MockApi {
        messages: Vec<(String, String)>,
        broadcasts: Vec<String>,
        time_set: Option<i64>,
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                messages: vec![],
                broadcasts: vec![],
                time_set: None,
            }
        }
    }

    impl ServerApi for MockApi {
        fn online_players(&self) -> Vec<PluginPlayer> {
            vec![]
        }
        fn get_player(&self, _: &str) -> Option<PluginPlayer> {
            None
        }
        fn send_message(&mut self, p: &str, m: &str) {
            self.messages.push((p.into(), m.into()));
        }
        fn broadcast_message(&mut self, m: &str) {
            self.broadcasts.push(m.into());
        }
        fn kick_player(&mut self, _: &str, _: &str) {}
        fn set_player_health(&mut self, _: &str, _: f32) {}
        fn set_player_food(&mut self, _: &str, _: i32) {}
        fn teleport_player(&mut self, _: &str, _: f32, _: f32, _: f32) {}
        fn get_time(&self) -> i64 {
            0
        }
        fn set_time(&mut self, t: i64) {
            self.time_set = Some(t);
        }
        fn is_raining(&self) -> bool {
            false
        }
        fn spawn_mob(&mut self, _: &str, _: f32, _: f32, _: f32) {}
        fn remove_mob(&mut self, _: u64) {}
        fn get_tick(&self) -> u64 {
            0
        }
        fn log(&self, _: LogLevel, _: &str) {}
        fn schedule_delayed(&mut self, _: &str, _: u64, _: u32) {}
        fn schedule_repeating(&mut self, _: &str, _: u64, _: u64, _: u32) {}
        fn cancel_task(&mut self, _: &str, _: u32) {}
        fn register_command(&mut self, _: &str, _: &str, _: &str) {}
    }
}
