//! Port de PMMP `src/command/defaults/DeopCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    usage, CommandDefinition, CommandInvocation, CommandOverload, PermissionDefault,
    PermissionRegistry,
};

use super::{register_command, soft_player_param, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut deop = CommandDefinition::new("deop", "Revoke operator status");
    deop.usage = "/deop <player>".into();
    deop.permissions = vec!["server.command.deop".into()];
    deop.overloads.push(CommandOverload {
        parameters: vec![soft_player_param("player", false)],
    });
    register_command(
        permissions,
        map,
        deop,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /deop <player>");
            };
            runtime.deop(name);
            runtime.send_feedback(&format!("{name} is no longer an operator."));
            Ok(())
        },
    );
}
