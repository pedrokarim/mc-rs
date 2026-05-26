//! Port de PMMP `src/command/defaults/DumpmemoryCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut dumpmemory = CommandDefinition::new("dumpmemory", "Show lightweight memory/debug info");
    dumpmemory.usage = "/dumpmemory".into();
    dumpmemory.permissions = vec!["server.command.dumpmemory".into()];
    register_command(
        permissions,
        map,
        dumpmemory,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!(
                "Debug snapshot: players={} itemEntities={} world={}",
                runtime.online_players(),
                runtime
                    .selector_entities()
                    .into_iter()
                    .filter(|entity| entity.entity_type == "item")
                    .count(),
                runtime.world_name()
            ));
            Ok(())
        },
    );
}
