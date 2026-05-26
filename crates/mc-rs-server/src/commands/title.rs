//! Port de PMMP `src/command/defaults/TitleCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets, send_title_to_targets,
    ServerCommandMap, ServerCommandRuntime, TitlePacketAction,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut title = CommandDefinition::new("title", "Send Bedrock title packets");
    title.usage = "/title <target> <clear|reset|title|subtitle|actionbar|times> [...]".into();
    title.permissions = vec!["server.command.title".into()];
    title.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            hard_enum_param(
                "action",
                "title_action",
                &["clear", "reset", "title", "subtitle", "actionbar", "times"],
                false,
            ),
            param("value", ParamType::Message, true),
        ],
    });
    register_command(
        permissions,
        map,
        title,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage(
                    "Usage: /title <target> <clear|reset|title|subtitle|actionbar|times> [...]",
                );
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            match invocation
                .arg(1)
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "clear" => {
                    send_title_to_targets(runtime, &targets, TitlePacketAction::Clear);
                    runtime.send_feedback("Cleared titles.");
                }
                "reset" => {
                    send_title_to_targets(runtime, &targets, TitlePacketAction::Reset);
                    runtime.send_feedback("Reset titles.");
                }
                "title" => {
                    if invocation.args.len() < 3 {
                        return usage("Usage: /title <target> title <text>");
                    }
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Title(invocation.tail(2)),
                    );
                    runtime.send_feedback("Sent title.");
                }
                "subtitle" => {
                    if invocation.args.len() < 3 {
                        return usage("Usage: /title <target> subtitle <text>");
                    }
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Subtitle(invocation.tail(2)),
                    );
                    runtime.send_feedback("Sent subtitle.");
                }
                "actionbar" => {
                    if invocation.args.len() < 3 {
                        return usage("Usage: /title <target> actionbar <text>");
                    }
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Actionbar(invocation.tail(2)),
                    );
                    runtime.send_feedback("Sent actionbar.");
                }
                "times" => {
                    if invocation.args.len() != 5 {
                        return usage("Usage: /title <target> times <fadeIn> <stay> <fadeOut>");
                    }
                    let fade_in = invocation
                        .arg(2)
                        .unwrap_or("")
                        .parse::<i32>()
                        .map_err(|_| {
                            CommandDispatchError::Message("Invalid fadeIn value.".to_string())
                        })?;
                    let stay = invocation
                        .arg(3)
                        .unwrap_or("")
                        .parse::<i32>()
                        .map_err(|_| {
                            CommandDispatchError::Message("Invalid stay value.".to_string())
                        })?;
                    let fade_out =
                        invocation
                            .arg(4)
                            .unwrap_or("")
                            .parse::<i32>()
                            .map_err(|_| {
                                CommandDispatchError::Message("Invalid fadeOut value.".to_string())
                            })?;
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Times {
                            fade_in,
                            stay,
                            fade_out,
                        },
                    );
                    runtime.send_feedback("Updated title timings.");
                }
                _ => {
                    return usage(
                        "Usage: /title <target> <clear|reset|title|subtitle|actionbar|times> [...]",
                    )
                }
            }
            Ok(())
        },
    );
}
