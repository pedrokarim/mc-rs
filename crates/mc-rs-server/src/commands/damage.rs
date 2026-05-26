//! Port de PMMP `src/command/defaults/DamageCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut damage = CommandDefinition::new("damage", "Damage an entity");
    damage.usage = "/damage <target> <amount> [cause]".into();
    damage.permissions = vec!["server.command.damage".into()];
    damage.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("amount", ParamType::Float, false),
            param("cause", ParamType::String, true),
        ],
    });
    register_command(
        permissions,
        map,
        damage,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /damage <target> <amount> [cause]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let amount: f32 = invocation
                .arg(1)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| CommandDispatchError::Message("Amount must be a number.".into()))?;
            if amount < 0.0 {
                return Err(CommandDispatchError::Message(
                    "Amount must be ≥ 0.".into(),
                ));
            }
            // Note : la `cause` est ignorée pour l'instant — combat::attack_entity
            // utilise toujours DamageCause::Custom pour /damage (les modifiers
            // armor/effect ne dépendent pas de cause atm).
            let mut died_count = 0;
            for addr in &targets {
                if runtime.damage_player(*addr, amount).unwrap_or(false) {
                    died_count += 1;
                }
            }
            runtime.send_feedback(&format!(
                "Damaged {} player(s) for {} HP{}.",
                targets.len(),
                amount,
                if died_count > 0 {
                    format!(" ({died_count} died)")
                } else {
                    String::new()
                }
            ));
            Ok(())
        },
    );
}
