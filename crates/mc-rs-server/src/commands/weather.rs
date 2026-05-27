//! Port de PMMP `src/command/defaults/WeatherCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, usage, CommandDefinition, CommandInvocation, CommandOverload,
    PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut weather = CommandDefinition::new("weather", "Control world weather");
    weather.usage = "/weather <clear|rain|thunder>".into();
    weather.permissions = vec!["server.command.weather".into()];
    weather.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "state",
            "weather_state",
            &["clear", "rain", "thunder"],
            false,
        )],
    });
    register_command(
        permissions,
        map,
        weather,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(state) = invocation.arg(0) else {
                return usage("Usage: /weather <clear|rain|thunder>");
            };
            match state.to_ascii_lowercase().as_str() {
                "clear" => {
                    runtime.set_weather(false, false);
                    runtime.send_feedback("Weather set to clear.");
                }
                "rain" => {
                    runtime.set_weather(true, false);
                    runtime.send_feedback("Weather set to rain.");
                }
                "thunder" => {
                    runtime.set_weather(true, true);
                    runtime.send_feedback("Weather set to thunder.");
                }
                _ => return usage("Usage: /weather <clear|rain|thunder>"),
            }
            Ok(())
        },
    );
}
