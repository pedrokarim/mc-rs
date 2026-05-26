//! Port de PMMP `src/command/defaults/PardonipCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut pardon_ip = CommandDefinition::new("pardon-ip", "Remove an IP ban");
    pardon_ip.usage = "/pardon-ip <ip>".into();
    pardon_ip.permissions = vec!["server.command.pardonip".into()];
    pardon_ip.overloads.push(CommandOverload {
        parameters: vec![param("ip", ParamType::String, false)],
    });
    register_command(
        permissions,
        map,
        pardon_ip,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(ip) = invocation.arg(0) else {
                return usage("Usage: /pardon-ip <ip>");
            };
            runtime.pardon_ip(ip);
            runtime.send_feedback(&format!("Removed IP ban for {ip}."));
            Ok(())
        },
    );
}
