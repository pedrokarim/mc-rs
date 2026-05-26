//! Port de PMMP `src/command/defaults/SetblockCommand.php` — voir
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
    let mut setblock = CommandDefinition::new("setblock", "Set a block at a position");
    setblock.usage = "/setblock <x> <y> <z> <block> [destroy|keep|replace]".into();
    setblock.permissions = vec!["server.command.setblock".into()];
    setblock.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
            param("block", ParamType::String, false),
            hard_enum_param(
                "mode",
                "setblock_mode",
                &["destroy", "keep", "replace"],
                true,
            ),
        ],
    });
    register_command(
        permissions,
        map,
        setblock,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 4 {
                return usage("Usage: /setblock <x> <y> <z> <block> [destroy|keep|replace]");
            }
            let x_tok = invocation.arg(0).unwrap();
            let y_tok = invocation.arg(1).unwrap();
            let z_tok = invocation.arg(2).unwrap();
            let block_name = invocation.arg(3).unwrap();
            let mode = invocation
                .arg(4)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "replace".to_string());

            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            let pos = parse_position_triplet_for_source(runtime, origin, x_tok, y_tok, z_tok)?;
            let (ix, iy, iz) = (
                pos[0].floor() as i32,
                pos[1].floor() as i32,
                pos[2].floor() as i32,
            );

            let Some(block_id) = runtime.resolve_block_name(block_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown block: {block_name}"
                )));
            };

            // Modes — voir Minecraft Wiki /setblock :
            // - replace : remplace tout (default)
            // - keep : ne fait rien si le bloc existant est ≠ air
            // - destroy : remplace ET supprime l'ancien (sans drop pour l'instant)
            let air = crate::world::block_registry::BLOCKS.air;
            match mode.as_str() {
                "keep" => {
                    let current = runtime.world_block_at(ix, iy, iz);
                    if current != air {
                        runtime.send_feedback(&format!(
                            "Kept existing block at ({ix},{iy},{iz})."
                        ));
                        return Ok(());
                    }
                }
                "destroy" | "replace" => {}
                _ => return usage("Mode must be destroy, keep, or replace."),
            }

            if runtime.set_world_block(ix, iy, iz, block_id) {
                runtime.send_feedback(&format!("Block set at ({ix},{iy},{iz})."));
            } else {
                runtime.send_feedback(&format!("Block at ({ix},{iy},{iz}) unchanged."));
            }
            Ok(())
        },
    );
}
