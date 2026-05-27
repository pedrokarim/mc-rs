//! Port de PMMP `src/command/defaults/ReloadCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, CommandDispatchError, CommandInvocation, PermissionDefault,
    PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut reload = CommandDefinition::new("reload", "Reload server state from disk");
    reload.usage = "/reload".into();
    reload.permissions = vec!["server.command.reload".into()];
    register_command(
        permissions,
        map,
        reload,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _invocation: &CommandInvocation| {
            runtime
                .reload_server_state()
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback("Reloaded ops/whitelist/bans from disk.");
            runtime.sync_available_commands_for_all();
            Ok(())
        },
    );
}
