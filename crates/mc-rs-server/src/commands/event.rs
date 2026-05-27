//! Port de PMMP `src/command/defaults/EventCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut event_cmd = CommandDefinition::new("event", "Trigger an entity event");
    event_cmd.usage = "/event entity <target> <event_id|hurt|death|eating|respawn>".into();
    event_cmd.permissions = vec!["server.command.event".into()];
    event_cmd.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("kind", "event_kind", &["entity"], false),
            param("target", ParamType::Target, false),
            param("event", ParamType::String, false),
        ],
    });
    register_command(
        permissions,
        map,
        event_cmd,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 3 {
                return usage("Usage: /event entity <target> <event_id|hurt|death|eating|respawn>");
            }
            if !invocation.arg(0).unwrap().eq_ignore_ascii_case("entity") {
                return usage("Only 'entity' kind is supported.");
            }
            let target_token = invocation.arg(1).unwrap();
            let event_arg = invocation.arg(2).unwrap();
            // Résolution event_id : numérique ou nom court PMMP actor_event::*
            let event_id: u32 = match event_arg.to_ascii_lowercase().as_str() {
                "hurt" | "hurt_animation" => crate::combat_packets::actor_event::HURT_ANIMATION,
                "death" | "death_animation" => crate::combat_packets::actor_event::DEATH_ANIMATION,
                "eating" | "eating_item" => crate::combat_packets::actor_event::EATING_ITEM,
                "respawn" => crate::combat_packets::actor_event::RESPAWN,
                "tame_success" => crate::combat_packets::actor_event::TAME_SUCCESS,
                "tame_fail" => crate::combat_packets::actor_event::TAME_FAIL,
                "shake_wet" => crate::combat_packets::actor_event::SHAKE_WET,
                "complete_trade" => crate::combat_packets::actor_event::COMPLETE_TRADE,
                other => other.parse::<u32>().map_err(|_| {
                    CommandDispatchError::Message(format!("Unknown event: {event_arg}"))
                })?,
            };

            let Some(rid) = runtime.first_entity_runtime_id(target_token) else {
                return Err(CommandDispatchError::Message("No matching entity.".into()));
            };
            runtime.actor_event_broadcast(rid, event_id, 0);
            runtime.send_feedback(&format!("Triggered event {event_id} on entity {rid}."));
            Ok(())
        },
    );
}
