//! Port de PMMP `src/command/defaults/MeCommand.php` — voir
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
    let mut me = CommandDefinition::new("me", "Broadcast an emote");
    me.usage = "/me <action>".into();
    me.permissions = vec!["server.command.me".into()];
    me.overloads.push(CommandOverload {
        parameters: vec![CommandParameter {
            name: "action".into(),
            param_type: ParamType::Message,
            optional: false,
        }],
    });
    register_command(
        permissions,
        map,
        me,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() {
                return usage("Usage: /me <action>");
            }
            let sender_name = runtime.sender_name().to_string();
            runtime.broadcast_action(&sender_name, &invocation.raw_args);
            Ok(())
        },
    );
}
