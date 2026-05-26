//! Port de PMMP `src/command/defaults/TransferserverCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    message, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{
    register_command, resolve_player_targets,
    ServerCommandMap, ServerCommandRuntime,
};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut transfer =
        CommandDefinition::new("transferserver", "Transfer players to another server");
    transfer.usage = "/transferserver [target] <host> <port>".into();
    transfer.permissions = vec!["server.command.transferserver".into()];
    transfer.overloads.push(CommandOverload {
        parameters: vec![
            param("host", ParamType::String, false),
            param("port", ParamType::Int, false),
        ],
    });
    transfer.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("host", ParamType::String, false),
            param("port", ParamType::Int, false),
        ],
    });
    register_command(
        permissions,
        map,
        transfer,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let (targets, host_token, port_token) = match invocation.args.len() {
                2 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify a player target. Usage: /transferserver <target> <host> <port>",
                        );
                    }
                    (
                        resolve_player_targets(runtime, None, true)?,
                        invocation.arg(0).unwrap_or(""),
                        invocation.arg(1).unwrap_or(""),
                    )
                }
                3 => (
                    resolve_player_targets(runtime, invocation.arg(0), true)?,
                    invocation.arg(1).unwrap_or(""),
                    invocation.arg(2).unwrap_or(""),
                ),
                _ => return usage("Usage: /transferserver [target] <host> <port>"),
            };
            let port = port_token.parse::<u16>().map_err(|_| {
                CommandDispatchError::Message(format!("Invalid port: {port_token}"))
            })?;
            let count = targets.len();
            for target in targets {
                runtime.transfer(target, host_token, port);
            }
            runtime.send_feedback(&format!(
                "Transferred {count} player(s) to {host_token}:{port}."
            ));
            Ok(())
        },
    );
}
