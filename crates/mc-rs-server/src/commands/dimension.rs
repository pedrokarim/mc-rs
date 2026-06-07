//! `/dimension` — commande de **debug** pour transférer un joueur entre dimensions
//! sans passer par un portail (sert à valider le transfert + les générateurs avant
//! que les portails existent — cf. `26-DIMENSIONS-CADRAGE.md` §10, levier DIM-D).

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, resolve_player_targets, ServerCommandMap, ServerCommandRuntime};

/// Parse un identifiant de dimension : nom ou numéro.
fn parse_dimension(token: &str) -> Option<i32> {
    match token.to_lowercase().as_str() {
        "overworld" | "0" => Some(0),
        "nether" | "the_nether" | "1" => Some(1),
        "end" | "the_end" | "2" => Some(2),
        _ => None,
    }
}

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut dimension = CommandDefinition::new("dimension", "Teleport a player to a dimension");
    dimension.usage = "/dimension <overworld|nether|end> [player]".into();
    dimension.permissions = vec!["server.command.dimension".into()];
    dimension.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param(
                "dimension",
                "dimension_values",
                &["overworld", "nether", "end"],
                false,
            ),
            param("player", ParamType::Target, true),
        ],
    });
    register_command(
        permissions,
        map,
        dimension,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(dim_token) = invocation.arg(0) else {
                return usage("Usage: /dimension <overworld|nether|end> [player]");
            };
            let dim = parse_dimension(dim_token).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown dimension: {dim_token}"))
            })?;
            if invocation.arg(1).is_none() && !runtime.sender_is_player() {
                return Err(CommandDispatchError::Message(
                    "Console must specify a player target. Usage: /dimension <dim> <player>".into(),
                ));
            }
            let targets = resolve_player_targets(runtime, invocation.arg(1), true)?;
            let count = targets.len();
            let mut moved = 0;
            for target in targets {
                if runtime.player_dimension(target) == Some(dim) {
                    continue; // déjà dans cette dimension
                }
                runtime.change_player_dimension(target, dim);
                moved += 1;
            }
            let name = match dim {
                1 => "the Nether",
                2 => "the End",
                _ => "the Overworld",
            };
            runtime.send_feedback(&format!("Sent {moved}/{count} player(s) to {name}."));
            Ok(())
        },
    );
}
