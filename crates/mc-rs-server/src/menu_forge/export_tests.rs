//! Parité entre l’export Menu Forge versionné (`menu_forge/export/`, écrit par
//! le studio) et le serveur : accroche du pack `mcrs_ui`, écrans de `/menu`, et
//! comportement de chaque bouton des anciens formulaires (`connection/forms.rs`).

use std::collections::BTreeMap;
use std::sync::Arc;

use super::*;
use crate::connection::{legacy_click, legacy_form, Connection, LegacyClick};

const RUNTIME: &str = include_str!("../../../../menu_forge/export/runtime.json");
const PACK_MANIFEST: &str = include_str!("../../../../menu_forge/export/pack/manifest.json");
const ROUTER: &str = include_str!("../../../../menu_forge/export/pack/ui/menu_forge/router.json");
const UI_DEFS: &str = include_str!("../../../../menu_forge/export/pack/ui/_ui_defs.json");
const SERVER_FORM: &str = include_str!("../../../../resource_packs/mcrs_ui/ui/server_form.json");
const GLOBALS: &str = include_str!("../../../../resource_packs/mcrs_ui/ui/_global_variables.json");

/// Point de réapparition de `viewer()` (variable `{mcrs.spawn}`), pour les anciens formulaires.
const SPAWN: [f32; 3] = [1.5, 64.0, -2.5];

/// Icônes vanilla ajoutées à quatre boutons du hub : la seule différence voulue avec l’ancien hub.
const HUB_ICONS: [(usize, &str); 4] = [
    (0, "textures/items/compass_item"),
    (1, "textures/blocks/grass_side_carried"),
    (3, "textures/items/clock_item"),
    (6, "textures/items/map_filled"),
];

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
            catalog.form(panel).is_some(),
            "formulaire « {panel} » absent de l’export"
        );
        assert!(
            catalog.menu(&format!("grid_{panel}")).is_some(),
            "grille « grid_{panel} » absente de l’export"
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
fn form_layouts_match_mcrs_ui() {
    let globals: serde_json::Value = serde_json::from_str(GLOBALS).unwrap();
    for (layout, flag) in catalog::FORM_LAYOUTS {
        assert_eq!(globals[format!("$flag_{layout}")], flag, "{layout}");
    }
}

#[test]
fn ported_forms_send_the_legacy_json() {
    // Même JSON que l’ancien formulaire (titre à drapeau, contenu, textes, images),
    // aux icônes d’exemple du hub près.
    let catalog = exported();
    for panel in Connection::DEMO_PANEL_NAMES {
        let (_, legacy) = legacy_form(panel).expect("ancien formulaire");
        let mut expected: serde_json::Value = serde_json::from_str(&legacy).unwrap();
        if *panel == "hub" {
            for (index, path) in HUB_ICONS {
                expected["buttons"][index]["image"] =
                    serde_json::json!({ "type": "path", "data": path });
            }
        }
        let mut session = Session::open(Arc::clone(&catalog), panel, &BTreeMap::new()).unwrap();
        let composed: serde_json::Value =
            serde_json::from_str(&session.compose(&viewer(), &NoLists).to_json()).unwrap();
        assert_eq!(composed, expected, "{panel}");
    }
}

#[test]
fn ported_forms_keep_the_legacy_clicks() {
    // Chaque bouton de chaque écran fait ce que faisait l’ancien : commande puis
    // fermeture, ouverture du même écran, ou fermeture seule.
    let catalog = exported();
    for panel in Connection::DEMO_PANEL_NAMES {
        let (kind, legacy) = legacy_form(panel).expect("ancien formulaire");
        let legacy: serde_json::Value = serde_json::from_str(&legacy).unwrap();
        let count = legacy["buttons"].as_array().unwrap().len();
        for index in 0..count {
            let mut session = Session::open(Arc::clone(&catalog), panel, &BTreeMap::new()).unwrap();
            session.compose(&viewer(), &NoLists);
            let effects = session.click(index, &viewer(), &NoLists);
            match legacy_click(kind, index, SPAWN) {
                LegacyClick::Command(line) => assert_eq!(
                    effects,
                    [
                        Effect::Command {
                            line,
                            runner: CommandRunner::Player
                        },
                        Effect::Closed
                    ],
                    "{panel}[{index}]"
                ),
                LegacyClick::Open(target) => {
                    assert_eq!(effects.len(), 1, "{panel}[{index}]");
                    assert_eq!(opened(&effects), target, "{panel}[{index}]");
                }
                LegacyClick::Nothing => assert_eq!(effects, [Effect::Closed], "{panel}[{index}]"),
            }
        }
    }
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
    let catalog = exported();
    // Un formulaire Bedrock n’a pas de disposition générée : ni jeton dans le routeur, ni fichier.
    for panel in Connection::DEMO_PANEL_NAMES {
        assert!(!router_text.contains(&format!("[mf:{panel}]")), "{panel}");
        assert!(
            !ui_defs.contains(&format!("ui/menu_forge/{panel}.json")),
            "{panel}"
        );
    }
    for id in catalog
        .menu_ids()
        .into_iter()
        .filter(|id| catalog.menu(id).is_some())
    {
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
fn grid_menus_keep_the_e2_behaviour() {
    // Les écrans en grille de l’étape E2, renommés grid_* : mêmes commandes, navigation entre eux.
    for (slot, command) in [
        ("spawn", "tp 1.5 64 -2.5"),
        ("creative", "gamemode creative"),
        ("survival", "gamemode survival"),
        ("day", "time set day"),
        ("night", "time set night"),
        ("biome", "biome"),
    ] {
        assert_eq!(
            click("grid_hub", slot),
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
    assert_eq!(opened(&click("grid_hub", "showcase")), "grid_showcase");
}

#[test]
fn grid_showcase_and_panels_navigate_between_grids() {
    for (slot, panel) in [
        ("open_grid", "grid_grid"),
        ("open_left_button", "grid_left_button"),
        ("open_bottom_button", "grid_bottom_button"),
        ("open_image_grid", "grid_image_grid"),
        ("open_square_image", "grid_square_image"),
        ("open_motd", "grid_motd"),
        ("open_store", "grid_store"),
        ("open_wrapped", "grid_wrapped"),
        ("back", "grid_hub"),
    ] {
        assert_eq!(opened(&click("grid_showcase", slot)), panel, "{slot}");
    }

    // Panneaux : tout bouton ramène au showcase en grille, sauf « Rejoindre » (fermeture).
    let catalog = exported();
    for panel in &Connection::DEMO_PANEL_NAMES[2..] {
        let id = format!("grid_{panel}");
        let menu = catalog.menu(&id).unwrap();
        let clickable: Vec<&str> = menu
            .slots
            .iter()
            .filter(|slot| slot.kind == catalog::SlotKind::Button && !slot.on_click.is_empty())
            .map(|slot| slot.id.as_str())
            .collect();
        assert!(!clickable.is_empty(), "{id}");
        for slot in clickable {
            let effects = click(&id, slot);
            if slot.starts_with("join_") {
                assert_eq!(effects, [Effect::Closed], "{id}.{slot}");
            } else {
                assert_eq!(opened(&effects), "grid_showcase", "{id}.{slot}");
            }
        }
    }
}
