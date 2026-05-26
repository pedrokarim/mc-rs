//! Port de PMMP `src/command/defaults/LocateCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut locate = CommandDefinition::new("locate", "Locate the nearest structure");
    locate.usage = "/locate <structure>".into();
    locate.permissions = vec!["server.command.locate".into()];
    locate.overloads.push(CommandOverload {
        parameters: vec![param("structure", ParamType::String, false)],
    });
    register_command(
        permissions,
        map,
        locate,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /locate <structure>");
            };
            let Some(kind) = crate::structures::StructureKind::parse(name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown structure: {name}"
                )));
            };
            if !runtime.sender_is_player() {
                return Err(CommandDispatchError::Message(
                    "Console must be in a player context for /locate.".into(),
                ));
            }
            let origin = runtime.sender_position();
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
        },
    );
}
