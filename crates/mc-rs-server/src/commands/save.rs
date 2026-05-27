//! Port de PMMP `src/command/defaults/SaveCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, CommandDispatchError, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut save = CommandDefinition::new("save", "Save the world immediately");
    save.usage = "/save".into();
    save.permissions = vec!["server.command.save".into()];
    register_command(
        permissions,
        map,
        save,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime
                .save_world()
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback("World and server state saved.");
            Ok(())
        },
    );
}
