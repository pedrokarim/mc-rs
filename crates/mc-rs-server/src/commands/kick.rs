//! Port de PMMP `src/command/defaults/KickCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut kick = CommandDefinition::new("kick", "Kick one or more players");
    kick.usage = "/kick <target> [reason]".into();
    kick.permissions = vec!["server.command.kick".into()];
    kick.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("reason", ParamType::Message, true),
        ],
    });
    register_command(
        permissions,
        map,
        kick,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() {
                return usage("Usage: /kick <target> [reason]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let reason = if invocation.args.len() > 1 {
                invocation.tail(1)
            } else {
                "Kicked from the server.".to_string()
            };
            let count = targets.len();
            for target in targets {
                runtime.kick(target, &reason);
            }
            runtime.send_feedback(&format!("Kicked {count} player(s)."));
            Ok(())
        },
    );
}
