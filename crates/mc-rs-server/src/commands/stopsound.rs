//! Port de PMMP `src/command/defaults/StopsoundCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut stopsound = CommandDefinition::new("stopsound", "Stop a sound for players");
    stopsound.usage = "/stopsound <target> [sound]".into();
    stopsound.permissions = vec!["server.command.stopsound".into()];
    stopsound.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("sound", ParamType::String, true),
        ],
    });
    register_command(
        permissions,
        map,
        stopsound,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_token) = invocation.arg(0) else {
                return usage("Usage: /stopsound <target> [sound]");
            };
            let targets = resolve_player_targets(runtime, Some(target_token), true)?;
            let sound = invocation.arg(1);
            runtime.stop_sound(&targets, sound);
            runtime.send_feedback(&format!(
                "Stopped sound{} for {} player(s).",
                if sound.is_some() { "" } else { "s" },
                targets.len()
            ));
            Ok(())
        },
    );
}
