//! Port de PMMP `src/command/defaults/StopCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{CommandDefinition, PermissionDefault, PermissionRegistry};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut stop = CommandDefinition::new("stop", "Stop the server");
    stop.usage = "/stop".into();
    stop.permissions = vec!["server.command.stop".into()];
    register_command(
        permissions,
        map,
        stop,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback("Server shutting down...");
            runtime.stop_server();
            Ok(())
        },
    );
}
