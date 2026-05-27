//! Port de PMMP `src/command/defaults/ScoreboardCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut sb = CommandDefinition::new("scoreboard", "Set a player score on a sidebar objective");
    sb.usage = "/scoreboard <objective> <player> <score>".into();
    sb.permissions = vec!["server.command.scoreboard".into()];
    sb.overloads.push(CommandOverload {
        parameters: vec![
            param("objective", ParamType::String, false),
            param("player", ParamType::String, false),
            param("score", ParamType::Int, false),
        ],
    });
    register_command(
        permissions,
        map,
        sb,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(obj) = invocation.arg(0) else {
                return usage("Usage: /scoreboard <objective> <player> <score>");
            };
            let Some(player) = invocation.arg(1) else {
                return usage("Usage: /scoreboard <objective> <player> <score>");
            };
            let score: i32 = invocation
                .arg(2)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| CommandDispatchError::Message("score must be an integer".into()))?;
            runtime.scoreboard_set(obj, player, score);
            runtime.send_feedback(&format!("Scoreboard {obj}: {player} = {score}"));
            Ok(())
        },
    );
}
