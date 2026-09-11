//! `/mf <open|list|reload> [menu]` : menus Menu Forge exportés par le studio
//! (voir `crate::menu_forge`). Réservé aux opérateurs : un menu peut lancer
//! des commandes en console.

use mc_rs_command::{
    hard_enum_param, soft_enum_param, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

/// Enum souple des identifiants de menus (complétion de `/mf open`).
pub(super) const MENU_ENUM: &str = "menu_forge_menus";
const USAGE: &str = "/mf <open|list|reload> [menu]";

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut mf = CommandDefinition::new(
        "mf",
        "Menu Forge : ouvrir, lister ou recharger les menus exportés",
    );
    mf.usage = USAGE.into();
    mf.permissions = vec!["server.command.mf".into()];
    mf.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "mf_open", &["open"], false),
            soft_enum_param("menu", MENU_ENUM, false),
        ],
    });
    mf.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "action",
            "mf_manage",
            &["list", "reload"],
            false,
        )],
    });
    register_command(
        permissions,
        map,
        mf,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| match invocation
            .arg(0)
        {
            Some("open") => {
                let menu = invocation
                    .arg(1)
                    .ok_or_else(|| CommandDispatchError::Usage("/mf open <menu>".into()))?;
                runtime
                    .open_sender_menu_forge(menu)
                    .map_err(CommandDispatchError::Message)
            }
            Some("list") => {
                let menus = runtime.menu_forge_menus();
                let message = if menus.is_empty() {
                    "Aucun menu Menu Forge chargé.".to_string()
                } else {
                    format!("Menus Menu Forge ({}) : {}", menus.len(), menus.join(", "))
                };
                runtime.send_feedback(&message);
                Ok(())
            }
            Some("reload") => {
                let summary = runtime
                    .reload_menu_forge()
                    .map_err(CommandDispatchError::Message)?;
                runtime.send_feedback(&format!(
                        "Menu Forge rechargé : {summary} (un nouveau visuel exige un redémarrage et une reconnexion)"
                    ));
                Ok(())
            }
            _ => Err(CommandDispatchError::Usage(USAGE.into())),
        },
    );
}
