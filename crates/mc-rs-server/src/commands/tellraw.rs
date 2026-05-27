//! Port de PMMP `src/command/defaults/TellrawCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandInvocation, CommandOverload, ParamType,
    PermissionDefault, PermissionRegistry,
};

use super::{register_command, resolve_player_targets, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut tellraw = CommandDefinition::new("tellraw", "Send a JSON rawtext message");
    tellraw.usage = "/tellraw <target> <json>".into();
    tellraw.permissions = vec!["server.command.tellraw".into()];
    tellraw.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("json", ParamType::Json, false),
        ],
    });
    register_command(
        permissions,
        map,
        tellraw,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /tellraw <target> <json>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            // Le JSON peut contenir des espaces, on reconstruit avec `tail(1)`
            // qui rassemble tout après le target.
            let json = invocation.tail(1);
            if json.is_empty() {
                return usage("Usage: /tellraw <target> <json>");
            }
            let payload = mc_rs_proto::packets::player::Text::json(&json);
            for addr in targets {
                runtime.tellraw_send(addr, &payload);
            }
            runtime.send_feedback("Tellraw sent.");
            Ok(())
        },
    );
}
