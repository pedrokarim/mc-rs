//! Tests du moteur Menu Forge sur un descripteur de test, de la même forme que
//! celui qu’écrit l’exporteur du studio (`studio/tests/bedrock.test.ts`).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::json;

use super::catalog::{Action, StateValue};
use super::session::{interpolate, ComposedForm, ListItem, Lists, MAX_STACK};
use super::*;

fn fixture() -> serde_json::Value {
    json!({
        "format": "menu-forge-bedrock",
        "formatVersion": 1,
        "flag": MENU_FORGE_FLAG,
        "pack": { "name": "Menu Forge", "uuid": "0b0e7a4c-5d1f-8b2a-9c3d-4e5f60718293", "version": [1, 0, 3] },
        "warnings": ["shop : slot « sell », type input sans équivalent, ignoré"],
        "menus": [
            {
                "id": "shop",
                "name": "Boutique (test)",
                "rows": 3,
                "entries": 28,
                "token": "[mf:shop]",
                "state": {
                    "tab": { "type": "enum", "values": ["a", "b"], "default": "a" },
                    "count": { "type": "int", "min": 0, "max": 3 },
                    "page": { "type": "page", "list": "goods" }
                },
                "tokens": [
                    { "token": "(1)", "layer": "tab_on", "visibleWhen": { "state": "tab", "is": "b" } },
                    { "token": "(t1)", "text": "hint", "visibleWhen": { "flag": "viewer.op" } }
                ],
                "texts": [
                    { "id": "greeting", "entry": 27, "prefix": "{t0}", "value": "Bonjour {viewer.name}, {state.tab} {state.count} {page.number}/{page.count}" }
                ],
                "slots": [
                    {
                        "id": "tab_b", "kind": "button", "cells": [1, 2], "label": "§aOnglet B", "icon": null,
                        "enabledWhen": { "state": "tab", "is": "a" },
                        "onClick": [
                            { "type": "setState", "state": "tab", "value": "b" },
                            { "type": "sound", "sound": "random.click", "volume": 0.5 }
                        ]
                    },
                    { "id": "gem", "kind": "decoration", "cells": [26], "label": "§8Joyau", "icon": "textures/items/diamond" },
                    {
                        "id": "secret", "kind": "button", "cells": [0], "label": "Secret {viewer.name}",
                        "visibleWhen": { "flag": "viewer.op" },
                        "onClick": [{ "type": "command", "command": "/say {viewer.name} {mcrs.spawn}", "as": "console" }]
                    },
                    { "id": "goods", "kind": "list", "cells": [9, 10, 11], "list": "goods" },
                    { "id": "sell", "kind": "input", "cells": [13] },
                    {
                        "id": "next", "kind": "button", "cells": [17], "label": "Suivant",
                        "enabledWhen": { "flag": "page.hasNext" },
                        "onClick": [{ "type": "nextPage", "list": "goods" }]
                    },
                    {
                        "id": "prev", "kind": "button", "cells": [16], "label": "Précédent",
                        "enabledWhen": { "flag": "page.hasPrev" },
                        "onClick": [{ "type": "prevPage", "list": "goods" }]
                    },
                    { "id": "more", "kind": "button", "cells": [18], "label": "Plus", "onClick": [{ "type": "setState", "state": "count", "value": 9 }] },
                    { "id": "other", "kind": "button", "cells": [25], "label": "Autre", "onClick": [{ "type": "open", "menu": "other", "state": { "flag": true } }] },
                    { "id": "quit", "kind": "button", "cells": [8], "label": "Quitter", "onClick": [{ "type": "close" }] }
                ]
            },
            {
                "id": "other",
                "name": "Autre",
                "rows": 1,
                "entries": 10,
                "token": "[mf:other]",
                "state": { "flag": { "type": "bool", "default": false } },
                "texts": [{ "id": "flag", "entry": 9, "prefix": "{t0}", "value": "{state.flag}" }],
                "slots": [
                    { "id": "back", "kind": "button", "cells": [0], "label": "Retour", "onClick": [{ "type": "back" }] },
                    { "id": "loop", "kind": "button", "cells": [1], "label": "Encore", "onClick": [{ "type": "open", "menu": "other" }] },
                    { "id": "ghost", "kind": "button", "cells": [2], "label": "Fantôme", "onClick": [{ "type": "open", "menu": "absent" }] },
                    { "id": "custom", "kind": "button", "cells": [3], "label": "{", "onClick": [{ "type": "custom", "id": "shop.buy", "args": { "item": 3 } }] }
                ]
            }
        ],
        "forms": [
            {
                "id": "panel",
                "name": "Panneau",
                "layout": "left_button",
                "flag": "§m§b",
                "title": "§6Panneau {viewer.name}",
                "content": "Onglet {state.tab}",
                "state": { "tab": { "type": "enum", "values": ["a", "b"], "default": "a" } },
                "buttons": [
                    {
                        "id": "secret", "text": "Secret", "visibleWhen": { "flag": "viewer.op" },
                        "onClick": [{ "type": "command", "command": "say secret {viewer.name}", "as": "console" }]
                    },
                    { "id": "banner", "text": "§m§a Bannière", "image": { "type": "url", "data": "https://example.org/a.png" } },
                    {
                        "id": "to_b", "text": "Vers B", "image": { "type": "path", "data": "textures/items/diamond" },
                        "visibleWhen": { "state": "tab", "is": "a" },
                        "onClick": [{ "type": "setState", "state": "tab", "value": "b" }]
                    },
                    { "id": "other", "text": "Autre {state.tab}", "onClick": [{ "type": "open", "menu": "other" }] },
                    { "id": "quit", "text": "Quitter", "onClick": [{ "type": "close" }] }
                ]
            }
        ]
    })
}

fn catalog() -> Arc<Catalog> {
    Arc::new(Catalog::from_json(&fixture().to_string()).expect("descripteur valide"))
}

fn player() -> Viewer {
    Viewer::new("Karim", "uuid-1").with_variable("mcrs.spawn", "1 2 3")
}

fn texts(form: &ComposedForm) -> Vec<&str> {
    form.buttons
        .iter()
        .map(|entry| entry.text.as_str())
        .collect()
}

fn shown(effects: &[Effect]) -> &ComposedForm {
    match effects.last() {
        Some(Effect::Show(form)) => form,
        other => panic!("dernier effet inattendu : {other:?}"),
    }
}

fn open(menu: &str) -> Session {
    Session::open(catalog(), menu, &BTreeMap::new()).unwrap()
}

/// Source de test : 7 éléments, un clic sur les éléments pairs.
struct Goods;

impl Lists for Goods {
    fn items(&self, list: &str, _viewer: &Viewer) -> Vec<ListItem> {
        assert_eq!(list, "goods");
        (0..7)
            .map(|index| ListItem {
                label: format!("Item {index}"),
                icon: Some(format!("textures/items/item_{index}")),
                on_click: if index % 2 == 0 {
                    vec![Action::Command {
                        command: format!("give {{viewer.name}} {index}"),
                        runner: CommandRunner::Player,
                    }]
                } else {
                    Vec::new()
                },
            })
            .collect()
    }
}

#[test]
fn catalog_loads_the_exported_descriptor() {
    let catalog = catalog();
    assert_eq!(catalog.menu_ids(), ["other", "panel", "shop"]);
    assert_eq!(catalog.form_count(), 1);
    assert!(catalog.form("panel").is_some() && catalog.menu("panel").is_none());
    assert_eq!(catalog.pack().unwrap().version, [1, 0, 3]);
    let warnings = catalog.warnings().join("\n");
    assert!(warnings.contains("export : shop"), "{warnings}");
    assert!(warnings.contains("« absent »"), "{warnings}");
    assert!(
        catalog
            .summary()
            .starts_with("3 menu(s) chargé(s) dont 1 formulaire(s) Bedrock"),
        "{}",
        catalog.summary()
    );
    assert!(Catalog::empty().is_empty());

    // Un descripteur d’avant les formulaires (sans `forms`) reste lisible.
    let mut without_forms = fixture();
    without_forms.as_object_mut().unwrap().remove("forms");
    assert_eq!(
        Catalog::from_json(&without_forms.to_string())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn catalog_rejects_structural_errors() {
    let broken = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = fixture();
        edit(&mut value);
        Catalog::from_json(&value.to_string())
    };
    assert!(broken(&|value| value["format"] = json!("autre")).is_err());
    assert!(broken(&|value| value["formatVersion"] = json!(2)).is_err());
    assert!(broken(&|value| value["flag"] = json!("\u{00A7}m\u{00A7}z")).is_err());
    assert!(broken(&|value| value["menus"][0]["rows"] = json!(7)).is_err());
    assert!(broken(&|value| value["menus"][0]["entries"] = json!(20)).is_err());
    assert!(broken(&|value| value["menus"][0]["token"] = json!("[mf:autre]")).is_err());
    assert!(broken(&|value| value["menus"][0]["slots"][0]["cells"] = json!([27])).is_err());
    assert!(broken(&|value| value["menus"][0]["texts"][0]["entry"] = json!(5)).is_err());
    assert!(broken(&|value| value["menus"][0]["texts"][0]["prefix"] = json!("[t0]")).is_err());
    assert!(broken(&|value| value["menus"][1]["id"] = json!("shop")).is_err());
    assert!(broken(&|value| value["menus"][1]["id"] = json!("Bad-Id")).is_err());
    // Formulaires : disposition connue, drapeau du pack mcrs_ui, identifiants distincts, images non vides.
    assert!(broken(&|value| value["forms"][0]["layout"] = json!("carousel")).is_err());
    assert!(broken(&|value| value["forms"][0]["flag"] = json!("§m§a")).is_err());
    assert!(broken(&|value| value["forms"][0]["id"] = json!("shop")).is_err());
    assert!(broken(&|value| value["forms"][0]["buttons"][1]["id"] = json!("secret")).is_err());
    assert!(
        broken(&|value| value["forms"][0]["buttons"][1]["image"]["data"] = json!(" ")).is_err()
    );
    assert!(
        broken(&|value| value["forms"][0]["buttons"][1]["image"]["type"] = json!("file")).is_err()
    );
    assert!(Catalog::from_json("{").is_err());
}

#[test]
fn composes_the_title_tokens_and_entries() {
    let mut session = open("shop");
    let form = session.compose(&player(), &NoLists);
    assert_eq!(form.menu, "shop");
    assert_eq!(
        form.title,
        "\u{00A7}m\u{00A7}v[mf:shop]\u{00A7}rBoutique test"
    );
    assert_eq!(form.buttons.len(), 28);
    let entries = texts(&form);
    assert_eq!(entries[0], "", "slot masqué pour un joueur non opérateur");
    assert_eq!(entries[1], "\u{00A7}aOnglet B");
    assert_eq!(entries[2], "\u{00A7}aOnglet B");
    assert_eq!(entries[26], "{x}\u{00A7}8Joyau", "décoration inerte");
    assert_eq!(
        form.buttons[26]
            .image
            .as_ref()
            .map(|image| image.data.as_str()),
        Some("textures/items/diamond")
    );
    assert_eq!(entries[13], "", "slot input sans équivalent");
    assert_eq!(entries[16], "{x}Précédent", "enabledWhen faux");
    assert_eq!(entries[17], "{x}Suivant", "une seule page sans source");
    assert_eq!(entries[8], "Quitter");
    assert_eq!(entries[27], "{t0}Bonjour Karim, a 0 1/1");

    let op = player().with_flag("viewer.op");
    let form = open("shop").compose(&op, &NoLists);
    assert_eq!(
        form.title,
        "\u{00A7}m\u{00A7}v[mf:shop](t1)\u{00A7}rBoutique test"
    );
    assert_eq!(texts(&form)[0], "Secret Karim");

    let json: serde_json::Value = serde_json::from_str(&form.to_json()).unwrap();
    assert_eq!(json["type"], "form");
    assert_eq!(json["content"], "");
    assert_eq!(json["buttons"].as_array().unwrap().len(), 28);
    assert_eq!(
        json["buttons"][26]["image"],
        json!({ "type": "path", "data": "textures/items/diamond" })
    );
    assert!(json["buttons"][0].get("image").is_none());
}

#[test]
fn a_click_runs_its_actions_then_recomposes() {
    let mut session = open("shop");
    session.compose(&player(), &NoLists);
    let effects = session.click(1, &player(), &NoLists);
    assert_eq!(
        effects[0],
        Effect::Sound {
            name: "random.click".into(),
            volume: 0.5,
            pitch: 1.0
        }
    );
    let form = shown(&effects);
    assert!(
        form.title.contains("(1)"),
        "jeton de la couche visible quand tab = b"
    );
    assert_eq!(
        texts(form)[1],
        "{x}\u{00A7}aOnglet B",
        "plus actif quand tab = b"
    );
    assert_eq!(texts(form)[27], "{t0}Bonjour Karim, b 0 1/1");

    // Case inerte, vide ou hors de la liste : le menu est simplement renvoyé.
    for index in [26, 5, 999] {
        let effects = session.click(index, &player(), &NoLists);
        assert_eq!(effects.len(), 1, "{index}");
        assert!(matches!(effects[0], Effect::Show(_)), "{index}");
    }
}

#[test]
fn state_values_follow_their_definition() {
    let mut session = open("shop");
    session.compose(&player(), &NoLists);
    let effects = session.click(18, &player(), &NoLists);
    assert_eq!(
        texts(shown(&effects))[27],
        "{t0}Bonjour Karim, a 3 1/1",
        "int ramené à max"
    );

    let mut refused = Session::open(
        catalog(),
        "other",
        &BTreeMap::from([("flag".to_string(), StateValue::Text("oui".into()))]),
    )
    .unwrap();
    assert_eq!(refused.take_warnings().len(), 1);
    assert_eq!(texts(&refused.compose(&player(), &NoLists))[9], "{t0}false");
}

#[test]
fn open_back_and_close_manage_the_stack() {
    let mut session = open("shop");
    session.compose(&player(), &NoLists);
    let effects = session.click(25, &player(), &NoLists);
    let form = shown(&effects);
    assert_eq!(form.menu, "other");
    assert_eq!(texts(form)[9], "{t0}true", "état initial de l’action open");
    assert_eq!(session.depth(), 1);

    let effects = session.click(2, &player(), &NoLists);
    assert!(matches!(&effects[0], Effect::Warning(message) if message.contains("absent")));
    assert_eq!(shown(&effects).menu, "other");

    let effects = session.click(3, &player(), &NoLists);
    assert_eq!(
        effects[0],
        Effect::Custom {
            id: "shop.buy".into(),
            args: json!({ "item": 3 })
        }
    );
    assert_eq!(
        texts(shown(&effects))[3],
        "(",
        "« {{ » devient « ( », jamais pris pour un marqueur"
    );

    let effects = session.click(0, &player(), &NoLists);
    assert_eq!(shown(&effects).menu, "shop");
    assert_eq!(session.depth(), 0);
    assert_eq!(session.click(8, &player(), &NoLists), [Effect::Closed]);

    let mut alone = open("other");
    alone.compose(&player(), &NoLists);
    assert_eq!(
        alone.click(0, &player(), &NoLists),
        [Effect::Closed],
        "back sans menu précédent"
    );

    let mut looping = open("other");
    looping.compose(&player(), &NoLists);
    for _ in 0..(MAX_STACK + 5) {
        shown(&looping.click(1, &player(), &NoLists));
    }
    assert_eq!(looping.depth(), MAX_STACK);
}

#[test]
fn commands_are_interpolated_and_keep_their_runner() {
    let op = player().with_flag("viewer.op");
    let mut session = open("shop");
    session.compose(&op, &NoLists);
    let effects = session.click(0, &op, &NoLists);
    assert_eq!(
        effects[0],
        Effect::Command {
            line: "say Karim 1 2 3".into(),
            runner: CommandRunner::Console
        }
    );
    assert!(matches!(effects[1], Effect::Show(_)));
}

#[test]
fn list_pages_follow_the_source() {
    let mut session = open("shop");
    let form = session.compose(&player(), &Goods);
    let entries = texts(&form);
    assert_eq!(&entries[9..12], ["Item 0", "{x}Item 1", "Item 2"]);
    assert_eq!(
        form.buttons[9]
            .image
            .as_ref()
            .map(|image| image.data.as_str()),
        Some("textures/items/item_0")
    );
    assert_eq!(entries[17], "Suivant");
    assert_eq!(entries[16], "{x}Précédent");
    assert_eq!(entries[27], "{t0}Bonjour Karim, a 0 1/3");

    let form = shown(&session.click(17, &player(), &Goods)).clone();
    assert_eq!(&texts(&form)[9..12], ["{x}Item 3", "Item 4", "{x}Item 5"]);
    assert_eq!(texts(&form)[16], "Précédent");
    let form = shown(&session.click(17, &player(), &Goods)).clone();
    assert_eq!(&texts(&form)[9..12], ["Item 6", "", ""]);
    assert_eq!(texts(&form)[17], "{x}Suivant", "dernière page");
    assert_eq!(texts(&form)[27], "{t0}Bonjour Karim, a 0 3/3");

    let effects = session.click(9, &player(), &Goods);
    assert_eq!(
        effects[0],
        Effect::Command {
            line: "give Karim 6".into(),
            runner: CommandRunner::Player
        }
    );
    let form = shown(&session.click(16, &player(), &Goods)).clone();
    assert_eq!(texts(&form)[9], "{x}Item 3");
}

#[test]
fn interpolation_keeps_unknown_variables() {
    let variables = BTreeMap::from([("x".to_string(), "1".to_string())]);
    assert_eq!(
        interpolate("a {x} {y.z} {bad name} {", &variables),
        "a 1 {y.z} {bad name} {"
    );
    assert_eq!(interpolate("{x}{x}", &variables), "11");
}

#[test]
fn runtime_serves_menus_only_when_enabled() {
    let settings = MenuForgeSettings {
        enabled: true,
        directory: std::path::PathBuf::from("absent"),
        menu_command: true,
    };
    let runtime = MenuForgeRuntime::new(
        settings.clone(),
        Catalog::from_json(&fixture().to_string()).unwrap(),
    );
    assert!(runtime.serves_menu_command("shop"));
    assert!(
        runtime.serves_menu_command("panel"),
        "un formulaire Bedrock sert aussi /menu"
    );
    assert!(!runtime.serves_menu_command("hub"));
    assert_eq!(runtime.menu_ids(), ["other", "panel", "shop"]);
    assert!(
        runtime.reload().is_err(),
        "dossier absent : le catalogue courant est gardé"
    );
    assert_eq!(runtime.catalog().len(), 3);

    let without_menu_command = MenuForgeRuntime::new(
        MenuForgeSettings {
            menu_command: false,
            ..settings
        },
        Catalog::from_json(&fixture().to_string()).unwrap(),
    );
    assert!(!without_menu_command.serves_menu_command("shop"));
    assert!(
        !MenuForgeRuntime::new(MenuForgeSettings::disabled(), Catalog::empty())
            .serves_menu_command("shop")
    );
}

#[test]
fn a_form_is_sent_with_its_flag_content_and_visible_buttons() {
    let mut session = open("panel");
    let form = session.compose(&player(), &NoLists);
    assert_eq!(form.menu, "panel");
    assert_eq!(
        form.title, "§m§b §6Panneau Karim",
        "drapeau, une espace, titre"
    );
    assert_eq!(form.content, "Onglet a");
    assert_eq!(
        texts(&form),
        ["§m§a Bannière", "Vers B", "Autre a", "Quitter"],
        "le bouton des opérateurs n’est pas envoyé"
    );

    let json: serde_json::Value = serde_json::from_str(&form.to_json()).unwrap();
    assert_eq!(json["type"], "form");
    assert_eq!(json["content"], "Onglet a");
    assert_eq!(
        json["buttons"][0]["image"],
        json!({ "type": "url", "data": "https://example.org/a.png" })
    );
    assert_eq!(
        json["buttons"][1]["image"],
        json!({ "type": "path", "data": "textures/items/diamond" })
    );
    assert!(json["buttons"][2].get("image").is_none());

    let op = player().with_flag("viewer.op");
    let form = open("panel").compose(&op, &NoLists);
    assert_eq!(texts(&form)[0], "Secret");
    assert_eq!(form.buttons.len(), 5);
}

#[test]
fn a_form_click_is_the_rank_among_the_sent_buttons() {
    let mut session = open("panel");
    session.compose(&player(), &NoLists);
    // Index 1 : « Vers B » ; le bouton masqué des opérateurs ne compte pas.
    let form = shown(&session.click(1, &player(), &NoLists)).clone();
    assert_eq!(form.content, "Onglet b");
    assert_eq!(texts(&form), ["§m§a Bannière", "Autre b", "Quitter"]);

    // La bannière n’a pas d’action : le formulaire est simplement renvoyé.
    assert_eq!(
        session.click(0, &player(), &NoLists).len(),
        1,
        "rien que le formulaire"
    );
    // « Autre » ouvre un menu coffre ; son retour ramène le formulaire, état compris.
    assert_eq!(shown(&session.click(1, &player(), &NoLists)).menu, "other");
    assert_eq!(session.depth(), 1);
    let back = shown(&session.click(0, &player(), &NoLists)).clone();
    assert_eq!(
        (back.menu.as_str(), back.content.as_str()),
        ("panel", "Onglet b")
    );
    // Index hors des boutons envoyés : le formulaire est renvoyé, rien d’autre.
    assert!(matches!(
        session.click(9, &player(), &NoLists).as_slice(),
        [Effect::Show(_)]
    ));
    assert_eq!(session.click(2, &player(), &NoLists), [Effect::Closed]);

    let op = player().with_flag("viewer.op");
    let mut session = open("panel");
    session.compose(&op, &NoLists);
    assert_eq!(
        session.click(0, &op, &NoLists)[0],
        Effect::Command {
            line: "say secret Karim".into(),
            runner: CommandRunner::Console
        }
    );
}

#[test]
fn a_chest_menu_can_open_a_form() {
    let mut catalog_json = fixture();
    catalog_json["menus"][1]["slots"][1]["onClick"] =
        json!([{ "type": "open", "menu": "panel", "state": { "tab": "b" } }]);
    let catalog = Arc::new(Catalog::from_json(&catalog_json.to_string()).unwrap());
    let mut session = Session::open(catalog, "other", &BTreeMap::new()).unwrap();
    session.compose(&player(), &NoLists);
    let form = shown(&session.click(1, &player(), &NoLists)).clone();
    assert_eq!(form.menu, "panel");
    assert_eq!(form.content, "Onglet b", "état initial de l’action open");
    assert_eq!(texts(&form), ["§m§a Bannière", "Autre b", "Quitter"]);
}
