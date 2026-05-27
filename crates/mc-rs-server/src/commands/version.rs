//! Port de PMMP `src/command/defaults/VersionCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{CommandDefinition, PermissionDefault, PermissionRegistry};

use super::{register_command, ServerCommandMap, ServerCommandRuntime, SERVER_VERSION};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut version = CommandDefinition::new("version", "Show server version");
    version.usage = "/version".into();
    version.permissions = vec!["server.command.version".into()];
    register_command(
        permissions,
        map,
        version,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!(
                "{} | world={} | seed={}",
                SERVER_VERSION,
                runtime.world_name(),
                runtime.world_seed()
            ));
            Ok(())
        },
    );
}
