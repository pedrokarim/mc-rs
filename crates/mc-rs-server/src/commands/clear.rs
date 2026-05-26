//! Port de PMMP `src/command/defaults/ClearCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    message, param, CommandDefinition, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut clear = CommandDefinition::new("clear", "Clear player inventories");
    clear.usage = "/clear [target]".into();
    clear.permissions = vec!["server.command.clear".into()];
    clear.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::Target, true)],
    });
    register_command(
        permissions,
        map,
        clear,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.arg(0).is_none() && !runtime.sender_is_player() {
                return message("Console must specify a player target. Usage: /clear <target>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let count = targets.len();
            for target in targets {
                runtime.clear_inventory(target);
            }
            runtime.send_feedback(&format!("Cleared inventory for {count} player(s)."));
            Ok(())
        },
    );
}
