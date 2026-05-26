//! Port de PMMP `src/command/defaults/BanCommand.php` — voir
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
    let mut ban = CommandDefinition::new("ban", "Ban a player name");
    ban.usage = "/ban <player> [reason]".into();
    ban.permissions = vec!["server.command.ban".into()];
    ban.overloads.push(CommandOverload {
        parameters: vec![
            param("player", ParamType::Target, false),
            param("reason", ParamType::Message, true),
        ],
    });
    register_command(
        permissions,
        map,
        ban,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_token) = invocation.arg(0) else {
                return usage("Usage: /ban <player> [reason]");
            };
            let mut banned_names = Vec::new();
            if let Ok(targets) = resolve_player_targets(runtime, Some(target_token), true) {
                for target in targets {
                    if let Some(name) = runtime.player_name(target) {
                        runtime.ban_name(&name);
                        banned_names.push(name);
                    }
                }
            } else {
                runtime.ban_name(target_token);
                banned_names.push(target_token.to_string());
            }
            runtime.send_feedback(&format!("Banned: {}", banned_names.join(", ")));
            Ok(())
        },
    );
}
