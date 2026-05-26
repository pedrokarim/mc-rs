//! Port de PMMP `src/command/defaults/GiveCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, CommandParameter, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    parse_item_stack, register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut give = CommandDefinition::new("give", "Give items to players");
    give.usage = "/give <target> <item> [count]".into();
    give.permissions = vec!["server.command.give".into()];
    give.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            CommandParameter {
                name: "itemName".into(),
                param_type: ParamType::SoftEnum {
                    name: "Item".into(),
                },
                optional: false,
            },
            param("count", ParamType::Int, true),
        ],
    });
    register_command(
        permissions,
        map,
        give,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /give <target> <item> [count]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let count = if let Some(count_token) = invocation.arg(2) {
                count_token.parse::<u16>().map_err(|_| {
                    CommandDispatchError::Message(format!("Invalid count: {count_token}"))
                })?
            } else {
                1
            };
            let item = parse_item_stack(invocation.arg(1).unwrap_or(""), count)?;
            let count_targets = targets.len();
            for target in targets {
                runtime
                    .give_item(target, item.clone())
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!("Gave item to {count_targets} player(s)."));
            Ok(())
        },
    );
}
