//! Port de PMMP `src/command/defaults/TagCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut tag = CommandDefinition::new("tag", "Manage entity tags");
    tag.usage = "/tag <target> <add|remove|list> [<tag>]".into();
    tag.permissions = vec!["server.command.tag".into()];
    tag.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            hard_enum_param("action", "tag_action", &["add", "remove", "list"], false),
            param("tag", ParamType::String, true),
        ],
    });
    register_command(
        permissions,
        map,
        tag,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /tag <target> <add|remove|list> [<tag>]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let action = invocation.arg(1).unwrap().to_ascii_lowercase();
            match action.as_str() {
                "list" => {
                    // PMMP/vanilla : `list` n'accepte qu'un target unique.
                    if targets.len() != 1 {
                        return Err(CommandDispatchError::Message(
                            "Tag list requires exactly one target.".into(),
                        ));
                    }
                    let tags = runtime.player_tag_list(targets[0]);
                    let name = runtime
                        .player_name(targets[0])
                        .unwrap_or_else(|| "(player)".into());
                    if tags.is_empty() {
                        runtime.send_feedback(&format!("{name} has no tags."));
                    } else {
                        runtime.send_feedback(&format!(
                            "{name} has {} tag(s): {}",
                            tags.len(),
                            tags.join(", ")
                        ));
                    }
                }
                "add" | "remove" => {
                    let Some(tag_value) = invocation.arg(2) else {
                        return usage("Usage: /tag <target> <add|remove> <tag>");
                    };
                    // Validation vanilla : tag doit être [a-zA-Z0-9_.+-] non vide
                    // et ≤ 25 char. On garde proche de la limite vanilla.
                    if tag_value.is_empty() || tag_value.len() > 25 {
                        return Err(CommandDispatchError::Message(
                            "Tag must be 1-25 chars.".into(),
                        ));
                    }
                    let mut changed = 0usize;
                    for addr in &targets {
                        let ok = if action == "add" {
                            runtime.player_tag_add(*addr, tag_value)
                        } else {
                            runtime.player_tag_remove(*addr, tag_value)
                        };
                        if ok {
                            changed += 1;
                        }
                    }
                    let verb = if action == "add" { "Added" } else { "Removed" };
                    runtime.send_feedback(&format!(
                        "{verb} tag '{tag_value}' for {changed}/{} player(s).",
                        targets.len()
                    ));
                }
                _ => return usage("Action must be add, remove, or list."),
            }
            Ok(())
        },
    );
}
