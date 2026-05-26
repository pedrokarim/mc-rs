//! Port de PMMP `src/command/defaults/SayCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    usage, CommandDefinition, CommandInvocation,
    CommandOverload, CommandParameter, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut say = CommandDefinition::new("say", "Broadcast a server message");
    say.usage = "/say <message>".into();
    say.permissions = vec!["server.command.say".into()];
    say.overloads.push(CommandOverload {
        parameters: vec![CommandParameter {
            name: "message".into(),
            param_type: ParamType::Message,
            optional: false,
        }],
    });
    register_command(
        permissions,
        map,
        say,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() {
                return usage("Usage: /say <message>");
            }
            runtime.broadcast_chat("Server", &invocation.raw_args);
            Ok(())
        },
    );
}
