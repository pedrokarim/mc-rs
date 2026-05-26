//! Port de PMMP `src/command/defaults/TestforCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    param, usage, CommandDefinition, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut testfor = CommandDefinition::new("testfor", "Test for matching entities");
    testfor.usage = "/testfor <target>".into();
    testfor.permissions = vec!["server.command.testfor".into()];
    testfor.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::Target, false)],
    });
    register_command(
        permissions,
        map,
        testfor,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                return usage("Usage: /testfor <target>");
            };
            // resolve_player_targets renvoie une erreur si aucun match — on
            // veut juste le count, donc on attrape l'erreur.
            let count = match resolve_player_targets(runtime, Some(token), true) {
                Ok(targets) => targets.len(),
                Err(_) => 0,
            };
            if count > 0 {
                runtime.send_feedback(&format!("Found {count} matching player(s)."));
            } else {
                runtime.send_feedback("No matching player.");
            }
            Ok(())
        },
    );
}
