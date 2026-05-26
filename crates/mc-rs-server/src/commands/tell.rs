//! Port de PMMP `src/command/defaults/TellCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    usage, CommandDefinition, CommandInvocation,
    CommandOverload, CommandParameter, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut tell = CommandDefinition::new("tell", "Send a private message");
    tell.aliases = vec!["msg".into(), "w".into()];
    tell.usage = "/tell <target> <message>".into();
    tell.permissions = vec!["server.command.tell".into()];
    tell.overloads.push(CommandOverload {
        parameters: vec![
            CommandParameter {
                name: "target".into(),
                param_type: ParamType::SoftEnum {
                    name: "online_players".into(),
                },
                optional: false,
            },
            CommandParameter {
                name: "message".into(),
                param_type: ParamType::Message,
                optional: false,
            },
        ],
    });
    register_command(
        permissions,
        map,
        tell,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /tell <target> <message>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let message_text = invocation.tail(1);
            for target in &targets {
                runtime.send_message(
                    *target,
                    &format!("[{} -> you] {}", runtime.sender_name(), message_text),
                );
            }
            runtime.send_feedback(&format!(
                "[you -> {}] {}",
                invocation.arg(0).unwrap_or("?"),
                message_text
            ));
            Ok(())
        },
    );
}
