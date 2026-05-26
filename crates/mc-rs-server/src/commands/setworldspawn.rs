//! Port de PMMP `src/command/defaults/SetworldspawnCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    message, param, parse_position_triplet_for_source, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut setworldspawn = CommandDefinition::new("setworldspawn", "Set the world spawn");
    setworldspawn.usage = "/setworldspawn [x y z]".into();
    setworldspawn.permissions = vec!["server.command.setworldspawn".into()];
    setworldspawn.overloads.push(CommandOverload::default());
    setworldspawn.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        permissions,
        map,
        setworldspawn,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() && !runtime.sender_is_player() {
                return message(
                    "Console must specify coordinates. Usage: /setworldspawn <x> <y> <z>",
                );
            }
            let sender = runtime.sender_addr();
            let sender_pos = sender.and_then(|addr| runtime.player_position(addr));
            let position = match invocation.args.len() {
                0 => sender_pos.ok_or_else(|| {
                    CommandDispatchError::Message(
                        "This command requires an in-game sender.".to_string(),
                    )
                })?,
                3 => parse_position_triplet_for_source(
                    runtime,
                    sender_pos,
                    invocation.arg(0).unwrap_or(""),
                    invocation.arg(1).unwrap_or(""),
                    invocation.arg(2).unwrap_or(""),
                )?,
                _ => return usage("Usage: /setworldspawn [x y z]"),
            };
            runtime.set_world_spawn(position);
            runtime.send_feedback("World spawn updated.");
            Ok(())
        },
    );
}
