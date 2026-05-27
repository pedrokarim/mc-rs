//! Port de PMMP `src/command/defaults/DefaultgamemodeCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    PermissionDefault, PermissionRegistry,
};

use super::{parse_gamemode, register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut default_gamemode = CommandDefinition::new(
        "defaultgamemode",
        "Show or change the default world gamemode",
    );
    default_gamemode.usage = "/defaultgamemode [mode]".into();
    default_gamemode.permissions = vec!["server.command.defaultgamemode".into()];
    default_gamemode.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "gamemode",
            "default_gamemode_values",
            &[
                "survival",
                "creative",
                "adventure",
                "spectator",
                "s",
                "c",
                "a",
                "sp",
                "0",
                "1",
                "2",
                "3",
            ],
            true,
        )],
    });
    register_command(
        permissions,
        map,
        default_gamemode,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                runtime.send_feedback(&format!(
                    "Current default gamemode: {}",
                    runtime.current_default_gamemode()
                ));
                return Ok(());
            };
            let gamemode = parse_gamemode(token).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown gamemode: {token}"))
            })?;
            runtime.set_default_gamemode(gamemode);
            runtime.send_feedback(&format!("Default gamemode set to {gamemode}."));
            Ok(())
        },
    );
}
