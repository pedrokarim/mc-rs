//! Port de PMMP `src/command/defaults/SpawnpointCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    message, param, parse_position_triplet_for_source, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut spawnpoint = CommandDefinition::new("spawnpoint", "Set a player's respawn point");
    spawnpoint.usage = "/spawnpoint [player] [x y z]".into();
    spawnpoint.permissions = vec!["server.command.spawnpoint".into()];
    spawnpoint.overloads.push(CommandOverload::default());
    spawnpoint.overloads.push(CommandOverload {
        parameters: vec![param("player", ParamType::Target, false)],
    });
    spawnpoint.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    spawnpoint.overloads.push(CommandOverload {
        parameters: vec![
            param("player", ParamType::Target, false),
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        permissions,
        map,
        spawnpoint,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if !runtime.sender_is_player() && matches!(invocation.args.len(), 0 | 1 | 3) {
                return message(
                    "Console must specify a player target and absolute coordinates when needed. Usage: /spawnpoint <player> [x y z]",
                );
            }
            let sender = runtime.sender_addr();
            let sender_pos = sender.and_then(|addr| runtime.player_position(addr));

            let (targets, position) = match invocation.args.len() {
                0 => (
                    vec![sender.ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?],
                    sender_pos.ok_or_else(|| {
                        CommandDispatchError::Message("Sender position is unavailable.".to_string())
                    })?,
                ),
                1 => {
                    if invocation.arg(0).unwrap_or("").starts_with('~')
                        || invocation.arg(0).unwrap_or("").parse::<f32>().is_ok()
                    {
                        return usage("Usage: /spawnpoint [player] [x y z]");
                    }
                    (
                        resolve_player_targets(runtime, invocation.arg(0), true)?,
                        sender_pos.ok_or_else(|| {
                            CommandDispatchError::Message(
                                "Sender position is unavailable.".to_string(),
                            )
                        })?,
                    )
                }
                3 => (
                    vec![sender.ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?],
                    parse_position_triplet_for_source(
                        runtime,
                        sender_pos,
                        invocation.arg(0).unwrap_or(""),
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                    )?,
                ),
                4 => (
                    resolve_player_targets(runtime, invocation.arg(0), true)?,
                    parse_position_triplet_for_source(
                        runtime,
                        sender_pos,
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                        invocation.arg(3).unwrap_or(""),
                    )?,
                ),
                _ => return usage("Usage: /spawnpoint [player] [x y z]"),
            };

            let count = targets.len();
            for target in targets {
                runtime
                    .set_player_spawn(target, position)
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!("Updated spawnpoint for {count} player(s)."));
            Ok(())
        },
    );
}
