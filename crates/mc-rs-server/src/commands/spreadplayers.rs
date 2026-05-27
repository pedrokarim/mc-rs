//! Port de PMMP `src/command/defaults/SpreadplayersCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, parse_coord, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, resolve_player_targets, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut spread =
        CommandDefinition::new("spreadplayers", "Spread players around a center point");
    spread.usage = "/spreadplayers <cx> <cz> <spreadDist> <maxRange> <target>".into();
    spread.permissions = vec!["server.command.spreadplayers".into()];
    spread.overloads.push(CommandOverload {
        parameters: vec![
            param("cx", ParamType::Position, false),
            param("cz", ParamType::Position, false),
            param("spreadDist", ParamType::Float, false),
            param("maxRange", ParamType::Float, false),
            param("target", ParamType::Target, false),
        ],
    });
    register_command(
        permissions,
        map,
        spread,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 5 {
                return usage("Usage: /spreadplayers <cx> <cz> <spreadDist> <maxRange> <target>");
            }
            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            // cx, cz : utilise X et Z avec parse_coord (~). Y est fictif (ignoré).
            let cx_str = invocation.arg(0).unwrap();
            let cz_str = invocation.arg(1).unwrap();
            let center_x = parse_coord(cx_str, origin.map(|o| o[0]).unwrap_or(0.0))
                .ok_or_else(|| CommandDispatchError::Message("Invalid cx".into()))?;
            let center_z = parse_coord(cz_str, origin.map(|o| o[2]).unwrap_or(0.0))
                .ok_or_else(|| CommandDispatchError::Message("Invalid cz".into()))?;
            let spread_dist: f32 =
                invocation
                    .arg(2)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| {
                        CommandDispatchError::Message("spreadDist must be a number".into())
                    })?;
            let max_range: f32 = invocation
                .arg(3)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| CommandDispatchError::Message("maxRange must be a number".into()))?;
            if max_range <= 0.0 {
                return Err(CommandDispatchError::Message("maxRange must be > 0".into()));
            }
            let _ = spread_dist; // pas appliqué dans cette MVP — note doc.

            let targets = resolve_player_targets(runtime, invocation.arg(4), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let count = targets.len();
            for addr in targets {
                let dx = (runtime.random_index(1000) as f32 / 1000.0) * 2.0 * max_range - max_range;
                let dz = (runtime.random_index(1000) as f32 / 1000.0) * 2.0 * max_range - max_range;
                let tx = center_x + dx;
                let tz = center_z + dz;
                // Y : on prend une altitude raisonnable au-dessus du terrain.
                // 128 = safe height qui retombe en y=64 ground typique.
                runtime.teleport_player(addr, [tx, 128.0, tz]);
            }
            runtime.send_feedback(&format!(
                "Spread {count} player(s) around ({center_x:.0},{center_z:.0})."
            ));
            Ok(())
        },
    );
}
