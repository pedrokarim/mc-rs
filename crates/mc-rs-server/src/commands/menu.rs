//! Port de PMMP `src/command/defaults/MenuCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut menu = CommandDefinition::new("menu", "Open the hub menu");
    menu.usage = "/menu".into();
    menu.permissions = vec!["server.command.menu".into()];
    register_command(
        permissions,
        map,
        menu,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.open_sender_menu();
            Ok(())
        },
    );
}
