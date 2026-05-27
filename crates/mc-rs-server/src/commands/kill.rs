//! Port de PMMP `src/command/defaults/KillCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    message, param, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, resolve_entity_targets, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut kill = CommandDefinition::new("kill", "Kill players or remove entities");
    kill.usage = "/kill [target]".into();
    kill.permissions = vec!["server.command.kill".into()];
    kill.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::Target, true)],
    });
    register_command(
        permissions,
        map,
        kill,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                if !runtime.sender_is_player() {
                    return message("Console must specify a target. Usage: /kill <target>");
                }
                let sender = runtime.sender_addr().ok_or_else(|| {
                    CommandDispatchError::Message(
                        "This command requires an in-game sender.".to_string(),
                    )
                })?;
                runtime.kill_player(sender);
                runtime.send_feedback("You died.");
                return Ok(());
            };
            let targets = resolve_entity_targets(runtime, token)?;
            let count = targets.len();
            for (entity_id, player_addr) in targets {
                if let Some(addr) = player_addr {
                    runtime.kill_player(addr);
                } else {
                    runtime
                        .remove_entity(entity_id)
                        .map_err(CommandDispatchError::Message)?;
                }
            }
            runtime.send_feedback(&format!("Killed or removed {count} target(s)."));
            Ok(())
        },
    );
}
