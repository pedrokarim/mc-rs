//! Port de PMMP `src/command/defaults/GcCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    CommandDefinition, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut gc = CommandDefinition::new("gc", "Explain Rust memory management");
    gc.usage = "/gc".into();
    gc.permissions = vec!["server.command.gc".into()];
    register_command(
        permissions,
        map,
        gc,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback("Rust has no manual GC cycle to trigger. Memory is reclaimed automatically when values are dropped.");
            Ok(())
        },
    );
}
