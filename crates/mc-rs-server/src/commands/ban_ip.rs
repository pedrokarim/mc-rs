//! Port de PMMP `src/command/defaults/BanipCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, resolve_player_targets, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut ban_ip = CommandDefinition::new("ban-ip", "Ban an IP address or online player IP");
    ban_ip.usage = "/ban-ip <ip|player>".into();
    ban_ip.permissions = vec!["server.command.banip".into()];
    ban_ip.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::String, false)],
    });
    register_command(
        permissions,
        map,
        ban_ip,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_token) = invocation.arg(0) else {
                return usage("Usage: /ban-ip <ip|player>");
            };
            if let Ok(targets) = resolve_player_targets(runtime, Some(target_token), false) {
                let target = targets[0];
                let ip = runtime.player_ip(target).ok_or_else(|| {
                    CommandDispatchError::Message("Player IP is unavailable.".to_string())
                })?;
                runtime.ban_ip(&ip);
                runtime.kick(target, "Your IP has been banned from this server.");
                runtime.send_feedback(&format!("Banned IP {ip}."));
            } else {
                runtime.ban_ip(target_token);
                runtime.send_feedback(&format!("Banned IP {target_token}."));
            }
            Ok(())
        },
    );
}
