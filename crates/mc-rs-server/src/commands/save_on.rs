//! Port de PMMP `src/command/defaults/SaveonCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{CommandDefinition, PermissionDefault, PermissionRegistry};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut save_on = CommandDefinition::new("save-on", "Enable auto-save");
    save_on.usage = "/save-on".into();
    save_on.permissions = vec!["server.command.save".into()];
    register_command(
        permissions,
        map,
        save_on,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.set_auto_save(true);
            runtime.send_feedback("Auto-save enabled.");
            Ok(())
        },
    );
}
