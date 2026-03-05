//! Lua scripting plugin runtime using mlua (Lua 5.4).

mod manifest;

pub use manifest::LuaPluginManifest;

use mc_rs_plugin_api::*;
use mlua::prelude::*;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

// ─── Action queue ────────────────────────────────────────────────────────────

/// An action queued by Lua code during a callback.
enum LuaAction {
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

// ─── Host data (stored in Lua app_data) ──────────────────────────────────────

struct LuaHostData {
    actions: Vec<LuaAction>,
    cached_time: i64,
    cached_tick: u64,
    cached_is_raining: bool,
    cached_players: Vec<PluginPlayer>,
    plugin_name: String,
    next_task_id: u32,
}

impl LuaHostData {
    fn new(plugin_name: String) -> Self {
        Self {
            actions: Vec::new(),
            cached_time: 0,
            cached_tick: 0,
            cached_is_raining: false,
            cached_players: Vec::new(),
            plugin_name,
            next_task_id: 1,
        }
    }

    fn alloc_task_id(&mut self) -> u32 {
        let id = self.next_task_id;
        self.next_task_id = self.next_task_id.wrapping_add(1);
        id
    }
}

// ─── LuaPlugin ───────────────────────────────────────────────────────────────

/// A Lua-based plugin loaded from a directory containing plugin.toml + main.lua.
pub struct LuaPlugin {
    manifest: LuaPluginManifest,
    lua: Lua,
    script_path: PathBuf,
}

// Safety: LuaPlugin is only accessed from the single game-tick thread.
unsafe impl Send for LuaPlugin {}

impl LuaPlugin {
    /// Sync cached fields from the ServerApi into LuaHostData.
    fn sync_snapshot(&self, api: &dyn ServerApi) {
        if let Some(mut data) = self.lua.app_data_mut::<LuaHostData>() {
            data.cached_time = api.get_time();
            data.cached_tick = api.get_tick();
            data.cached_is_raining = api.is_raining();
            data.cached_players = api.online_players();
        }
    }

    /// Drain accumulated LuaActions and forward them through the ServerApi.
    fn flush_actions(&self, api: &mut dyn ServerApi) {
        let actions: Vec<LuaAction> = self
            .lua
            .app_data_mut::<LuaHostData>()
            .map(|mut d| std::mem::take(&mut d.actions))
            .unwrap_or_default();

        let plugin_name = self.manifest.name.clone();
        for action in actions {
            match action {
                LuaAction::SendMessage {
                    player_name,
                    message,
                } => api.send_message(&player_name, &message),
                LuaAction::BroadcastMessage { message } => api.broadcast_message(&message),
                LuaAction::KickPlayer {
                    player_name,
                    reason,
                } => api.kick_player(&player_name, &reason),
                LuaAction::SetPlayerHealth {
                    player_name,
                    health,
                } => api.set_player_health(&player_name, health),
                LuaAction::SetPlayerFood { player_name, food } => {
                    api.set_player_food(&player_name, food)
                }
                LuaAction::TeleportPlayer {
                    player_name,
                    x,
                    y,
                    z,
                } => api.teleport_player(&player_name, x, y, z),
                LuaAction::SetTime { time } => api.set_time(time),
                LuaAction::SpawnMob { mob_type, x, y, z } => api.spawn_mob(&mob_type, x, y, z),
                LuaAction::RemoveMob { runtime_id } => api.remove_mob(runtime_id),
                LuaAction::Log { level, message } => {
                    api.log(level, &message);
                }
                LuaAction::RegisterCommand { name, description } => {
                    api.register_command(&name, &description, &plugin_name);
                }
                LuaAction::ScheduleDelayed {
                    delay_ticks,
                    task_id,
                } => api.schedule_delayed(&plugin_name, delay_ticks, task_id),
                LuaAction::ScheduleRepeating {
                    delay_ticks,
                    interval_ticks,
                    task_id,
                } => api.schedule_repeating(&plugin_name, delay_ticks, interval_ticks, task_id),
                LuaAction::CancelTask { task_id } => api.cancel_task(&plugin_name, task_id),
            }
        }
    }

    /// Convert a PluginEvent to a Lua table + its event name string.
    fn event_to_lua_table(lua: &Lua, event: &PluginEvent) -> LuaResult<(String, LuaTable)> {
        let t = lua.create_table()?;

        let name = match event {
            PluginEvent::PlayerJoin { player } => {
                Self::set_player_fields(lua, &t, player)?;
                "player_join"
            }
            PluginEvent::PlayerQuit { player } => {
                Self::set_player_fields(lua, &t, player)?;
                "player_quit"
            }
            PluginEvent::PlayerChat { player, message } => {
                Self::set_player_fields(lua, &t, player)?;
                t.set("message", message.as_str())?;
                "player_chat"
            }
            PluginEvent::PlayerCommand {
                player,
                command,
                args,
            } => {
                Self::set_player_fields(lua, &t, player)?;
                t.set("command", command.as_str())?;
                let args_table = lua.create_sequence_from(args.iter().map(|s| s.as_str()))?;
                t.set("args", args_table)?;
                "player_command"
            }
            PluginEvent::PlayerMove { player, from, to } => {
                Self::set_player_fields(lua, &t, player)?;
                let from_t = lua.create_table()?;
                from_t.set("x", from.0)?;
                from_t.set("y", from.1)?;
                from_t.set("z", from.2)?;
                t.set("from", from_t)?;
                let to_t = lua.create_table()?;
                to_t.set("x", to.0)?;
                to_t.set("y", to.1)?;
                to_t.set("z", to.2)?;
                t.set("to", to_t)?;
                "player_move"
            }
            PluginEvent::PlayerDeath { player, message } => {
                Self::set_player_fields(lua, &t, player)?;
                t.set("message", message.as_str())?;
                "player_death"
            }
            PluginEvent::PlayerDamage {
                player,
                damage,
                cause,
            } => {
                Self::set_player_fields(lua, &t, player)?;
                t.set("damage", *damage)?;
                t.set("cause", format!("{cause:?}"))?;
                "player_damage"
            }
            PluginEvent::PlayerRespawn { player } => {
                Self::set_player_fields(lua, &t, player)?;
                "player_respawn"
            }
            PluginEvent::BlockBreak {
                player,
                position,
                block_id,
            } => {
                Self::set_player_fields(lua, &t, player)?;
                Self::set_block_pos(lua, &t, position)?;
                t.set("block_id", *block_id)?;
                "block_break"
            }
            PluginEvent::BlockPlace {
                player,
                position,
                block_id,
            } => {
                Self::set_player_fields(lua, &t, player)?;
                Self::set_block_pos(lua, &t, position)?;
                t.set("block_id", *block_id)?;
                "block_place"
            }
            PluginEvent::MobSpawn {
                mob_type,
                runtime_id,
                position,
            } => {
                t.set("mob_type", mob_type.as_str())?;
                t.set("runtime_id", *runtime_id)?;
                t.set("x", position.0)?;
                t.set("y", position.1)?;
                t.set("z", position.2)?;
                "mob_spawn"
            }
            PluginEvent::MobDeath {
                mob_type,
                runtime_id,
                killer_runtime_id,
            } => {
                t.set("mob_type", mob_type.as_str())?;
                t.set("runtime_id", *runtime_id)?;
                if let Some(kid) = killer_runtime_id {
                    t.set("killer_runtime_id", *kid)?;
                }
                "mob_death"
            }
            PluginEvent::EntityDamage {
                runtime_id,
                damage,
                attacker_runtime_id,
            } => {
                t.set("runtime_id", *runtime_id)?;
                t.set("damage", *damage)?;
                if let Some(aid) = attacker_runtime_id {
                    t.set("attacker_runtime_id", *aid)?;
                }
                "entity_damage"
            }
            PluginEvent::WeatherChange {
                raining,
                thundering,
            } => {
                t.set("raining", *raining)?;
                t.set("thundering", *thundering)?;
                "weather_change"
            }
            PluginEvent::TimeChange { new_time } => {
                t.set("new_time", *new_time)?;
                "time_change"
            }
            PluginEvent::FormResponse {
                player,
                form_id,
                response,
            } => {
                Self::set_player_fields(lua, &t, player)?;
                t.set("form_id", *form_id)?;
                t.set("response", format!("{response:?}"))?;
                "form_response"
            }
            PluginEvent::ServerStarted => "server_started",
            PluginEvent::ServerStopping => "server_stopping",
        };

        t.set("cancelled", false)?;
        Ok((name.to_string(), t))
    }

    fn set_player_fields(lua: &Lua, t: &LuaTable, player: &PluginPlayer) -> LuaResult<()> {
        let pt = lua.create_table()?;
        pt.set("name", player.name.as_str())?;
        pt.set("uuid", player.uuid.as_str())?;
        pt.set("runtime_id", player.runtime_id)?;
        pt.set("x", player.position.0)?;
        pt.set("y", player.position.1)?;
        pt.set("z", player.position.2)?;
        pt.set("gamemode", player.gamemode)?;
        pt.set("health", player.health)?;
        t.set("player", pt)?;
        Ok(())
    }

    fn set_block_pos(lua: &Lua, t: &LuaTable, pos: &PluginBlockPos) -> LuaResult<()> {
        let pt = lua.create_table()?;
        pt.set("x", pos.x)?;
        pt.set("y", pos.y)?;
        pt.set("z", pos.z)?;
        t.set("position", pt)?;
        Ok(())
    }
}

// ─── Plugin trait implementation ─────────────────────────────────────────────

impl Plugin for LuaPlugin {
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

        let script = match std::fs::read_to_string(&self.script_path) {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "Failed to read Lua script {}: {e}",
                    self.script_path.display()
                );
                return;
            }
        };

        if let Err(e) = self.lua.load(&script).set_name(&self.manifest.name).exec() {
            error!("Failed to execute Lua plugin '{}': {e}", self.manifest.name);
            return;
        }

        info!(
            "Enabled Lua plugin: {} v{}",
            self.manifest.name, self.manifest.version
        );
        self.flush_actions(api);
    }

    fn on_disable(&mut self) {
        info!("Disabled Lua plugin: {}", self.manifest.name);
    }

    fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult {
        self.sync_snapshot(api);

        let result = (|| -> LuaResult<EventResult> {
            let (event_name, event_table) = Self::event_to_lua_table(&self.lua, event)?;

            // Get handlers table: __event_handlers[event_name]
            let handlers: LuaTable = match self.lua.named_registry_value("__event_handlers") {
                Ok(h) => h,
                Err(_) => return Ok(EventResult::Continue),
            };
            let handler_list: LuaTable = match handlers.get(event_name.as_str()) {
                Ok(h) => h,
                Err(_) => return Ok(EventResult::Continue),
            };

            // Call each handler with the event table
            let len = handler_list.len()?;
            for i in 1..=len {
                let handler: LuaFunction = handler_list.get(i)?;
                handler.call::<()>(event_table.clone())?;

                // Check if cancelled
                if event_table.get::<bool>("cancelled").unwrap_or(false) {
                    return Ok(EventResult::Cancelled);
                }
            }

            Ok(EventResult::Continue)
        })();

        let event_result = match result {
            Ok(r) => r,
            Err(e) => {
                warn!("Lua plugin '{}' error in on_event: {e}", self.manifest.name);
                EventResult::Continue
            }
        };

        self.flush_actions(api);
        event_result
    }

    fn on_task(&mut self, task_id: u32, api: &mut dyn ServerApi) {
        self.sync_snapshot(api);

        let result = (|| -> LuaResult<()> {
            let handlers: LuaTable = self.lua.named_registry_value("__task_handlers")?;
            let handler: LuaFunction = handlers.get(task_id)?;
            handler.call::<()>(())?;
            Ok(())
        })();

        if let Err(e) = result {
            warn!(
                "Lua plugin '{}' error in on_task({}): {e}",
                self.manifest.name, task_id
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

        let result = (|| -> LuaResult<Option<String>> {
            let handlers: LuaTable = self.lua.named_registry_value("__command_handlers")?;
            let handler: LuaFunction = handlers.get(command.to_string())?;

            let args_table = self
                .lua
                .create_sequence_from(args.iter().map(|s| s.as_str()))?;

            let response: LuaValue = handler.call((sender, args_table))?;
            match response {
                LuaValue::String(s) => Ok(Some(s.to_str()?.to_string())),
                LuaValue::Nil => Ok(None),
                _ => Ok(Some(response.to_string()?)),
            }
        })();

        let response = match result {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    "Lua plugin '{}' error in on_command({}): {e}",
                    self.manifest.name, command
                );
                None
            }
        };

        self.flush_actions(api);
        response
    }
}

// ─── Sandbox & API setup ─────────────────────────────────────────────────────

/// Remove dangerous Lua globals for sandboxing.
fn setup_sandbox(lua: &Lua, memory_mb: usize) -> LuaResult<()> {
    let globals = lua.globals();
    globals.set("os", LuaValue::Nil)?;
    globals.set("io", LuaValue::Nil)?;
    globals.set("debug", LuaValue::Nil)?;
    globals.set("loadfile", LuaValue::Nil)?;
    globals.set("dofile", LuaValue::Nil)?;

    lua.set_memory_limit(memory_mb * 1024 * 1024)?;
    Ok(())
}

/// Set up the global `mc` table with all API functions.
fn setup_mc_api(lua: &Lua) -> LuaResult<()> {
    // Create registry tables for handlers
    let event_handlers = lua.create_table()?;
    lua.set_named_registry_value("__event_handlers", event_handlers)?;
    let task_handlers = lua.create_table()?;
    lua.set_named_registry_value("__task_handlers", task_handlers)?;
    let command_handlers = lua.create_table()?;
    lua.set_named_registry_value("__command_handlers", command_handlers)?;

    let mc = lua.create_table()?;

    // mc.on(event_name, handler)
    mc.set(
        "on",
        lua.create_function(|lua, (event_name, handler): (String, LuaFunction)| {
            let handlers: LuaTable = lua.named_registry_value("__event_handlers")?;
            let list: LuaTable = match handlers.get::<LuaTable>(event_name.as_str()) {
                Ok(t) => t,
                Err(_) => {
                    let t = lua.create_table()?;
                    handlers.set(event_name.as_str(), t.clone())?;
                    t
                }
            };
            let len = list.len()? + 1;
            list.set(len, handler)?;
            Ok(())
        })?,
    )?;

    // mc.broadcast(msg)
    mc.set(
        "broadcast",
        lua.create_function(|lua, msg: String| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions
                    .push(LuaAction::BroadcastMessage { message: msg });
            }
            Ok(())
        })?,
    )?;

    // mc.send_message(player_name, msg)
    mc.set(
        "send_message",
        lua.create_function(|lua, (player_name, message): (String, String)| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::SendMessage {
                    player_name,
                    message,
                });
            }
            Ok(())
        })?,
    )?;

    // mc.kick(player_name, reason)
    mc.set(
        "kick",
        lua.create_function(|lua, (player_name, reason): (String, String)| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::KickPlayer {
                    player_name,
                    reason,
                });
            }
            Ok(())
        })?,
    )?;

    // mc.set_health(player_name, health)
    mc.set(
        "set_health",
        lua.create_function(|lua, (player_name, health): (String, f32)| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::SetPlayerHealth {
                    player_name,
                    health,
                });
            }
            Ok(())
        })?,
    )?;

    // mc.set_food(player_name, food)
    mc.set(
        "set_food",
        lua.create_function(|lua, (player_name, food): (String, i32)| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions
                    .push(LuaAction::SetPlayerFood { player_name, food });
            }
            Ok(())
        })?,
    )?;

    // mc.teleport(player_name, x, y, z)
    mc.set(
        "teleport",
        lua.create_function(|lua, (player_name, x, y, z): (String, f32, f32, f32)| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::TeleportPlayer {
                    player_name,
                    x,
                    y,
                    z,
                });
            }
            Ok(())
        })?,
    )?;

    // mc.get_time()
    mc.set(
        "get_time",
        lua.create_function(|lua, ()| {
            let time = lua
                .app_data_ref::<LuaHostData>()
                .map(|d| d.cached_time)
                .unwrap_or(0);
            Ok(time)
        })?,
    )?;

    // mc.set_time(time)
    mc.set(
        "set_time",
        lua.create_function(|lua, time: i64| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::SetTime { time });
            }
            Ok(())
        })?,
    )?;

    // mc.is_raining()
    mc.set(
        "is_raining",
        lua.create_function(|lua, ()| {
            let raining = lua
                .app_data_ref::<LuaHostData>()
                .map(|d| d.cached_is_raining)
                .unwrap_or(false);
            Ok(raining)
        })?,
    )?;

    // mc.get_tick()
    mc.set(
        "get_tick",
        lua.create_function(|lua, ()| {
            let tick = lua
                .app_data_ref::<LuaHostData>()
                .map(|d| d.cached_tick)
                .unwrap_or(0);
            Ok(tick)
        })?,
    )?;

    // mc.online_players() -> table of player tables
    mc.set(
        "online_players",
        lua.create_function(|lua, ()| {
            let players: Vec<PluginPlayer> = lua
                .app_data_ref::<LuaHostData>()
                .map(|d| d.cached_players.clone())
                .unwrap_or_default();
            let result = lua.create_table()?;
            for (i, p) in players.iter().enumerate() {
                let pt = lua.create_table()?;
                pt.set("name", p.name.as_str())?;
                pt.set("uuid", p.uuid.as_str())?;
                pt.set("runtime_id", p.runtime_id)?;
                pt.set("x", p.position.0)?;
                pt.set("y", p.position.1)?;
                pt.set("z", p.position.2)?;
                pt.set("gamemode", p.gamemode)?;
                pt.set("health", p.health)?;
                result.set(i + 1, pt)?;
            }
            Ok(result)
        })?,
    )?;

    // mc.get_player(name) -> player table or nil
    mc.set(
        "get_player",
        lua.create_function(|lua, name: String| {
            let player: Option<PluginPlayer> = lua
                .app_data_ref::<LuaHostData>()
                .and_then(|d| d.cached_players.iter().find(|p| p.name == name).cloned());
            match player {
                Some(p) => {
                    let pt = lua.create_table()?;
                    pt.set("name", p.name.as_str())?;
                    pt.set("uuid", p.uuid.as_str())?;
                    pt.set("runtime_id", p.runtime_id)?;
                    pt.set("x", p.position.0)?;
                    pt.set("y", p.position.1)?;
                    pt.set("z", p.position.2)?;
                    pt.set("gamemode", p.gamemode)?;
                    pt.set("health", p.health)?;
                    Ok(LuaValue::Table(pt))
                }
                None => Ok(LuaValue::Nil),
            }
        })?,
    )?;

    // mc.spawn_mob(mob_type, x, y, z)
    mc.set(
        "spawn_mob",
        lua.create_function(|lua, (mob_type, x, y, z): (String, f32, f32, f32)| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::SpawnMob { mob_type, x, y, z });
            }
            Ok(())
        })?,
    )?;

    // mc.remove_mob(runtime_id)
    mc.set(
        "remove_mob",
        lua.create_function(|lua, runtime_id: u64| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::RemoveMob { runtime_id });
            }
            Ok(())
        })?,
    )?;

    // mc.log(msg), mc.log_warn(msg), mc.log_error(msg), mc.log_debug(msg)
    mc.set(
        "log",
        lua.create_function(|lua, msg: String| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                let prefixed = format!("[{}] {}", data.plugin_name, msg);
                data.actions.push(LuaAction::Log {
                    level: LogLevel::Info,
                    message: prefixed,
                });
            }
            Ok(())
        })?,
    )?;
    mc.set(
        "log_warn",
        lua.create_function(|lua, msg: String| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                let prefixed = format!("[{}] {}", data.plugin_name, msg);
                data.actions.push(LuaAction::Log {
                    level: LogLevel::Warn,
                    message: prefixed,
                });
            }
            Ok(())
        })?,
    )?;
    mc.set(
        "log_error",
        lua.create_function(|lua, msg: String| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                let prefixed = format!("[{}] {}", data.plugin_name, msg);
                data.actions.push(LuaAction::Log {
                    level: LogLevel::Error,
                    message: prefixed,
                });
            }
            Ok(())
        })?,
    )?;
    mc.set(
        "log_debug",
        lua.create_function(|lua, msg: String| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                let prefixed = format!("[{}] {}", data.plugin_name, msg);
                data.actions.push(LuaAction::Log {
                    level: LogLevel::Debug,
                    message: prefixed,
                });
            }
            Ok(())
        })?,
    )?;

    // mc.register_command(name, description, handler)
    mc.set(
        "register_command",
        lua.create_function(
            |lua, (name, description, handler): (String, String, LuaFunction)| {
                let cmd_handlers: LuaTable = lua.named_registry_value("__command_handlers")?;
                cmd_handlers.set(name.as_str(), handler)?;
                if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                    data.actions
                        .push(LuaAction::RegisterCommand { name, description });
                }
                Ok(())
            },
        )?,
    )?;

    // mc.schedule(delay_ticks, callback) -> task_id
    mc.set(
        "schedule",
        lua.create_function(|lua, (delay_ticks, handler): (u64, LuaFunction)| {
            let task_id = lua
                .app_data_mut::<LuaHostData>()
                .map(|mut d| {
                    let id = d.alloc_task_id();
                    d.actions.push(LuaAction::ScheduleDelayed {
                        delay_ticks,
                        task_id: id,
                    });
                    id
                })
                .unwrap_or(0);
            let task_handlers: LuaTable = lua.named_registry_value("__task_handlers")?;
            task_handlers.set(task_id, handler)?;
            Ok(task_id)
        })?,
    )?;

    // mc.schedule_repeating(delay_ticks, interval_ticks, callback) -> task_id
    mc.set(
        "schedule_repeating",
        lua.create_function(
            |lua, (delay_ticks, interval_ticks, handler): (u64, u64, LuaFunction)| {
                let task_id = lua
                    .app_data_mut::<LuaHostData>()
                    .map(|mut d| {
                        let id = d.alloc_task_id();
                        d.actions.push(LuaAction::ScheduleRepeating {
                            delay_ticks,
                            interval_ticks,
                            task_id: id,
                        });
                        id
                    })
                    .unwrap_or(0);
                let task_handlers: LuaTable = lua.named_registry_value("__task_handlers")?;
                task_handlers.set(task_id, handler)?;
                Ok(task_id)
            },
        )?,
    )?;

    // mc.cancel_task(task_id)
    mc.set(
        "cancel_task",
        lua.create_function(|lua, task_id: u32| {
            if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
                data.actions.push(LuaAction::CancelTask { task_id });
            }
            let task_handlers: LuaTable = lua.named_registry_value("__task_handlers")?;
            task_handlers.set(task_id, LuaValue::Nil)?;
            Ok(())
        })?,
    )?;

    lua.globals().set("mc", mc)?;
    Ok(())
}

// ─── Plugin loading ──────────────────────────────────────────────────────────

/// Load a single Lua plugin from a directory containing plugin.toml.
fn load_single_plugin(dir: &Path) -> Result<LuaPlugin, String> {
    let manifest_path = dir.join("plugin.toml");
    let toml_content =
        std::fs::read_to_string(&manifest_path).map_err(|e| format!("read plugin.toml: {e}"))?;
    let manifest =
        LuaPluginManifest::parse(&toml_content).map_err(|e| format!("parse plugin.toml: {e}"))?;

    let script_path = dir.join(&manifest.main);
    if !script_path.exists() {
        return Err(format!(
            "main script '{}' not found in {}",
            manifest.main,
            dir.display()
        ));
    }

    let lua = Lua::new();
    setup_sandbox(&lua, manifest.memory_mb).map_err(|e| format!("sandbox: {e}"))?;

    lua.set_app_data(LuaHostData::new(manifest.name.clone()));

    setup_mc_api(&lua).map_err(|e| format!("mc api: {e}"))?;

    Ok(LuaPlugin {
        manifest,
        lua,
        script_path,
    })
}

/// Scan a directory for Lua plugin subdirectories and load all valid plugins.
pub fn load_lua_plugins(plugins_dir: &Path) -> Vec<Box<dyn Plugin>> {
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
        // Skip directories without plugin.toml
        if !path.join("plugin.toml").exists() {
            continue;
        }
        // Skip WASM plugins (they have wasm_file in their manifest)
        let toml_content = match std::fs::read_to_string(path.join("plugin.toml")) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if toml_content.contains("wasm_file") {
            continue;
        }

        match load_single_plugin(&path) {
            Ok(plugin) => {
                info!(
                    "Loaded Lua plugin: {} v{}",
                    plugin.manifest.name, plugin.manifest.version
                );
                plugins.push(Box::new(plugin));
            }
            Err(e) => {
                error!("Failed to load Lua plugin from {}: {e}", path.display());
            }
        }
    }

    plugins
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a fresh Lua VM with sandbox + mc API for testing.
    fn test_lua() -> Lua {
        let lua = Lua::new();
        setup_sandbox(&lua, 16).unwrap();
        lua.set_app_data(LuaHostData::new("TestPlugin".to_string()));
        setup_mc_api(&lua).unwrap();
        lua
    }

    fn take_actions(lua: &Lua) -> Vec<LuaAction> {
        lua.app_data_mut::<LuaHostData>()
            .map(|mut d| std::mem::take(&mut d.actions))
            .unwrap_or_default()
    }

    // ── Sandbox tests ────────────────────────────────────────────────────

    #[test]
    fn sandbox_removes_dangerous_globals() {
        let lua = test_lua();
        let result: LuaValue = lua.load("return os").eval().unwrap();
        assert!(result.is_nil());
        let result: LuaValue = lua.load("return io").eval().unwrap();
        assert!(result.is_nil());
        let result: LuaValue = lua.load("return debug").eval().unwrap();
        assert!(result.is_nil());
        let result: LuaValue = lua.load("return loadfile").eval().unwrap();
        assert!(result.is_nil());
        let result: LuaValue = lua.load("return dofile").eval().unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn sandbox_safe_globals_available() {
        let lua = test_lua();
        let result: String = lua.load("return type(tostring)").eval().unwrap();
        assert_eq!(result, "function");
        let result: String = lua.load("return type(table)").eval().unwrap();
        assert_eq!(result, "table");
        let result: String = lua.load("return type(string)").eval().unwrap();
        assert_eq!(result, "table");
    }

    // ── mc.on + event dispatch ───────────────────────────────────────────

    #[test]
    fn mc_on_registers_handlers() {
        let lua = test_lua();
        lua.load(
            r#"
            mc.on("player_join", function(event)
                mc.broadcast("Welcome " .. event.player.name .. "!")
            end)
        "#,
        )
        .exec()
        .unwrap();

        let handlers: LuaTable = lua.named_registry_value("__event_handlers").unwrap();
        let list: LuaTable = handlers.get("player_join").unwrap();
        assert_eq!(list.len().unwrap(), 1);
    }

    #[test]
    fn event_dispatch_calls_handler() {
        let lua = test_lua();
        lua.load(
            r#"
            mc.on("player_join", function(event)
                mc.broadcast("Hello " .. event.player.name)
            end)
        "#,
        )
        .exec()
        .unwrap();

        let event = PluginEvent::PlayerJoin {
            player: PluginPlayer {
                name: "Alice".into(),
                uuid: "u1".into(),
                runtime_id: 1,
                position: (0.0, 65.0, 0.0),
                gamemode: 0,
                health: 20.0,
            },
        };
        let (event_name, event_table) = LuaPlugin::event_to_lua_table(&lua, &event).unwrap();
        assert_eq!(event_name, "player_join");

        let handlers: LuaTable = lua.named_registry_value("__event_handlers").unwrap();
        let list: LuaTable = handlers.get::<LuaTable>(event_name.as_str()).unwrap();
        let handler: LuaFunction = list.get(1).unwrap();
        handler.call::<()>(event_table).unwrap();

        let actions = take_actions(&lua);
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], LuaAction::BroadcastMessage { message } if message == "Hello Alice")
        );
    }

    #[test]
    fn event_cancel() {
        let lua = test_lua();
        lua.load(
            r#"
            mc.on("player_damage", function(event)
                event.cancelled = true
            end)
        "#,
        )
        .exec()
        .unwrap();

        let event = PluginEvent::PlayerDamage {
            player: PluginPlayer {
                name: "Bob".into(),
                uuid: "u2".into(),
                runtime_id: 2,
                position: (0.0, 65.0, 0.0),
                gamemode: 0,
                health: 20.0,
            },
            damage: 5.0,
            cause: DamageCause::Attack,
        };
        let (event_name, event_table) = LuaPlugin::event_to_lua_table(&lua, &event).unwrap();
        let handlers: LuaTable = lua.named_registry_value("__event_handlers").unwrap();
        let list: LuaTable = handlers.get::<LuaTable>(event_name.as_str()).unwrap();
        let handler: LuaFunction = list.get(1).unwrap();
        handler.call::<()>(event_table.clone()).unwrap();

        assert!(event_table.get::<bool>("cancelled").unwrap());
    }

    // ── mc API function tests ────────────────────────────────────────────

    #[test]
    fn mc_broadcast_queues_action() {
        let lua = test_lua();
        lua.load(r#"mc.broadcast("Hello world")"#).exec().unwrap();
        let actions = take_actions(&lua);
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], LuaAction::BroadcastMessage { message } if message == "Hello world")
        );
    }

    #[test]
    fn mc_send_message_queues_action() {
        let lua = test_lua();
        lua.load(r#"mc.send_message("Alice", "Hi there")"#)
            .exec()
            .unwrap();
        let actions = take_actions(&lua);
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], LuaAction::SendMessage { player_name, message } if player_name == "Alice" && message == "Hi there")
        );
    }

    #[test]
    fn mc_set_health_and_food() {
        let lua = test_lua();
        lua.load(
            r#"
            mc.set_health("Alice", 10.5)
            mc.set_food("Alice", 15)
        "#,
        )
        .exec()
        .unwrap();
        let actions = take_actions(&lua);
        assert_eq!(actions.len(), 2);
        assert!(
            matches!(&actions[0], LuaAction::SetPlayerHealth { player_name, health } if player_name == "Alice" && (*health - 10.5).abs() < 0.01)
        );
        assert!(
            matches!(&actions[1], LuaAction::SetPlayerFood { player_name, food } if player_name == "Alice" && *food == 15)
        );
    }

    #[test]
    fn mc_get_set_time() {
        let lua = test_lua();
        if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
            data.cached_time = 6000;
        }

        let result: i64 = lua.load("return mc.get_time()").eval().unwrap();
        assert_eq!(result, 6000);

        lua.load("mc.set_time(12000)").exec().unwrap();
        let actions = take_actions(&lua);
        assert!(matches!(&actions[0], LuaAction::SetTime { time } if *time == 12000));
    }

    #[test]
    fn mc_is_raining_and_get_tick() {
        let lua = test_lua();
        if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
            data.cached_is_raining = true;
            data.cached_tick = 42;
        }

        let raining: bool = lua.load("return mc.is_raining()").eval().unwrap();
        assert!(raining);

        let tick: u64 = lua.load("return mc.get_tick()").eval().unwrap();
        assert_eq!(tick, 42);
    }

    #[test]
    fn mc_online_players_and_get_player() {
        let lua = test_lua();
        if let Some(mut data) = lua.app_data_mut::<LuaHostData>() {
            data.cached_players = vec![PluginPlayer {
                name: "Alice".into(),
                uuid: "u1".into(),
                runtime_id: 1,
                position: (10.0, 65.0, 20.0),
                gamemode: 0,
                health: 20.0,
            }];
        }

        let count: i64 = lua.load("return #mc.online_players()").eval().unwrap();
        assert_eq!(count, 1);

        let name: String = lua
            .load("return mc.online_players()[1].name")
            .eval()
            .unwrap();
        assert_eq!(name, "Alice");

        let name: String = lua
            .load("return mc.get_player('Alice').name")
            .eval()
            .unwrap();
        assert_eq!(name, "Alice");

        let result: LuaValue = lua.load("return mc.get_player('Bob')").eval().unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn mc_spawn_and_remove_mob() {
        let lua = test_lua();
        lua.load(
            r#"
            mc.spawn_mob("zombie", 10, 65, 20)
            mc.remove_mob(42)
        "#,
        )
        .exec()
        .unwrap();
        let actions = take_actions(&lua);
        assert_eq!(actions.len(), 2);
        assert!(
            matches!(&actions[0], LuaAction::SpawnMob { mob_type, .. } if mob_type == "zombie")
        );
        assert!(matches!(&actions[1], LuaAction::RemoveMob { runtime_id } if *runtime_id == 42));
    }

    #[test]
    fn mc_log_variants() {
        let lua = test_lua();
        lua.load(
            r#"
            mc.log("info msg")
            mc.log_warn("warn msg")
            mc.log_error("error msg")
            mc.log_debug("debug msg")
        "#,
        )
        .exec()
        .unwrap();
        let actions = take_actions(&lua);
        assert_eq!(actions.len(), 4);
        assert!(matches!(
            &actions[0],
            LuaAction::Log {
                level: LogLevel::Info,
                ..
            }
        ));
        assert!(matches!(
            &actions[1],
            LuaAction::Log {
                level: LogLevel::Warn,
                ..
            }
        ));
        assert!(matches!(
            &actions[2],
            LuaAction::Log {
                level: LogLevel::Error,
                ..
            }
        ));
        assert!(matches!(
            &actions[3],
            LuaAction::Log {
                level: LogLevel::Debug,
                ..
            }
        ));
    }

    // ── Commands ─────────────────────────────────────────────────────────

    #[test]
    fn mc_register_command_and_call() {
        let lua = test_lua();
        lua.load(
            r#"
            mc.register_command("greet", "Greet someone", function(sender, args)
                mc.send_message(sender, "Hello " .. (args[1] or "world"))
                return "Greeted!"
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Check registration action
        let actions = take_actions(&lua);
        assert!(matches!(&actions[0], LuaAction::RegisterCommand { name, .. } if name == "greet"));

        // Simulate on_command call
        let handlers: LuaTable = lua.named_registry_value("__command_handlers").unwrap();
        let handler: LuaFunction = handlers.get("greet").unwrap();
        let args_table = lua.create_sequence_from(vec!["Alice"]).unwrap();
        let result: String = handler.call(("Bob", args_table)).unwrap();
        assert_eq!(result, "Greeted!");

        let actions = take_actions(&lua);
        assert!(
            matches!(&actions[0], LuaAction::SendMessage { player_name, message } if player_name == "Bob" && message == "Hello Alice")
        );
    }

    // ── Scheduler ────────────────────────────────────────────────────────

    #[test]
    fn mc_schedule_and_task_call() {
        let lua = test_lua();
        lua.load(
            r#"
            local id = mc.schedule(20, function()
                mc.broadcast("Delayed!")
            end)
        "#,
        )
        .exec()
        .unwrap();

        let actions = take_actions(&lua);
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], LuaAction::ScheduleDelayed { delay_ticks, task_id } if *delay_ticks == 20 && *task_id == 1)
        );

        // Simulate on_task callback
        let handlers: LuaTable = lua.named_registry_value("__task_handlers").unwrap();
        let handler: LuaFunction = handlers.get(1u32).unwrap();
        handler.call::<()>(()).unwrap();

        let actions = take_actions(&lua);
        assert!(
            matches!(&actions[0], LuaAction::BroadcastMessage { message } if message == "Delayed!")
        );
    }

    #[test]
    fn mc_schedule_repeating_action() {
        let lua = test_lua();
        lua.load(
            r#"
            local id = mc.schedule_repeating(10, 100, function() end)
        "#,
        )
        .exec()
        .unwrap();

        let actions = take_actions(&lua);
        assert!(
            matches!(&actions[0], LuaAction::ScheduleRepeating { delay_ticks, interval_ticks, task_id } if *delay_ticks == 10 && *interval_ticks == 100 && *task_id == 1)
        );
    }

    #[test]
    fn mc_cancel_task_action() {
        let lua = test_lua();
        lua.load("mc.cancel_task(5)").exec().unwrap();
        let actions = take_actions(&lua);
        assert!(matches!(&actions[0], LuaAction::CancelTask { task_id } if *task_id == 5));
    }

    // ── Plugin loading ───────────────────────────────────────────────────

    #[test]
    fn load_plugins_empty_dir() {
        let dir = std::env::temp_dir().join("mc_rs_lua_test_empty");
        std::fs::create_dir_all(&dir).ok();
        let plugins = load_lua_plugins(&dir);
        assert!(plugins.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_plugins_nonexistent_dir() {
        let plugins = load_lua_plugins(Path::new("/nonexistent/path"));
        assert!(plugins.is_empty());
    }

    #[test]
    fn load_single_plugin_roundtrip() {
        let dir = std::env::temp_dir().join("mc_rs_lua_test_single");
        let plugin_dir = dir.join("my_plugin");
        std::fs::create_dir_all(&plugin_dir).ok();

        std::fs::write(
            plugin_dir.join("plugin.toml"),
            r#"
[plugin]
name = "TestLua"
version = "1.0.0"
author = "Dev"
description = "Test"
main = "main.lua"
"#,
        )
        .unwrap();

        std::fs::write(
            plugin_dir.join("main.lua"),
            r#"
mc.on("server_started", function()
    mc.log("TestLua started!")
end)
"#,
        )
        .unwrap();

        let plugin = load_single_plugin(&plugin_dir).unwrap();
        assert_eq!(plugin.manifest.name, "TestLua");
        assert_eq!(plugin.manifest.version, "1.0.0");

        std::fs::remove_dir_all(&dir).ok();
    }
}
