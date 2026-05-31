//! Port de PMMP `src/command/defaults/LocateCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut locate = CommandDefinition::new("locate", "Locate the nearest structure or biome");
    locate.usage = "/locate <biome|structure> <name>".into();
    locate.permissions = vec!["server.command.locate".into()];
    // `/locate biome <name>` ou `/locate structure <name>`.
    locate.overloads.push(CommandOverload {
        parameters: vec![
            param("biome|structure", ParamType::String, false),
            param("name", ParamType::String, true),
        ],
    });
    // Rétro-compat : `/locate <structure>`.
    locate.overloads.push(CommandOverload {
        parameters: vec![param("structure", ParamType::String, false)],
    });
    register_command(
        permissions,
        map,
        locate,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(a0) = invocation.arg(0) else {
                return usage("Usage: /locate <biome|structure> <name>");
            };
            if !runtime.sender_is_player() {
                return Err(CommandDispatchError::Message(
                    "Console must be in a player context for /locate.".into(),
                ));
            }
            let origin = runtime.sender_position();
            match a0 {
                "biome" => {
                    let Some(name) = invocation.arg(1) else {
                        return usage("Usage: /locate biome <biome>");
                    };
                    locate_biome(runtime, origin, name)
                }
                // `/locate structure <name>` ou rétro-compat `/locate <name>`.
                "structure" => {
                    let Some(name) = invocation.arg(1) else {
                        return usage("Usage: /locate structure <structure>");
                    };
                    locate_structure(runtime, origin, name)
                }
                name => locate_structure(runtime, origin, name),
            }
        },
    );
}

fn locate_structure(
    runtime: &mut dyn ServerCommandRuntime,
    origin: [f32; 3],
    name: &str,
) -> Result<(), CommandDispatchError> {
    let Some(kind) = crate::structures::StructureKind::parse(name) else {
        return Err(CommandDispatchError::Message(format!(
            "Unknown structure: {name}"
        )));
    };
    let pos = crate::structures::locate_nearest(kind, origin);
    let dx = pos[0] as f32 - origin[0];
    let dz = pos[2] as f32 - origin[2];
    let dist = (dx * dx + dz * dz).sqrt();
    runtime.send_feedback(&format!(
        "Nearest {} estimated at ({}, ~{}, {}) — {} blocks away (approximation grid).",
        kind.identifier(),
        pos[0],
        pos[1],
        pos[2],
        dist as i32,
    ));
    Ok(())
}

fn locate_biome(
    runtime: &mut dyn ServerCommandRuntime,
    origin: [f32; 3],
    name: &str,
) -> Result<(), CommandDispatchError> {
    let (ox, oy, oz) = (origin[0] as i32, origin[1] as i32, origin[2] as i32);
    let seed = runtime.world_seed();
    match crate::world::worldgen::noise_chunk::locate_biome(seed, ox, oz, oy, name) {
        Some((x, z)) => {
            let (dx, dz) = ((x - ox) as f32, (z - oz) as f32);
            let dist = (dx * dx + dz * dz).sqrt() as i32;
            runtime.send_feedback(&format!(
                "Nearest minecraft:{} at ({}, ~{}, {}) — {} blocks away.",
                name.strip_prefix("minecraft:").unwrap_or(name),
                x,
                oy,
                z,
                dist,
            ));
            Ok(())
        }
        None => Err(CommandDispatchError::Message(format!(
            "Could not find biome '{name}' within 6400 blocks (unknown biome name?)."
        ))),
    }
}
