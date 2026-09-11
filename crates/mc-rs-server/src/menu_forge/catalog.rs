//! Descripteur d’exécution exporté par le studio (`runtime.json`) : types et
//! validation. Voir le contrat, `docs/bedrock.md` § 5 du dépôt Menu Forge.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use super::session::COLUMNS;
use super::{MENU_FORGE_FLAG, RUNTIME_FILE, TEXT_MARKER};

/// Nom et version du format du descripteur.
pub const RUNTIME_FORMAT: &str = "menu-forge-bedrock";
pub const RUNTIME_FORMAT_VERSION: u32 = 1;
const MAX_ROWS: usize = 6;

/// Valeur d’une variable d’état (texte, nombre ou booléen, comme dans le format).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum StateValue {
    Bool(bool),
    Number(f64),
    Text(String),
}

impl StateValue {
    /// Texte d’une valeur dans une variable `{state.<nom>}` : nombres entiers sans décimale.
    pub fn display(&self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::Number(value) if value.fract() == 0.0 && value.abs() < 1e15 => {
                format!("{}", *value as i64)
            }
            Self::Number(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

/// Condition du format (`docs/format.md` § Conditions).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    All {
        all: Vec<Condition>,
    },
    Any {
        any: Vec<Condition>,
    },
    Not {
        not: Box<Condition>,
    },
    Flag {
        flag: String,
    },
    In {
        state: String,
        #[serde(rename = "in")]
        values: Vec<StateValue>,
    },
    Is {
        state: String,
        is: StateValue,
    },
}

/// Variable d’état d’un menu.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StateDefinition {
    Enum {
        values: Vec<String>,
        default: String,
    },
    Bool {
        default: bool,
    },
    Int {
        #[serde(default)]
        default: Option<f64>,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    Page {
        list: String,
    },
}

/// Exécutant d’une commande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandRunner {
    #[default]
    Player,
    Console,
}

/// Action d’un clic (`docs/format.md` § Actions) ; le nom d’un son est déjà traduit pour Bedrock.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    Open {
        menu: String,
        #[serde(default)]
        state: BTreeMap<String, StateValue>,
    },
    Back,
    Close,
    SetState {
        state: String,
        value: StateValue,
    },
    NextPage {
        list: String,
    },
    PrevPage {
        list: String,
    },
    Sound {
        sound: String,
        #[serde(default)]
        volume: Option<f32>,
        #[serde(default)]
        pitch: Option<f32>,
    },
    Command {
        command: String,
        #[serde(default, rename = "as")]
        runner: CommandRunner,
    },
    Custom {
        id: String,
        #[serde(default)]
        args: serde_json::Value,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotKind {
    Button,
    List,
    Input,
    Decoration,
}

/// Élément figé conditionnel : `token` va dans le titre si `visible_when` est vraie.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDescriptor {
    pub token: String,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    pub visible_when: Condition,
}

/// Texte dynamique : l’entrée `entry` vaut `prefix` + valeur interpolée.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDescriptor {
    pub id: String,
    pub entry: usize,
    pub prefix: String,
    pub value: String,
    #[serde(default)]
    pub visible_when: Option<Condition>,
}

/// Zone de slots : cases couvertes (`colonne + 9 × ligne`), rendu et actions.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotDescriptor {
    pub id: String,
    pub kind: SlotKind,
    pub cells: Vec<usize>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub list: Option<String>,
    #[serde(default)]
    pub on_click: Vec<Action>,
    #[serde(default)]
    pub visible_when: Option<Condition>,
    #[serde(default)]
    pub enabled_when: Option<Condition>,
}

/// Un menu résolu, prêt à composer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuDescriptor {
    pub id: String,
    pub name: String,
    pub rows: usize,
    /// Nombre de boutons à envoyer : `9 × rows` cases, puis les textes dynamiques.
    pub entries: usize,
    pub token: String,
    #[serde(default)]
    pub state: BTreeMap<String, StateDefinition>,
    #[serde(default)]
    pub tokens: Vec<TokenDescriptor>,
    #[serde(default)]
    pub texts: Vec<TextDescriptor>,
    #[serde(default)]
    pub slots: Vec<SlotDescriptor>,
}

/// Pack écrit avec ce descripteur.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackInfo {
    pub name: String,
    pub uuid: String,
    pub version: [u32; 3],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeDescriptor {
    format: String,
    format_version: u32,
    flag: String,
    pack: PackInfo,
    #[serde(default)]
    warnings: Vec<String>,
    menus: Vec<MenuDescriptor>,
}

/// Menus chargés d’un descripteur, validés.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pack: Option<PackInfo>,
    warnings: Vec<String>,
    menus: Vec<Arc<MenuDescriptor>>,
    index: HashMap<String, usize>,
}

impl Catalog {
    /// Catalogue sans menu.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Lit `<directory>/runtime.json`.
    pub fn load(directory: &Path) -> Result<Self, String> {
        let path = directory.join(RUNTIME_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("descripteur {path:?} illisible : {error}"))?;
        Self::from_json(&text).map_err(|error| format!("{path:?} : {error}"))
    }

    /// Valide un descripteur ; une incohérence de structure est une erreur,
    /// un menu cité mais absent un simple avertissement.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let descriptor: RuntimeDescriptor =
            serde_json::from_str(text).map_err(|error| format!("JSON invalide : {error}"))?;
        if descriptor.format != RUNTIME_FORMAT {
            return Err(format!(
                "format « {} » inattendu (attendu : {RUNTIME_FORMAT})",
                descriptor.format
            ));
        }
        if descriptor.format_version != RUNTIME_FORMAT_VERSION {
            return Err(format!(
                "version de contrat {} non prise en charge (attendu : {RUNTIME_FORMAT_VERSION})",
                descriptor.format_version
            ));
        }
        if descriptor.flag != MENU_FORGE_FLAG {
            return Err(format!(
                "drapeau {:?} différent de celui du serveur et du pack mcrs_ui ({MENU_FORGE_FLAG:?})",
                descriptor.flag
            ));
        }

        let mut catalog = Self {
            pack: Some(descriptor.pack),
            warnings: descriptor
                .warnings
                .into_iter()
                .map(|warning| format!("export : {warning}"))
                .collect(),
            ..Self::default()
        };
        for menu in descriptor.menus {
            validate_menu(&menu)?;
            if catalog.index.contains_key(&menu.id) {
                return Err(format!("menu « {} » présent deux fois", menu.id));
            }
            catalog.index.insert(menu.id.clone(), catalog.menus.len());
            catalog.menus.push(Arc::new(menu));
        }

        let known: BTreeSet<&str> = catalog.index.keys().map(String::as_str).collect();
        let mut missing = Vec::new();
        for menu in &catalog.menus {
            for slot in &menu.slots {
                for action in &slot.on_click {
                    if let Action::Open { menu: target, .. } = action {
                        if !known.contains(target.as_str()) {
                            missing.push(format!(
                                "{} : le slot « {} » ouvre le menu « {target} », absent",
                                menu.id, slot.id
                            ));
                        }
                    }
                }
            }
        }
        catalog.warnings.extend(missing);
        Ok(catalog)
    }

    pub fn menu(&self, id: &str) -> Option<Arc<MenuDescriptor>> {
        self.index
            .get(id)
            .map(|&index| Arc::clone(&self.menus[index]))
    }

    /// Identifiants des menus, triés.
    pub fn menu_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.index.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn len(&self) -> usize {
        self.menus.len()
    }

    pub fn is_empty(&self) -> bool {
        self.menus.is_empty()
    }

    pub fn pack(&self) -> Option<&PackInfo> {
        self.pack.as_ref()
    }

    /// Avertissements de l’export et du chargement.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Résumé d’une ligne pour le journal et `/mf reload`.
    pub fn summary(&self) -> String {
        let pack = self
            .pack
            .as_ref()
            .map(|pack| {
                format!(
                    ", pack {} {}",
                    pack.name,
                    pack.version.map(|part| part.to_string()).join(".")
                )
            })
            .unwrap_or_default();
        format!(
            "{} menu(s) chargé(s){pack} : {}",
            self.len(),
            self.menu_ids().join(", ")
        )
    }
}

fn validate_menu(menu: &MenuDescriptor) -> Result<(), String> {
    let id = &menu.id;
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(format!("identifiant de menu invalide : {id:?}"));
    }
    if !(1..=MAX_ROWS).contains(&menu.rows) {
        return Err(format!(
            "{id} : {} lignes (attendu : 1 à {MAX_ROWS})",
            menu.rows
        ));
    }
    let cells = COLUMNS * menu.rows;
    if menu.entries < cells {
        return Err(format!(
            "{id} : {} entrées pour {cells} cases",
            menu.entries
        ));
    }
    if menu.token != format!("[mf:{id}]") {
        return Err(format!("{id} : jeton de menu {:?} inattendu", menu.token));
    }
    for slot in &menu.slots {
        if let Some(cell) = slot.cells.iter().find(|&&cell| cell >= cells) {
            return Err(format!(
                "{id} : le slot « {} » cite la case {cell}, hors de la grille",
                slot.id
            ));
        }
    }
    let mut entries = BTreeSet::new();
    for text in &menu.texts {
        if !(cells..menu.entries).contains(&text.entry) || !entries.insert(text.entry) {
            return Err(format!(
                "{id} : entrée {} du texte « {} » invalide",
                text.entry, text.id
            ));
        }
        if !text.prefix.starts_with(TEXT_MARKER) {
            return Err(format!(
                "{id} : préfixe {:?} du texte « {} » invalide",
                text.prefix, text.id
            ));
        }
    }
    Ok(())
}
