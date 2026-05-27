//! Port de PMMP `src/command/defaults/GamemodeCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, message, param, usage, CommandDefinition, CommandDispatchError,
    CommandInvocation, CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    parse_gamemode, register_command, resolve_player_targets, ServerCommandMap,
    ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut gamemode = CommandDefinition::new("gamemode", "Change a player's gamemode");
    gamemode.usage = "/gamemode <survival|creative|adventure|spectator> [player]".into();
    gamemode.permissions = vec!["server.command.gamemode".into()];
    gamemode.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param(
                "gamemode",
                "gamemode_values",
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
                false,
            ),
            param("player", ParamType::Target, true),
        ],
    });
    register_command(
        permissions,
        map,
        gamemode,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(mode_token) = invocation.arg(0) else {
                return usage("Usage: /gamemode <survival|creative|adventure|spectator> [player]");
            };
            let mode = parse_gamemode(mode_token).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown gamemode: {mode_token}"))
            })?;
            if invocation.arg(1).is_none() && !runtime.sender_is_player() {
                return message(
                    "Console must specify a player target. Usage: /gamemode <mode> <player>",
                );
            }
            let targets = resolve_player_targets(runtime, invocation.arg(1), true)?;
            let count = targets.len();
            for target in targets {
                runtime.set_player_gamemode(target, mode);
            }
            runtime.send_feedback(&format!("Updated gamemode for {count} player(s)."));
            Ok(())
        },
    );
}
