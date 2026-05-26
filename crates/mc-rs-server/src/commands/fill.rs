//! Port de PMMP `src/command/defaults/FillCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, parse_position_triplet_for_source, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut fill = CommandDefinition::new("fill", "Fill a region with a block");
    fill.usage =
        "/fill <x1> <y1> <z1> <x2> <y2> <z2> <block> [destroy|hollow|keep|outline|replace]".into();
    fill.permissions = vec!["server.command.fill".into()];
    fill.overloads.push(CommandOverload {
        parameters: vec![
            param("x1", ParamType::Position, false),
            param("y1", ParamType::Position, false),
            param("z1", ParamType::Position, false),
            param("x2", ParamType::Position, false),
            param("y2", ParamType::Position, false),
            param("z2", ParamType::Position, false),
            param("block", ParamType::String, false),
            hard_enum_param(
                "mode",
                "fill_mode",
                &["destroy", "hollow", "keep", "outline", "replace"],
                true,
            ),
        ],
    });
    register_command(
        permissions,
        map,
        fill,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 7 {
                return usage(
                    "Usage: /fill <x1> <y1> <z1> <x2> <y2> <z2> <block> [destroy|hollow|keep|outline|replace]",
                );
            }
            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            let from = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(0).unwrap(),
                invocation.arg(1).unwrap(),
                invocation.arg(2).unwrap(),
            )?;
            let to = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(3).unwrap(),
                invocation.arg(4).unwrap(),
                invocation.arg(5).unwrap(),
            )?;
            let block_name = invocation.arg(6).unwrap();
            let mode = invocation
                .arg(7)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "replace".to_string());

            let Some(block_id) = runtime.resolve_block_name(block_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown block: {block_name}"
                )));
            };

            // Normalise les coins en (min, max). Vanilla : floor pour la position.
            let x1 = (from[0].floor() as i32).min(to[0].floor() as i32);
            let x2 = (from[0].floor() as i32).max(to[0].floor() as i32);
            let y1 = (from[1].floor() as i32).min(to[1].floor() as i32);
            let y2 = (from[1].floor() as i32).max(to[1].floor() as i32);
            let z1 = (from[2].floor() as i32).min(to[2].floor() as i32);
            let z2 = (from[2].floor() as i32).max(to[2].floor() as i32);

            // Vanilla limit : 32 768 blocs par /fill.
            let volume = ((x2 - x1 + 1) as i64)
                * ((y2 - y1 + 1) as i64)
                * ((z2 - z1 + 1) as i64);
            if volume > 32_768 {
                return Err(CommandDispatchError::Message(format!(
                    "Region too large ({volume} blocks). Maximum is 32768."
                )));
            }

            let air = crate::world::block_registry::BLOCKS.air;
            let mut changed: i64 = 0;
            for y in y1..=y2 {
                for z in z1..=z2 {
                    for x in x1..=x2 {
                        let is_border = x == x1
                            || x == x2
                            || y == y1
                            || y == y2
                            || z == z1
                            || z == z2;
                        let new_id = match mode.as_str() {
                            "keep" => {
                                if runtime.world_block_at(x, y, z) != air {
                                    continue;
                                }
                                block_id
                            }
                            "hollow" => {
                                if is_border {
                                    block_id
                                } else {
                                    air
                                }
                            }
                            "outline" => {
                                if is_border {
                                    block_id
                                } else {
                                    continue;
                                }
                            }
                            "destroy" | "replace" => block_id,
                            _ => {
                                return usage(
                                    "Mode must be destroy, hollow, keep, outline, or replace.",
                                )
                            }
                        };
                        if runtime.set_world_block(x, y, z, new_id) {
                            changed += 1;
                        }
                    }
                }
            }
            runtime.send_feedback(&format!("Filled {changed} blocks."));
            Ok(())
        },
    );
}
