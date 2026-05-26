//! Port de PMMP `src/command/defaults/PluginsCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut plugins = CommandDefinition::new("plugins", "List loaded plugins");
    plugins.usage = "/plugins".into();
    plugins.permissions = vec!["server.command.plugins".into()];
    register_command(
        permissions,
        map,
        plugins,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            let plugin_names = runtime.plugin_names();
            if plugin_names.is_empty() {
                runtime.send_feedback("Plugins: none loaded");
            } else {
                runtime.send_feedback(&format!(
                    "Plugins ({}): {}",
                    plugin_names.len(),
                    plugin_names.join(", ")
                ));
            }
            Ok(())
        },
    );
}
