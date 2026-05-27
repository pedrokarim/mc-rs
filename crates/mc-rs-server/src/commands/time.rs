//! Port de PMMP `src/command/defaults/TimeCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{parse_time_value, register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut time = CommandDefinition::new("time", "Control world time");
    time.usage = "/time <set|add|query> [value]".into();
    time.permissions = vec!["server.command.time".into()];
    // Hard enum (restrictif) : set+keyword affiche un dropdown (day/noon/…).
    time.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "time_set_action", &["set"], false),
            hard_enum_param(
                "value",
                "time_value",
                &[
                    "day", "noon", "midday", "sunset", "dusk", "night", "midnight", "sunrise",
                ],
                false,
            ),
        ],
    });
    // Overload générique : set/add/query avec une valeur numérique optionnelle.
    time.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "time_action", &["set", "add", "query"], false),
            param("ticks", ParamType::Int, true),
        ],
    });
    register_command(
        permissions,
        map,
        time,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /time <set|add|query> [value]");
            };
            match action.to_ascii_lowercase().as_str() {
                "set" => {
                    let Some(value_token) = invocation.arg(1) else {
                        return usage("Usage: /time set <value>");
                    };
                    let value = parse_time_value(value_token).ok_or_else(|| {
                        CommandDispatchError::Message(format!("Invalid time value: {value_token}"))
                    })?;
                    runtime.set_time(value);
                    runtime.send_feedback(&format!("Set time to {value}."));
                }
                "add" => {
                    let Some(value_token) = invocation.arg(1) else {
                        return usage("Usage: /time add <ticks>");
                    };
                    let delta = value_token.parse::<i32>().map_err(|_| {
                        CommandDispatchError::Message(format!("Invalid tick amount: {value_token}"))
                    })?;
                    let new_time = runtime.current_time().saturating_add(delta);
                    runtime.set_time(new_time);
                    runtime.send_feedback(&format!("Advanced time to {new_time}."));
                }
                "query" => {
                    runtime.send_feedback(&format!("Current time: {}", runtime.current_time()));
                }
                _ => return usage("Usage: /time <set|add|query> [value]"),
            }
            Ok(())
        },
    );
}
