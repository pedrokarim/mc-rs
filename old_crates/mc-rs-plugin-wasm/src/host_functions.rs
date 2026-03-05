//! Host functions exposed to WASM plugin guests.
//!
//! These are registered on the wasmtime `Linker` and called by WASM code
//! via imports in the `"mcrs"` module.

use crate::WasmHostData;
use wasmtime::{Caller, Engine, Linker};

/// Helper: read a UTF-8 string from guest memory at `ptr..ptr+len`.
///
/// `Caller::get_export` requires `&mut self`, so this takes `&mut Caller`.
/// Callers should extract all needed strings before calling `caller.data_mut()`.
fn read_guest_string(caller: &mut Caller<'_, WasmHostData>, ptr: i32, len: i32) -> Option<String> {
    let memory = caller.get_export("memory")?.into_memory()?;
    let data = memory.data(&caller);
    let start = ptr as usize;
    let end = start + len as usize;
    if end > data.len() {
        return None;
    }
    String::from_utf8(data[start..end].to_vec()).ok()
}

/// Helper: write a length-prefixed string into guest memory via `__malloc`.
///
/// Layout: `[u32_le byte_length][utf8 bytes]`. Returns the guest pointer, or 0
/// on failure (missing `__malloc` export, OOM, etc.).
fn write_guest_string(caller: &mut Caller<'_, WasmHostData>, s: &str) -> i32 {
    let bytes = s.as_bytes();
    let total_len = 4 + bytes.len();

    // Resolve __malloc export (guest must export it).
    let malloc = match caller.get_export("__malloc") {
        Some(ext) => match ext.into_func() {
            Some(f) => f,
            None => return 0,
        },
        None => return 0,
    };
    let malloc = match malloc.typed::<i32, i32>(&caller) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let ptr = match malloc.call(&mut *caller, total_len as i32) {
        Ok(p) => p,
        Err(_) => return 0,
    };

    // Write [u32_le length][utf8 data] into guest memory.
    let memory = match caller.get_export("memory") {
        Some(ext) => match ext.into_memory() {
            Some(m) => m,
            None => return 0,
        },
        None => return 0,
    };
    let mem = memory.data_mut(&mut *caller);
    let p = ptr as usize;
    if p + total_len > mem.len() {
        return 0;
    }
    mem[p..p + 4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    mem[p + 4..p + 4 + bytes.len()].copy_from_slice(bytes);
    ptr
}

/// Build a [`Linker<WasmHostData>`] with all host functions registered under
/// the `"mcrs"` import module.
pub fn build_linker(engine: &Engine) -> Result<Linker<WasmHostData>, wasmtime::Error> {
    let mut linker = Linker::new(engine);

    // ── Player API (write) ──────────────────────────────────────

    linker.func_wrap(
        "mcrs",
        "send_message",
        |mut caller: Caller<'_, WasmHostData>,
         name_ptr: i32,
         name_len: i32,
         msg_ptr: i32,
         msg_len: i32| {
            let name = read_guest_string(&mut caller, name_ptr, name_len).unwrap_or_default();
            let message = read_guest_string(&mut caller, msg_ptr, msg_len).unwrap_or_default();
            caller.data_mut().send_message(name, message);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "broadcast_message",
        |mut caller: Caller<'_, WasmHostData>, ptr: i32, len: i32| {
            let message = read_guest_string(&mut caller, ptr, len).unwrap_or_default();
            caller.data_mut().broadcast_message(message);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "kick_player",
        |mut caller: Caller<'_, WasmHostData>,
         name_ptr: i32,
         name_len: i32,
         reason_ptr: i32,
         reason_len: i32| {
            let name = read_guest_string(&mut caller, name_ptr, name_len).unwrap_or_default();
            let reason = read_guest_string(&mut caller, reason_ptr, reason_len).unwrap_or_default();
            caller.data_mut().kick_player(name, reason);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "set_player_health",
        |mut caller: Caller<'_, WasmHostData>, name_ptr: i32, name_len: i32, health: f32| {
            let name = read_guest_string(&mut caller, name_ptr, name_len).unwrap_or_default();
            caller.data_mut().set_player_health(name, health);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "set_player_food",
        |mut caller: Caller<'_, WasmHostData>, name_ptr: i32, name_len: i32, food: i32| {
            let name = read_guest_string(&mut caller, name_ptr, name_len).unwrap_or_default();
            caller.data_mut().set_player_food(name, food);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "teleport_player",
        |mut caller: Caller<'_, WasmHostData>,
         name_ptr: i32,
         name_len: i32,
         x: f32,
         y: f32,
         z: f32| {
            let name = read_guest_string(&mut caller, name_ptr, name_len).unwrap_or_default();
            caller.data_mut().teleport_player(name, x, y, z);
        },
    )?;

    // ── Player API (read) ───────────────────────────────────────

    linker.func_wrap(
        "mcrs",
        "online_players",
        |mut caller: Caller<'_, WasmHostData>| -> i32 {
            let json = caller.data().cached_players_json.clone();
            write_guest_string(&mut caller, &json)
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "get_player",
        |mut caller: Caller<'_, WasmHostData>, name_ptr: i32, name_len: i32| -> i32 {
            let name = read_guest_string(&mut caller, name_ptr, name_len).unwrap_or_default();
            // Look up player in cached data.
            let players_json = caller.data().cached_players_json.clone();
            let players: Vec<serde_json::Value> =
                serde_json::from_str(&players_json).unwrap_or_default();
            let found = players
                .iter()
                .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(&name));
            match found {
                Some(player) => {
                    let json = serde_json::to_string(player).unwrap_or_default();
                    write_guest_string(&mut caller, &json)
                }
                None => 0,
            }
        },
    )?;

    // ── World API ───────────────────────────────────────────────

    linker.func_wrap(
        "mcrs",
        "get_time",
        |caller: Caller<'_, WasmHostData>| -> i64 { caller.data().cached_time },
    )?;

    linker.func_wrap(
        "mcrs",
        "set_time",
        |mut caller: Caller<'_, WasmHostData>, time: i64| {
            caller.data_mut().set_time(time);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "is_raining",
        |caller: Caller<'_, WasmHostData>| -> i32 {
            if caller.data().cached_is_raining {
                1
            } else {
                0
            }
        },
    )?;

    // ── Entity API ──────────────────────────────────────────────

    linker.func_wrap(
        "mcrs",
        "spawn_mob",
        |mut caller: Caller<'_, WasmHostData>,
         type_ptr: i32,
         type_len: i32,
         x: f32,
         y: f32,
         z: f32| {
            let mob_type = read_guest_string(&mut caller, type_ptr, type_len).unwrap_or_default();
            caller.data_mut().spawn_mob(mob_type, x, y, z);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "remove_mob",
        |mut caller: Caller<'_, WasmHostData>, runtime_id: i64| {
            caller.data_mut().remove_mob(runtime_id as u64);
        },
    )?;

    // ── Server API ──────────────────────────────────────────────

    linker.func_wrap(
        "mcrs",
        "get_tick",
        |caller: Caller<'_, WasmHostData>| -> i64 { caller.data().cached_tick as i64 },
    )?;

    linker.func_wrap(
        "mcrs",
        "log",
        |mut caller: Caller<'_, WasmHostData>, level: i32, ptr: i32, len: i32| {
            let message = read_guest_string(&mut caller, ptr, len).unwrap_or_default();
            let log_level = match level {
                0 => mc_rs_plugin_api::LogLevel::Info,
                1 => mc_rs_plugin_api::LogLevel::Warn,
                2 => mc_rs_plugin_api::LogLevel::Error,
                _ => mc_rs_plugin_api::LogLevel::Debug,
            };
            caller.data_mut().log(log_level, message);
        },
    )?;

    // ── Scheduler ───────────────────────────────────────────────

    linker.func_wrap(
        "mcrs",
        "schedule_delayed",
        |mut caller: Caller<'_, WasmHostData>, delay_ticks: i64, task_id: i32| {
            caller
                .data_mut()
                .schedule_delayed(delay_ticks as u64, task_id as u32);
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "schedule_repeating",
        |mut caller: Caller<'_, WasmHostData>,
         delay_ticks: i64,
         interval_ticks: i64,
         task_id: i32| {
            caller.data_mut().schedule_repeating(
                delay_ticks as u64,
                interval_ticks as u64,
                task_id as u32,
            );
        },
    )?;

    linker.func_wrap(
        "mcrs",
        "cancel_task",
        |mut caller: Caller<'_, WasmHostData>, task_id: i32| {
            caller.data_mut().cancel_task(task_id as u32);
        },
    )?;

    // ── Commands ────────────────────────────────────────────────

    linker.func_wrap(
        "mcrs",
        "register_command",
        |mut caller: Caller<'_, WasmHostData>,
         name_ptr: i32,
         name_len: i32,
         desc_ptr: i32,
         desc_len: i32| {
            let name = read_guest_string(&mut caller, name_ptr, name_len).unwrap_or_default();
            let description =
                read_guest_string(&mut caller, desc_ptr, desc_len).unwrap_or_default();
            caller.data_mut().register_command(name, description);
        },
    )?;

    Ok(linker)
}
