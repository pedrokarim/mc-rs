//! Port de PMMP `src/command/defaults/OpCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    usage, CommandDefinition, CommandInvocation, CommandOverload, PermissionDefault,
    PermissionRegistry,
};

use super::{register_command, soft_player_param, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut op = CommandDefinition::new("op", "Grant operator status");
    op.usage = "/op <player>".into();
    op.permissions = vec!["server.command.op".into()];
    op.overloads.push(CommandOverload {
        parameters: vec![soft_player_param("player", false)],
    });
    register_command(
        permissions,
        map,
        op,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /op <player>");
            };
            runtime.op(name);
            runtime.send_feedback(&format!("{name} is now an operator."));
            Ok(())
        },
    );
}
