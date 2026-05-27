//! Port de PMMP `src/command/defaults/WhitelistCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, usage, CommandDefinition, CommandInvocation, CommandOverload,
    PermissionDefault, PermissionRegistry,
};

use super::{register_command, soft_player_param, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut whitelist = CommandDefinition::new("whitelist", "Manage the server whitelist");
    whitelist.usage = "/whitelist <on|off|list|add|remove> [player]".into();
    whitelist.permissions = vec!["server.command.whitelist".into()];
    whitelist.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "action",
            "whitelist_action",
            &["on", "off", "list", "add", "remove"],
            false,
        )],
    });
    whitelist.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "whitelist_mutation", &["add", "remove"], false),
            soft_player_param("player", false),
        ],
    });
    register_command(
        permissions,
        map,
        whitelist,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /whitelist <on|off|list|add|remove> [player]");
            };
            match action.to_ascii_lowercase().as_str() {
                "on" => {
                    runtime.set_whitelist_enabled(true);
                    runtime.send_feedback("Whitelist enabled.");
                }
                "off" => {
                    runtime.set_whitelist_enabled(false);
                    runtime.send_feedback("Whitelist disabled.");
                }
                "list" => {
                    let entries = runtime.whitelist_entries();
                    runtime.send_feedback(&format!(
                        "Whitelist {} ({}): {}",
                        if runtime.whitelist_enabled() {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        entries.len(),
                        if entries.is_empty() {
                            "empty".to_string()
                        } else {
                            entries.join(", ")
                        }
                    ));
                }
                "add" => {
                    let Some(name) = invocation.arg(1) else {
                        return usage("Usage: /whitelist add <player>");
                    };
                    runtime.whitelist_add(name);
                    runtime.send_feedback(&format!("Added {name} to the whitelist."));
                }
                "remove" => {
                    let Some(name) = invocation.arg(1) else {
                        return usage("Usage: /whitelist remove <player>");
                    };
                    runtime.whitelist_remove(name);
                    runtime.send_feedback(&format!("Removed {name} from the whitelist."));
                }
                _ => return usage("Usage: /whitelist <on|off|list|add|remove> [player]"),
            }
            Ok(())
        },
    );
}
