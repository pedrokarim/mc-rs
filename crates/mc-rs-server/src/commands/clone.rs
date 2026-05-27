//! Port de PMMP `src/command/defaults/CloneCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, parse_position_triplet_for_source, usage, CommandDefinition,
    CommandDispatchError, CommandInvocation, CommandOverload, ParamType, PermissionDefault,
    PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut clone = CommandDefinition::new("clone", "Clone a region of blocks");
    clone.usage =
        "/clone <x1> <y1> <z1> <x2> <y2> <z2> <dx> <dy> <dz> [masked|replace] [force|move|normal]"
            .into();
    clone.permissions = vec!["server.command.clone".into()];
    clone.overloads.push(CommandOverload {
        parameters: vec![
            param("x1", ParamType::Position, false),
            param("y1", ParamType::Position, false),
            param("z1", ParamType::Position, false),
            param("x2", ParamType::Position, false),
            param("y2", ParamType::Position, false),
            param("z2", ParamType::Position, false),
            param("dx", ParamType::Position, false),
            param("dy", ParamType::Position, false),
            param("dz", ParamType::Position, false),
            hard_enum_param("mask_mode", "clone_mask", &["masked", "replace"], true),
            hard_enum_param(
                "clone_mode",
                "clone_collision",
                &["force", "move", "normal"],
                true,
            ),
        ],
    });
    register_command(
        permissions,
        map,
        clone,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 9 {
                return usage(
                    "Usage: /clone <x1> <y1> <z1> <x2> <y2> <z2> <dx> <dy> <dz> [masked|replace] [force|move|normal]",
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
            let dest = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(6).unwrap(),
                invocation.arg(7).unwrap(),
                invocation.arg(8).unwrap(),
            )?;
            let mask_mode = invocation
                .arg(9)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "replace".to_string());
            let clone_mode = invocation
                .arg(10)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "normal".to_string());

            let x1 = (from[0].floor() as i32).min(to[0].floor() as i32);
            let x2 = (from[0].floor() as i32).max(to[0].floor() as i32);
            let y1 = (from[1].floor() as i32).min(to[1].floor() as i32);
            let y2 = (from[1].floor() as i32).max(to[1].floor() as i32);
            let z1 = (from[2].floor() as i32).min(to[2].floor() as i32);
            let z2 = (from[2].floor() as i32).max(to[2].floor() as i32);
            let dx = dest[0].floor() as i32;
            let dy = dest[1].floor() as i32;
            let dz = dest[2].floor() as i32;

            let sx = x2 - x1 + 1;
            let sy = y2 - y1 + 1;
            let sz = z2 - z1 + 1;
            let volume = (sx as i64) * (sy as i64) * (sz as i64);
            if volume > 32_768 {
                return Err(CommandDispatchError::Message(format!(
                    "Region too large ({volume} blocks). Maximum is 32768."
                )));
            }

            // Lecture source first (au cas où source et dest se chevauchent).
            let mut buf: Vec<u32> = Vec::with_capacity(volume as usize);
            for y in 0..sy {
                for z in 0..sz {
                    for x in 0..sx {
                        buf.push(runtime.world_block_at(x1 + x, y1 + y, z1 + z));
                    }
                }
            }

            // En mode `move`, on doit aussi mémoriser les positions source pour
            // les remettre à air après écriture du dest.
            let air = crate::world::block_registry::BLOCKS.air;
            let mut changed: i64 = 0;
            let mut index = 0usize;
            for y in 0..sy {
                for z in 0..sz {
                    for x in 0..sx {
                        let block_id = buf[index];
                        index += 1;
                        if mask_mode == "masked" && block_id == air {
                            continue;
                        }
                        let tx = dx + x;
                        let ty = dy + y;
                        let tz = dz + z;
                        if runtime.set_world_block(tx, ty, tz, block_id) {
                            changed += 1;
                        }
                    }
                }
            }
            // Mode `move` : vider la source (sauf si overlap avec dest, géré
            // par le set séquentiel — le dest a déjà été écrit).
            if clone_mode == "move" {
                for y in 0..sy {
                    for z in 0..sz {
                        for x in 0..sx {
                            let sx_pos = x1 + x;
                            let sy_pos = y1 + y;
                            let sz_pos = z1 + z;
                            // Skip si la position source est dans le dest
                            // (sinon on effacerait ce qu'on vient de copier).
                            let in_dest = sx_pos >= dx
                                && sx_pos < dx + sx
                                && sy_pos >= dy
                                && sy_pos < dy + sy
                                && sz_pos >= dz
                                && sz_pos < dz + sz;
                            if !in_dest {
                                runtime.set_world_block(sx_pos, sy_pos, sz_pos, air);
                            }
                        }
                    }
                }
            }
            runtime.send_feedback(&format!("Cloned {changed} blocks."));
            Ok(())
        },
    );
}
