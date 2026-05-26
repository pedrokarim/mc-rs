//! Port de PMMP `src/command/defaults/ReplaceitemCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    parse_item_stack, register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut replaceitem =
        CommandDefinition::new("replaceitem", "Replace an item slot of an entity");
    replaceitem.usage =
        "/replaceitem entity <target> <slot_type> [slot] <item> [count]".into();
    replaceitem.permissions = vec!["server.command.replaceitem".into()];
    replaceitem.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("kind", "replaceitem_kind", &["entity"], false),
            param("target", ParamType::Target, false),
            param("slot_type", ParamType::String, false),
            param("slot_or_item", ParamType::String, false),
            param("item_or_count", ParamType::String, true),
            param("count", ParamType::Int, true),
        ],
    });
    register_command(
        permissions,
        map,
        replaceitem,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            // Forme attendue : /replaceitem entity <target> <slot_type> [slot] <item> [count]
            if invocation.args.len() < 4 {
                return usage(
                    "Usage: /replaceitem entity <target> <slot_type> [slot] <item> [count]",
                );
            }
            let kind = invocation.arg(0).unwrap();
            if !kind.eq_ignore_ascii_case("entity") {
                return usage("Only 'entity' kind is supported for /replaceitem.");
            }
            let target_token = invocation.arg(1).unwrap();
            let slot_type = invocation.arg(2).unwrap().to_ascii_lowercase();

            // Détermine si le slot_type prend un index numérique (hotbar/inventory)
            // ou non (armor.head/chest/legs/feet, weapon.mainhand/offhand).
            let needs_index =
                slot_type == "slot.hotbar" || slot_type == "slot.inventory";

            // Récupère index slot + item + count selon présence de l'index.
            let (slot_index_token, item_token, count_token): (Option<&str>, &str, Option<&str>) =
                if needs_index {
                    if invocation.args.len() < 5 {
                        return usage(
                            "slot.hotbar/slot.inventory require <slot> <item> [count]",
                        );
                    }
                    (
                        Some(invocation.arg(3).unwrap()),
                        invocation.arg(4).unwrap(),
                        invocation.arg(5),
                    )
                } else {
                    (
                        None,
                        invocation.arg(3).unwrap(),
                        invocation.arg(4),
                    )
                };

            // Résout slot_type + index → (InvKey, slot_index)
            let (inv_key, slot_index) = match slot_type.as_str() {
                "slot.weapon.mainhand" => (crate::inventory_manager::InvKey::Main, 0usize),
                "slot.weapon.offhand" => (crate::inventory_manager::InvKey::Offhand, 0),
                "slot.armor.head" => (crate::inventory_manager::InvKey::Armor, 0),
                "slot.armor.chest" => (crate::inventory_manager::InvKey::Armor, 1),
                "slot.armor.legs" => (crate::inventory_manager::InvKey::Armor, 2),
                "slot.armor.feet" => (crate::inventory_manager::InvKey::Armor, 3),
                "slot.hotbar" => {
                    let n: usize = slot_index_token.unwrap().parse().map_err(|_| {
                        CommandDispatchError::Message("Hotbar slot must be 0..8".into())
                    })?;
                    if n > 8 {
                        return Err(CommandDispatchError::Message(
                            "Hotbar slot must be 0..8".into(),
                        ));
                    }
                    (crate::inventory_manager::InvKey::Main, n)
                }
                "slot.inventory" => {
                    let n: usize = slot_index_token.unwrap().parse().map_err(|_| {
                        CommandDispatchError::Message("Inventory slot must be 0..26".into())
                    })?;
                    if n > 26 {
                        return Err(CommandDispatchError::Message(
                            "Inventory slot must be 0..26".into(),
                        ));
                    }
                    (crate::inventory_manager::InvKey::Main, 9 + n)
                }
                _ => {
                    return Err(CommandDispatchError::Message(format!(
                        "Unsupported slot_type '{slot_type}'"
                    )))
                }
            };

            let count: u16 = count_token
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            let stack = parse_item_stack(item_token, count)?;

            // /replaceitem doit cibler des joueurs en jeu — pas les mob entities
            // (qui n'ont pas d'InventoryManager).
            let targets = resolve_player_targets(runtime, Some(target_token), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let count = targets.len();
            for addr in targets {
                runtime
                    .replace_player_slot(addr, inv_key, slot_index, stack.clone())
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!("Replaced slot for {count} player(s)."));
            Ok(())
        },
    );
}
