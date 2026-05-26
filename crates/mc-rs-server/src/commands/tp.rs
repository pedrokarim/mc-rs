//! Port de PMMP `src/command/defaults/TpCommand.php` — voir
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
    let mut teleport = CommandDefinition::new("tp", "Teleport players");
    teleport.aliases = vec!["teleport".into()];
    teleport.usage = "/tp [target] <destination|x y z>".into();
    teleport.permissions = vec!["server.command.tp".into()];
    teleport.overloads.push(CommandOverload {
        parameters: vec![param("destination", ParamType::Target, false)],
    });
    teleport.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("destination", ParamType::Target, false),
        ],
    });
    teleport.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    teleport.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        permissions,
        map,
        teleport,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            match invocation.args.len() {
                1 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify a player target. Usage: /tp <target> <destination>",
                        );
                    }
                    let sender = runtime.sender_addr().ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?;
                    let destination = resolve_player_targets(runtime, invocation.arg(0), false)?[0];
                    let position = runtime.player_position(destination).ok_or_else(|| {
                        CommandDispatchError::Message(
                            "Destination player is unavailable.".to_string(),
                        )
                    })?;
                    runtime.teleport_player(sender, position);
                    runtime.send_feedback("Teleported.");
                }
                2 => {
                    let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
                    let destination = resolve_player_targets(runtime, invocation.arg(1), false)?[0];
                    let position = runtime.player_position(destination).ok_or_else(|| {
                        CommandDispatchError::Message(
                            "Destination player is unavailable.".to_string(),
                        )
                    })?;
                    let count = targets.len();
                    for target in targets {
                        runtime.teleport_player(target, position);
                    }
                    runtime.send_feedback(&format!("Teleported {count} player(s)."));
                }
                3 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify a player target. Usage: /tp <target> <x> <y> <z>",
                        );
                    }
                    let sender = runtime.sender_addr().ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?;
                    let origin = runtime.player_position(sender).ok_or_else(|| {
                        CommandDispatchError::Message("Sender position is unavailable.".to_string())
                    })?;
                    let position = parse_position_triplet_for_source(
                        runtime,
                        Some(origin),
                        invocation.arg(0).unwrap_or(""),
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                    )?;
                    runtime.teleport_player(sender, position);
                    runtime.send_feedback("Teleported.");
                }
                4 => {
                    let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
                    let origin = runtime
                        .sender_addr()
                        .and_then(|addr| runtime.player_position(addr));
                    let position = parse_position_triplet_for_source(
                        runtime,
                        origin,
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                        invocation.arg(3).unwrap_or(""),
                    )?;
                    let count = targets.len();
                    for target in targets {
                        runtime.teleport_player(target, position);
                    }
                    runtime.send_feedback(&format!("Teleported {count} player(s)."));
                }
                _ => return usage("Usage: /tp [target] <destination|x y z>"),
            }
            Ok(())
        },
    );
}
