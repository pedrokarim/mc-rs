//! Port de PMMP `src/command/defaults/TestforblockCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, parse_position_triplet_for_source, usage, CommandDefinition, CommandDispatchError,
    CommandInvocation, CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut testforblock = CommandDefinition::new("testforblock", "Test the block at a position");
    testforblock.usage = "/testforblock <x> <y> <z> <block>".into();
    testforblock.permissions = vec!["server.command.testforblock".into()];
    testforblock.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
            param("block", ParamType::String, false),
        ],
    });
    register_command(
        permissions,
        map,
        testforblock,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 4 {
                return usage("Usage: /testforblock <x> <y> <z> <block>");
            }
            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            let pos = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(0).unwrap(),
                invocation.arg(1).unwrap(),
                invocation.arg(2).unwrap(),
            )?;
            let (ix, iy, iz) = (
                pos[0].floor() as i32,
                pos[1].floor() as i32,
                pos[2].floor() as i32,
            );
            let expected_name = invocation.arg(3).unwrap();
            let Some(expected_id) = runtime.resolve_block_name(expected_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown block: {expected_name}"
                )));
            };
            let actual_id = runtime.world_block_at(ix, iy, iz);
            if actual_id == expected_id {
                runtime.send_feedback(&format!(
                    "Block at ({ix},{iy},{iz}) matches {expected_name}."
                ));
            } else {
                runtime.send_feedback(&format!(
                    "Block at ({ix},{iy},{iz}) does NOT match {expected_name} (got id={actual_id})."
                ));
            }
            Ok(())
        },
    );
}
