//! Port de PMMP `src/command/defaults/DifficultyCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    PermissionDefault, PermissionRegistry,
};

use super::{parse_difficulty, register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut difficulty = CommandDefinition::new("difficulty", "Show or change difficulty");
    difficulty.usage = "/difficulty [peaceful|easy|normal|hard]".into();
    difficulty.permissions = vec!["server.command.difficulty".into()];
    difficulty.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "difficulty",
            "difficulty_values",
            &[
                "peaceful", "easy", "normal", "hard", "p", "e", "n", "h", "0", "1", "2", "3",
            ],
            true,
        )],
    });
    register_command(
        permissions,
        map,
        difficulty,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                runtime.send_feedback(&format!(
                    "Current difficulty: {}",
                    runtime.current_difficulty()
                ));
                return Ok(());
            };
            let difficulty = parse_difficulty(token).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown difficulty: {token}"))
            })?;
            runtime.set_difficulty(difficulty);
            runtime.send_feedback(&format!("Difficulty set to {difficulty}."));
            Ok(())
        },
    );
}
