//! Port de PMMP `src/command/defaults/EffectCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    CommandParameter, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut effect = CommandDefinition::new("effect", "Apply a potion effect");
    effect.usage = "/effect <target> <effect> [duration] [amplifier]".into();
    effect.permissions = vec!["server.command.effect".into()];
    effect.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            CommandParameter {
                name: "effect".into(),
                param_type: ParamType::SoftEnum {
                    name: "Effect".into(),
                },
                optional: false,
            },
            param("duration", ParamType::Int, true),
            param("amplifier", ParamType::Int, true),
        ],
    });
    register_command(
        permissions,
        map,
        effect,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_name) = invocation.arg(0) else {
                return usage("Usage: /effect <target> <effect_name|id> [duration] [amplifier]");
            };
            let Some(effect_tok) = invocation.arg(1) else {
                return usage("Usage: /effect <target> <effect_name|id> [duration] [amplifier]");
            };
            // Accepte "minecraft:speed", "speed" ou un id numérique.
            let kind =
                crate::effects::EffectKind::from_name_or_id(effect_tok).ok_or_else(|| {
                    CommandDispatchError::Message(format!("Unknown effect: {effect_tok}"))
                })?;
            let effect_id: i32 = kind.id() as i32;
            let duration: i32 = invocation
                .arg(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(600);
            let amplifier: u8 = invocation.arg(3).and_then(|s| s.parse().ok()).unwrap_or(0);

            let entity_id = runtime
                .selector_entities()
                .into_iter()
                .find(|e| {
                    e.name
                        .as_deref()
                        .map(|n| n.eq_ignore_ascii_case(target_name))
                        .unwrap_or(false)
                })
                .map(|e| e.id);
            let addr = entity_id
                .and_then(|id| runtime.player_addr_by_entity(id))
                .ok_or_else(|| {
                    CommandDispatchError::Message(format!("Player not found: {target_name}"))
                })?;

            runtime
                .apply_player_effect(addr, effect_id, duration, amplifier)
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback(&format!(
                "Applied effect {effect_id} (duration={duration}, amplifier={amplifier})"
            ));
            Ok(())
        },
    );
}
