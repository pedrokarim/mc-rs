//! Port de PMMP `src/command/defaults/BanlistCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, usage, CommandDefinition, CommandInvocation,
    CommandOverload, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut banlist = CommandDefinition::new("banlist", "Show current bans");
    banlist.usage = "/banlist [players|ips|all]".into();
    banlist.permissions = vec!["server.command.banlist".into()];
    banlist.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "type",
            "banlist_type",
            &["players", "ips", "all"],
            true,
        )],
    });
    register_command(
        permissions,
        map,
        banlist,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let selection = invocation.arg(0).unwrap_or("all").to_ascii_lowercase();
            let names = runtime.banned_names();
            let ips = runtime.banned_ips();
            match selection.as_str() {
                "players" => runtime.send_feedback(&format!(
                    "Banned players ({}): {}",
                    names.len(),
                    if names.is_empty() {
                        "none".to_string()
                    } else {
                        names.join(", ")
                    }
                )),
                "ips" => runtime.send_feedback(&format!(
                    "Banned IPs ({}): {}",
                    ips.len(),
                    if ips.is_empty() {
                        "none".to_string()
                    } else {
                        ips.join(", ")
                    }
                )),
                "all" => runtime.send_feedback(&format!(
                    "Banned players ({}): {} | Banned IPs ({}): {}",
                    names.len(),
                    if names.is_empty() {
                        "none".to_string()
                    } else {
                        names.join(", ")
                    },
                    ips.len(),
                    if ips.is_empty() {
                        "none".to_string()
                    } else {
                        ips.join(", ")
                    }
                )),
                _ => return usage("Usage: /banlist [players|ips|all]"),
            }
            Ok(())
        },
    );
}
