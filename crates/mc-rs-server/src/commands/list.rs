//! Port de PMMP `src/command/defaults/ListCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut list = CommandDefinition::new("list", "List online players");
    list.usage = "/list".into();
    list.permissions = vec!["server.command.list".into()];
    register_command(
        permissions,
        map,
        list,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            let players = runtime.soft_enum_values("online_players");
            runtime.send_feedback(&format!(
                "Online players ({}): {}",
                players.len(),
                players.join(", ")
            ));
            Ok(())
        },
    );
}
