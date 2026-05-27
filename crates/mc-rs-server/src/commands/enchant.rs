//! Port de PMMP `src/command/defaults/EnchantCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    CommandParameter, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut enchant = CommandDefinition::new("enchant", "Add enchantment to held item");
    enchant.usage = "/enchant <target> <enchantment> [level]".into();
    enchant.permissions = vec!["server.command.enchant".into()];
    enchant.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            CommandParameter {
                name: "enchantmentName".into(),
                param_type: ParamType::SoftEnum {
                    name: "Enchantment".into(),
                },
                optional: false,
            },
            param("level", ParamType::Int, true),
        ],
    });
    register_command(
        permissions,
        map,
        enchant,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_name) = invocation.arg(0) else {
                return usage("Usage: /enchant <target> <enchant_name|id> [level]");
            };
            let Some(ench_tok) = invocation.arg(1) else {
                return usage("Usage: /enchant <target> <enchant_name|id> [level]");
            };
            let kind = crate::enchantments::EnchantmentKind::from_name_or_id(ench_tok).ok_or_else(
                || CommandDispatchError::Message(format!("Unknown enchantment: {ench_tok}")),
            )?;
            let level: u8 = invocation.arg(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let max = kind.max_level();
            let level = level.min(max).max(1);

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
                .apply_held_enchant(addr, kind.id(), level)
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback(&format!(
                "Enchanted held item with {ench_tok} {level} (max {max})"
            ));
            Ok(())
        },
    );
}
