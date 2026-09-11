//! Port de PMMP `src/command/defaults/MenuCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.
//!
//! Avec un argument optionnel `panel` (HardEnum) pour autocomplete et accès
//! direct à chaque layout custom du pack `mcrs_ui`.
//!
//! Les écrans passent par le moteur Menu Forge (`crate::menu_forge`) quand le
//! menu exporté du même nom existe et que `[menu_forge] menu_command` est
//! actif ; sinon, par les anciens formulaires de `connection/forms.rs`, gardés
//! en repli. `/menu legacy` rouvre toujours l’ancien hub.

use mc_rs_command::{
    hard_enum_param, CommandDefinition, CommandDispatchError, CommandInvocation, CommandOverload,
    PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};
use crate::connection::Connection;

/// Valeur de `/menu <panel>` qui ouvre l’ancien hub (formulaires écrits à la main).
pub(super) const LEGACY_PANEL: &str = "legacy";

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut menu = CommandDefinition::new("menu", "Open the hub menu or a specific UI panel");
    menu.usage = "/menu [panel]".into();
    menu.permissions = vec!["server.command.menu".into()];
    let mut panels = Connection::DEMO_PANEL_NAMES.to_vec();
    panels.push(LEGACY_PANEL);
    menu.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param("panel", "menu_panel", &panels, true)],
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
