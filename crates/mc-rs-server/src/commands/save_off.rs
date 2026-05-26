//! Port de PMMP `src/command/defaults/SaveoffCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut save_off = CommandDefinition::new("save-off", "Disable auto-save");
    save_off.usage = "/save-off".into();
    save_off.permissions = vec!["server.command.save".into()];
    register_command(
        permissions,
        map,
        save_off,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.set_auto_save(false);
            runtime.send_feedback("Auto-save disabled.");
            Ok(())
        },
    );
}
