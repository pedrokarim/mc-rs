//! Port de PMMP `src/command/defaults/SeedCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut seed = CommandDefinition::new("seed", "Show the world seed");
    seed.usage = "/seed".into();
    seed.permissions = vec!["server.command.seed".into()];
    register_command(
        permissions,
        map,
        seed,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!("World seed: {}", runtime.world_seed()));
            Ok(())
        },
    );
}
