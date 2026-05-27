//! Port de PMMP `src/command/defaults/StatusCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{CommandDefinition, PermissionDefault, PermissionRegistry};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut status = CommandDefinition::new("status", "Show basic server status");
    status.usage = "/status".into();
    status.permissions = vec!["server.command.status".into()];
    register_command(
        permissions,
        map,
        status,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!(
                "{} | players={}/{} | difficulty={} | defaultGamemode={} | autoSave={}",
                runtime.server_motd(),
                runtime.online_players(),
                runtime.max_players(),
                runtime.current_difficulty(),
                runtime.current_default_gamemode(),
                runtime.auto_save_enabled()
            ));
            Ok(())
        },
    );
}
