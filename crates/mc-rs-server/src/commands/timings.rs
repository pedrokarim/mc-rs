//! Port de PMMP `src/command/defaults/TimingsCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{CommandDefinition, PermissionDefault, PermissionRegistry};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut timings = CommandDefinition::new("timings", "Show lightweight timings status");
    timings.usage = "/timings".into();
    timings.permissions = vec!["server.command.timings".into()];
    register_command(
        permissions,
        map,
        timings,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback("Timings/profiling is not wired yet. Use tracing logs for now.");
            Ok(())
        },
    );
}
