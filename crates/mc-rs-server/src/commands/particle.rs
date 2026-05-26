//! Port de PMMP `src/command/defaults/ParticleCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut particle = CommandDefinition::new("particle", "Spawn a particle effect");
    particle.usage = "/particle <name> [x] [y] [z]".into();
    particle.permissions = vec!["server.command.particle".into()];
    particle.overloads.push(CommandOverload {
        parameters: vec![
            param("name", ParamType::String, false),
            param("x", ParamType::Float, true),
            param("y", ParamType::Float, true),
            param("z", ParamType::Float, true),
        ],
    });
    register_command(
        permissions,
        map,
        particle,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name_tok) = invocation.arg(0) else {
                return usage("Usage: /particle <name> [x] [y] [z]");
            };
            // Si pas de coords explicites, prend la position du sender.
            let sender_pos = runtime.sender_position();
            let x: f32 = invocation
                .arg(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(sender_pos[0]);
            let y: f32 = invocation
                .arg(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(sender_pos[1]);
            let z: f32 = invocation
                .arg(3)
                .and_then(|s| s.parse().ok())
                .unwrap_or(sender_pos[2]);
            let pname = if name_tok.contains(':') {
                name_tok.to_string()
            } else {
                format!("minecraft:{name_tok}")
            };
            runtime.spawn_particle([x, y, z], &pname);
            runtime.send_feedback(&format!(
                "Spawned particle {pname} at ({x:.1},{y:.1},{z:.1})"
            ));
            Ok(())
        },
    );
}
