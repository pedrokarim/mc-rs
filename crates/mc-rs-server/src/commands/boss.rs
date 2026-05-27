//! Port de PMMP `src/command/defaults/BossCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandInvocation, CommandOverload,
    ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut boss = CommandDefinition::new("boss", "Manage a boss bar (server-wide)");
    boss.usage = "/boss <show|hide|title|health> [args]".into();
    boss.permissions = vec!["server.command.boss".into()];
    boss.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param(
                "action",
                "boss_action",
                &["show", "hide", "title", "health"],
                false,
            ),
            param("value", ParamType::Message, true),
        ],
    });
    register_command(
        permissions,
        map,
        boss,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /boss <show|hide|title|health> [args]");
            };
            match action {
                "show" => {
                    let title = invocation.arg(1).unwrap_or("Boss");
                    runtime.boss_show(title, 1.0);
                    runtime.send_feedback(&format!("Boss bar shown: {title}"));
                }
                "hide" => {
                    runtime.boss_hide();
                    runtime.send_feedback("Boss bar hidden");
                }
                "title" => {
                    let t = invocation.arg(1).unwrap_or("");
                    runtime.boss_set_title(t);
                    runtime.send_feedback(&format!("Boss title: {t}"));
                }
                "health" => {
                    let p: f32 = invocation
                        .arg(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                    runtime.boss_set_health(p.clamp(0.0, 1.0));
                    runtime.send_feedback(&format!("Boss health: {:.0}%", p * 100.0));
                }
                _ => return usage("Usage: /boss <show|hide|title|health> [args]"),
            }
            Ok(())
        },
    );
}
