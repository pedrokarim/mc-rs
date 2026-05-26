//! Port de PMMP `src/command/defaults/PlaysoundCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, parse_position_triplet_for_source, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut playsound = CommandDefinition::new("playsound", "Play a sound for players");
    playsound.usage = "/playsound <sound> [target] [x] [y] [z] [volume] [pitch]".into();
    playsound.permissions = vec!["server.command.playsound".into()];
    playsound.overloads.push(CommandOverload {
        parameters: vec![
            param("sound", ParamType::String, false),
            param("target", ParamType::Target, true),
            param("x", ParamType::Position, true),
            param("y", ParamType::Position, true),
            param("z", ParamType::Position, true),
            param("volume", ParamType::Float, true),
            param("pitch", ParamType::Float, true),
        ],
    });
    register_command(
        permissions,
        map,
        playsound,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(sound) = invocation.arg(0) else {
                return usage("Usage: /playsound <sound> [target] [x y z] [volume] [pitch]");
            };
            let targets = if let Some(target_token) = invocation.arg(1) {
                resolve_player_targets(runtime, Some(target_token), true)?
            } else if let Some(addr) = runtime.sender_addr() {
                vec![addr]
            } else {
                return Err(CommandDispatchError::Message(
                    "Console must specify a target.".into(),
                ));
            };

            // Position : si non fournie, on prend la position du premier target
            // (ou celle du sender si plus simple). Fallback à origin (0,0,0)
            // si vraiment rien.
            let pos = if let (Some(x), Some(y), Some(z)) =
                (invocation.arg(2), invocation.arg(3), invocation.arg(4))
            {
                let origin = if runtime.sender_is_player() {
                    Some(runtime.sender_position())
                } else {
                    None
                };
                parse_position_triplet_for_source(runtime, origin, x, y, z)?
            } else if let Some(first) = targets.first() {
                runtime.player_position(*first).unwrap_or([0.0, 64.0, 0.0])
            } else {
                [0.0, 64.0, 0.0]
            };
            let volume = invocation
                .arg(5)
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.0);
            let pitch = invocation
                .arg(6)
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.0);

            // Bedrock attend les noms avec préfixe (`random.click`, `mob.cow.say`,
            // etc.) — on transmet brut.
            runtime.play_sound(&targets, sound, pos, volume, pitch);
            runtime.send_feedback(&format!(
                "Played {sound} for {} player(s).",
                targets.len()
            ));
            Ok(())
        },
    );
}
