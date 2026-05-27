//! Port de PMMP `src/command/defaults/PardonCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    usage, CommandDefinition, CommandInvocation, CommandOverload, PermissionDefault,
    PermissionRegistry,
};

use super::{register_command, soft_player_param, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut pardon = CommandDefinition::new("pardon", "Remove a player ban");
    pardon.usage = "/pardon <player>".into();
    pardon.permissions = vec!["server.command.pardon".into()];
    pardon.overloads.push(CommandOverload {
        parameters: vec![soft_player_param("player", false)],
    });
    register_command(
        permissions,
        map,
        pardon,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /pardon <player>");
            };
            runtime.pardon_name(name);
            runtime.send_feedback(&format!("Removed ban for {name}."));
            Ok(())
        },
    );
}
