//! Port de PMMP `src/command/defaults/LootCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, parse_position_triplet_for_source, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    parse_item_stack, register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut loot = CommandDefinition::new("loot", "Drop or give loot from a loot table");
    loot.usage = "/loot spawn <x> <y> <z> loot <table>  OR  /loot give <target> loot <table>".into();
    loot.permissions = vec!["server.command.loot".into()];
    loot.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("mode", "loot_mode", &["spawn", "give"], false),
            param("target_or_x", ParamType::String, false),
            param("y_or_subcmd", ParamType::String, true),
            param("z_or_table", ParamType::String, true),
            param("subcmd", ParamType::String, true),
            param("table", ParamType::String, true),
        ],
    });
    register_command(
        permissions,
        map,
        loot,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(mode) = invocation.arg(0) else {
                return usage(
                    "Usage: /loot spawn <x> <y> <z> loot <table>  OR  /loot give <target> loot <table>",
                );
            };
            match mode.to_ascii_lowercase().as_str() {
                "spawn" => {
                    // /loot spawn x y z loot <table>
                    if invocation.args.len() < 6 {
                        return usage("Usage: /loot spawn <x> <y> <z> loot <table>");
                    }
                    let origin = if runtime.sender_is_player() {
                        Some(runtime.sender_position())
                    } else {
                        None
                    };
                    let pos = parse_position_triplet_for_source(
                        runtime,
                        origin,
                        invocation.arg(1).unwrap(),
                        invocation.arg(2).unwrap(),
                        invocation.arg(3).unwrap(),
                    )?;
                    let subcmd = invocation.arg(4).unwrap();
                    if !subcmd.eq_ignore_ascii_case("loot") {
                        return usage("Expected 'loot' before table name.");
                    }
                    let table = invocation.arg(5).unwrap();
                    let drops = runtime.roll_chest_loot_drops(table);
                    if drops.is_empty() {
                        return Err(CommandDispatchError::Message(format!(
                            "Empty or unknown loot table: {table}"
                        )));
                    }
                    let mut count_total = 0u32;
                    for (name, count) in drops {
                        let stack = match parse_item_stack(&name, count as u16) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                        runtime.spawn_item_world(pos, stack);
                        count_total += count;
                    }
                    runtime.send_feedback(&format!(
                        "Spawned {count_total} loot item(s) at ({:.0},{:.0},{:.0}).",
                        pos[0], pos[1], pos[2]
                    ));
                }
                "give" => {
                    // /loot give <target> loot <table>
                    if invocation.args.len() < 4 {
                        return usage("Usage: /loot give <target> loot <table>");
                    }
                    let target_token = invocation.arg(1).unwrap();
                    let subcmd = invocation.arg(2).unwrap();
                    if !subcmd.eq_ignore_ascii_case("loot") {
                        return usage("Expected 'loot' before table name.");
                    }
                    let table = invocation.arg(3).unwrap();
                    let drops = runtime.roll_chest_loot_drops(table);
                    if drops.is_empty() {
                        return Err(CommandDispatchError::Message(format!(
                            "Empty or unknown loot table: {table}"
                        )));
                    }
                    let targets = resolve_player_targets(runtime, Some(target_token), true)?;
                    if targets.is_empty() {
                        return Err(CommandDispatchError::Message("No matching player.".into()));
                    }
                    let mut count_total = 0u32;
                    for addr in &targets {
                        for (name, count) in &drops {
                            let stack = match parse_item_stack(name, *count as u16) {
                                Ok(s) => s,
                                Err(_) => continue,
                            };
                            let _ = runtime.give_item(*addr, stack);
                            count_total += count;
                        }
                    }
                    runtime.send_feedback(&format!(
                        "Gave {count_total} loot item(s) to {} player(s).",
                        targets.len()
                    ));
                }
                _ => return usage("Mode must be spawn or give."),
            }
            Ok(())
        },
    );
}
