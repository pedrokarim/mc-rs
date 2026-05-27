//! Port de PMMP `src/command/defaults/SummonCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    message, param, parse_position_triplet_for_source, usage, CommandDefinition,
    CommandDispatchError, CommandInvocation, CommandOverload, CommandParameter, ParamType,
    PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut summon = CommandDefinition::new("summon", "Summon a basic mob entity");
    summon.usage = "/summon <entity> [x y z]".into();
    summon.permissions = vec!["server.command.summon".into()];
    let entity_param = || CommandParameter {
        name: "entityType".into(),
        param_type: ParamType::SoftEnum {
            name: "EntityType".into(),
        },
        optional: false,
    };
    summon.overloads.push(CommandOverload {
        parameters: vec![entity_param()],
    });
    summon.overloads.push(CommandOverload {
        parameters: vec![
            entity_param(),
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        permissions,
        map,
        summon,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(entity_name) = invocation.arg(0) else {
                return usage("Usage: /summon <entity> [x y z]");
            };

            let sender = runtime.sender_addr();
            let sender_pos = sender.and_then(|addr| runtime.player_position(addr));
            let position = match invocation.args.len() {
                1 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify absolute coordinates. Usage: /summon <entity> <x> <y> <z>",
                        );
                    }
                    let mut pos = sender_pos.ok_or_else(|| {
                        CommandDispatchError::Message("Sender position is unavailable.".to_string())
                    })?;
                    pos[1] += 1.0;
                    pos
                }
                4 => parse_position_triplet_for_source(
                    runtime,
                    sender_pos,
                    invocation.arg(1).unwrap_or(""),
                    invocation.arg(2).unwrap_or(""),
                    invocation.arg(3).unwrap_or(""),
                )?,
                _ => return usage("Usage: /summon <entity> [x y z]"),
            };

            let entity_id = runtime
                .spawn_mob(entity_name, position)
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback(&format!(
                "Summoned {entity_name} at {:.1} {:.1} {:.1} (entity_id={entity_id}).",
                position[0], position[1], position[2]
            ));
            Ok(())
        },
    );
}
