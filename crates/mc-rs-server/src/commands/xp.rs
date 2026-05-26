//! Port de PMMP `src/command/defaults/XpCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut xp = CommandDefinition::new("xp", "Manage player experience");
    xp.usage = "/xp <add|set|query> [amount] [target]".into();
    xp.permissions = vec!["server.command.xp".into()];
    xp.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "xp_action", &["add", "set", "query"], false),
            param("amount", ParamType::Int, true),
            param("target", ParamType::Target, true),
        ],
    });
    register_command(
        permissions,
        map,
        xp,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /xp <add|set|query> [amount] [target]");
            };

            // Cible par défaut = self si possible.
            let addr = match invocation.arg(2) {
                Some(target_name) => {
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
                    entity_id.and_then(|id| runtime.player_addr_by_entity(id))
                }
                None => runtime.sender_addr(),
            }
            .ok_or_else(|| {
                CommandDispatchError::Message(
                    "No target player (specify a name or run as a player)".into(),
                )
            })?;

            match action.to_ascii_lowercase().as_str() {
                "add" => {
                    let Some(amount_tok) = invocation.arg(1) else {
                        return usage("Usage: /xp add <amount> [target]");
                    };
                    let amount: i32 = amount_tok.parse().map_err(|_| {
                        CommandDispatchError::Message(format!("Invalid amount: {amount_tok}"))
                    })?;
                    let new_level = runtime
                        .add_player_xp(addr, amount)
                        .map_err(CommandDispatchError::Message)?;
                    runtime.send_feedback(&format!("Added {amount} XP (level now {new_level})"));
                }
                "set" => {
                    // set = query current then add diff (pour récupérer diff
                    // il faudrait query_xp ; on simplifie en retirant tout puis
                    // réajoutant).
                    let Some(amount_tok) = invocation.arg(1) else {
                        return usage("Usage: /xp set <amount> [target]");
                    };
                    let amount: i32 = amount_tok.parse().map_err(|_| {
                        CommandDispatchError::Message(format!("Invalid amount: {amount_tok}"))
                    })?;
                    // Clear total puis ajouter — simple et correct pour une
                    // 1ère version.
                    let _ = runtime.add_player_xp(addr, i32::MIN / 2);
                    let level = runtime
                        .add_player_xp(addr, amount)
                        .map_err(CommandDispatchError::Message)?;
                    runtime.send_feedback(&format!("Set XP to {amount} (level {level})"));
                }
                "query" => {
                    let level = runtime
                        .add_player_xp(addr, 0)
                        .map_err(CommandDispatchError::Message)?;
                    runtime.send_feedback(&format!("Level: {level}"));
                }
                _ => return usage("Usage: /xp <add|set|query> [amount] [target]"),
            }
            Ok(())
        },
    );
}
