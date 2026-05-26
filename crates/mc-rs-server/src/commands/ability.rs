//! Port de PMMP `src/command/defaults/AbilityCommand.php` — voir
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
    let mut ability_cmd = CommandDefinition::new("ability", "Toggle a player ability");
    ability_cmd.usage = "/ability <target> <mayfly|mute|worldbuilder|...> <true|false>".into();
    ability_cmd.permissions = vec!["server.command.ability".into()];
    ability_cmd.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("ability", ParamType::String, false),
            hard_enum_param("value", "ability_value", &["true", "false"], false),
        ],
    });
    register_command(
        permissions,
        map,
        ability_cmd,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 3 {
                return usage("Usage: /ability <target> <ability> <true|false>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let ability_name = invocation.arg(1).unwrap();
            let value = match invocation.arg(2).unwrap().to_ascii_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => true,
                "false" | "0" | "off" | "no" => false,
                _ => {
                    return Err(CommandDispatchError::Message(
                        "Value must be true or false.".into(),
                    ))
                }
            };
            let count = targets.len();
            for addr in targets {
                runtime
                    .set_player_ability(addr, ability_name, value)
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!(
                "Set {ability_name}={value} for {count} player(s)."
            ));
            Ok(())
        },
    );
}
