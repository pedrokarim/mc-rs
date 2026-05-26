//! Port de PMMP `src/command/defaults/HelpCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut help = CommandDefinition::new("help", "Show available commands");
    help.usage = "/help".into();
    help.permissions = vec!["server.command.help".into()];
    register_command(
        permissions,
        map,
        help,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            let commands = runtime.visible_command_names();
            runtime.send_feedback(&format!(
                "Available commands ({}): {}",
                commands.len(),
                commands.join(", ")
            ));
            Ok(())
        },
    );
}
