//! Parité entre l’export Menu Forge versionné (`menu_forge/export/`, écrit par
//! le studio) et le serveur : accroche du pack `mcrs_ui`, écrans de `/menu`, et
//! comportement de chaque bouton des anciens formulaires (`connection/forms.rs`).

use std::collections::BTreeMap;
use std::sync::Arc;

use super::*;
use crate::connection::Connection;

const RUNTIME: &str = include_str!("../../../../menu_forge/export/runtime.json");
const PACK_MANIFEST: &str = include_str!("../../../../menu_forge/export/pack/manifest.json");
const ROUTER: &str = include_str!("../../../../menu_forge/export/pack/ui/menu_forge/router.json");
const UI_DEFS: &str = include_str!("../../../../menu_forge/export/pack/ui/_ui_defs.json");
const SERVER_FORM: &str = include_str!("../../../../resource_packs/mcrs_ui/ui/server_form.json");
const GLOBALS: &str = include_str!("../../../../resource_packs/mcrs_ui/ui/_global_variables.json");

fn exported() -> Arc<Catalog> {
    Arc::new(Catalog::from_json(RUNTIME).expect("menu_forge/export/runtime.json valide"))
}

fn viewer() -> Viewer {
    Viewer::new("Karim", "uuid-1").with_variable("mcrs.spawn", "1.5 64 -2.5")
}

/// Clique le slot `slot` du menu `menu` (première case), dans une session neuve.
fn click(menu: &str, slot: &str) -> Vec<Effect> {
    let catalog = exported();
    let cell = catalog
        .menu(menu)
        .and_then(|descriptor| {
            descriptor
                .slots
                .iter()
                .find(|candidate| candidate.id == slot)
                .map(|found| found.cells[0])
        })
        .unwrap_or_else(|| panic!("slot {menu}.{slot} absent de l’export"));
    let mut session = Session::open(catalog, menu, &BTreeMap::new()).unwrap();
    session.compose(&viewer(), &NoLists);
    session.click(cell, &viewer(), &NoLists)
}

fn opened(effects: &[Effect]) -> &str {
    match effects.last() {
        Some(Effect::Show(form)) => &form.menu,
        other => panic!("formulaire attendu, obtenu {other:?}"),
    }
}

#[test]
fn the_export_covers_every_menu_panel() {
    let catalog = exported();
    for panel in Connection::DEMO_PANEL_NAMES {
        assert!(
            catalog.menu(panel).is_some(),
            "menu « {panel} » absent de l’export"
        );
    }
    assert!(catalog.menu("demo").is_some());
    assert!(catalog.warnings().is_empty(), "{:?}", catalog.warnings());

    let manifest: serde_json::Value = serde_json::from_str(PACK_MANIFEST).unwrap();
    let pack = catalog.pack().unwrap();
    assert_eq!(
        manifest["header"]["uuid"],
        pack.uuid.as_str(),
        "pack et descripteur du même export"
    );
    assert_eq!(
        manifest["header"]["version"],
        serde_json::json!(pack.version)
    );
}

#[test]
fn mcrs_ui_hooks_the_exported_router() {
    let globals: serde_json::Value = serde_json::from_str(GLOBALS).unwrap();
    assert_eq!(globals["$flag_menu_forge"], MENU_FORGE_FLAG);

    let form: serde_json::Value = serde_json::from_str(SERVER_FORM).unwrap();
    let switch = &form["mcrs_switching_long_form"];
    assert_eq!(switch["$flag_menu_forge"], MENU_FORGE_FLAG);
    let controls = switch["controls"].as_array().unwrap();
    let variant = controls
        .iter()
        .find_map(|control| control.get("mcrs_menu_forge@menu_forge_router.main_panel"))
        .expect("variante Menu Forge dans server_form.json");
    assert!(variant
        .to_string()
        .contains("(not ((#title_text - $flag_menu_forge) = #title_text))"));
    // Le repli vanilla exclut le drapeau, sinon la liste se superposerait au menu dessiné.
    let fallback = controls
        .iter()
        .find_map(|control| control.get("long_form@long_form"))
        .unwrap();
    let term = "((#title_text - $flag_menu_forge) = #title_text))";
    assert_eq!(
        fallback.to_string().matches(term).count(),
        2,
        "#visible et #enabled"
    );

    let router: serde_json::Value = serde_json::from_str(ROUTER).unwrap();
    assert_eq!(router["namespace"], "menu_forge_router");
    let router_text = router.to_string();
    let ui_defs = UI_DEFS.to_string();
    assert!(ui_defs.contains("ui/menu_forge/router.json"));
    for id in exported().menu_ids() {
        assert!(
            router_text.contains(&format!("menu_{id}@menu_forge_{id}.main_panel")),
            "{id}"
        );
        assert!(router_text.contains(&format!("[mf:{id}]")), "{id}");
        assert!(
            ui_defs.contains(&format!("ui/menu_forge/{id}.json")),
            "{id}"
        );
    }
}

#[test]
fn hub_buttons_keep_the_legacy_commands() {
    // Ancien hub : index 0 à 5 → commande du joueur, formulaire fermé.
    for (slot, command) in [
        ("spawn", "tp 1.5 64 -2.5"),
        ("creative", "gamemode creative"),
        ("survival", "gamemode survival"),
        ("day", "time set day"),
        ("night", "time set night"),
        ("biome", "biome"),
    ] {
        assert_eq!(
            click("hub", slot),
            [
                Effect::Command {
                    line: command.to_string(),
                    runner: CommandRunner::Player
                },
                Effect::Closed
            ],
            "{slot}"
        );
    }
    assert_eq!(opened(&click("hub", "showcase")), "showcase");
}

#[test]
fn showcase_and_panels_keep_the_legacy_navigation() {
    for (slot, panel) in [
        ("open_grid", "grid"),
        ("open_left_button", "left_button"),
        ("open_bottom_button", "bottom_button"),
        ("open_image_grid", "image_grid"),
        ("open_square_image", "square_image"),
        ("open_motd", "motd"),
        ("open_store", "store"),
        ("open_wrapped", "wrapped"),
        ("back", "hub"),
    ] {
        assert_eq!(opened(&click("showcase", slot)), panel, "{slot}");
    }

    // Panneaux : tout bouton ramène au showcase, sauf « Rejoindre » du bottom_button (formulaire fermé).
    let catalog = exported();
    for panel in &Connection::DEMO_PANEL_NAMES[2..] {
        let menu = catalog.menu(panel).unwrap();
        let clickable: Vec<&str> = menu
            .slots
            .iter()
            .filter(|slot| slot.kind == catalog::SlotKind::Button && !slot.on_click.is_empty())
            .map(|slot| slot.id.as_str())
            .collect();
        assert!(!clickable.is_empty(), "{panel}");
        for slot in clickable {
            let effects = click(panel, slot);
            if slot.starts_with("join_") {
                assert_eq!(effects, [Effect::Closed], "{panel}.{slot}");
            } else {
                assert_eq!(opened(&effects), "showcase", "{panel}.{slot}");
            }
        }
    }
}
