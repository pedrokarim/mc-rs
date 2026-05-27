//! Port de PMMP `src/command/defaults/GameruleCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload, ParamType,
    PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut gamerule = CommandDefinition::new("gamerule", "Show or change game rules");
    gamerule.usage = "/gamerule [<rule> [<value>]]".into();
    gamerule.permissions = vec!["server.command.gamerule".into()];
    gamerule.overloads.push(CommandOverload {
        parameters: vec![
            param("rule", ParamType::String, true),
            param("value", ParamType::String, true),
        ],
    });
    register_command(
        permissions,
        map,
        gamerule,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            // /gamerule (sans args) → liste toutes les rules
            if invocation.args.is_empty() {
                let rules = runtime.gamerule_list();
                if rules.is_empty() {
                    runtime.send_feedback("No game rules registered.");
                    return Ok(());
                }
                let names = rules
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                runtime.send_feedback(&format!("Game rules: {names}"));
                return Ok(());
            }

            let rule_name = invocation.arg(0).unwrap();

            // /gamerule <name> → affiche la valeur
            if invocation.args.len() == 1 {
                let Some(value) = runtime.gamerule_get(rule_name) else {
                    return Err(CommandDispatchError::Message(format!(
                        "Unknown game rule: {rule_name}"
                    )));
                };
                let display = match value {
                    crate::game_rules::GameRuleValue::Bool(b) => b.to_string(),
                    crate::game_rules::GameRuleValue::Int(i) => i.to_string(),
                    crate::game_rules::GameRuleValue::Float(f) => f.to_string(),
                };
                runtime.send_feedback(&format!("{rule_name} = {display}"));
                return Ok(());
            }

            // /gamerule <name> <value> → set
            let raw_value = invocation.arg(1).unwrap();
            let Some(existing) = runtime.gamerule_get(rule_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown game rule: {rule_name}"
                )));
            };
            let parsed = match existing {
                crate::game_rules::GameRuleValue::Bool(_) => match raw_value
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "true" | "1" | "on" | "yes" => crate::game_rules::GameRuleValue::Bool(true),
                    "false" | "0" | "off" | "no" => crate::game_rules::GameRuleValue::Bool(false),
                    _ => {
                        return Err(CommandDispatchError::Message(format!(
                            "Invalid bool value '{raw_value}', expected true/false."
                        )))
                    }
                },
                crate::game_rules::GameRuleValue::Int(_) => raw_value
                    .parse::<i32>()
                    .map(crate::game_rules::GameRuleValue::Int)
                    .map_err(|_| {
                        CommandDispatchError::Message(format!("Invalid int value '{raw_value}'."))
                    })?,
                crate::game_rules::GameRuleValue::Float(_) => raw_value
                    .parse::<f32>()
                    .map(crate::game_rules::GameRuleValue::Float)
                    .map_err(|_| {
                        CommandDispatchError::Message(format!("Invalid float value '{raw_value}'."))
                    })?,
            };
            runtime
                .gamerule_set(rule_name, parsed.clone())
                .map_err(CommandDispatchError::Message)?;
            let display = match parsed {
                crate::game_rules::GameRuleValue::Bool(b) => b.to_string(),
                crate::game_rules::GameRuleValue::Int(i) => i.to_string(),
                crate::game_rules::GameRuleValue::Float(f) => f.to_string(),
            };
            runtime.send_feedback(&format!("Game rule {rule_name} set to {display}"));
            Ok(())
        },
    );
}
