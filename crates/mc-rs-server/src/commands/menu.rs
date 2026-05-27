//! Port de PMMP `src/command/defaults/MenuCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.
//!
//! Avec un argument optionnel `panel` (HardEnum) pour autocomplete et accès
//! direct à chaque layout custom du pack `mcrs_ui`.

use mc_rs_command::{
    hard_enum_param, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};
use crate::connection::Connection;

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut menu = CommandDefinition::new("menu", "Open the hub menu or a specific UI panel");
    menu.usage = "/menu [panel]".into();
    menu.permissions = vec!["server.command.menu".into()];
    menu.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "panel",
            "menu_panel",
            Connection::DEMO_PANEL_NAMES,
            true,
        )],
    });
    register_command(
        permissions,
        map,
        menu,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| match invocation
            .arg(0)
        {
            None => {
                runtime.open_sender_menu();
                Ok(())
            }
            Some(panel) => runtime
                .open_sender_panel(panel)
                .map_err(CommandDispatchError::Message),
        },
    );
}
