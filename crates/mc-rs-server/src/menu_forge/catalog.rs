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

/// Dispositions des formulaires Bedrock et leur drapeau de titre : pack
/// `mcrs_ui` (`ui/_global_variables.json`, `ui/server_form.json`) ; contrat :
/// `docs/bedrock.md` § 9 du dépôt Menu Forge (`model/bedrockForm.ts`).
pub const FORM_LAYOUTS: [(&str, &str); 8] = [
    ("grid", "§m§a"),
    ("left_button", "§m§b"),
    ("bottom_button", "§m§c"),
    ("image_grid", "§m§d"),
    ("square_image", "§m§e"),
    ("motd", "§m§f"),
    ("store", "§m§0"),
    ("wrapped", "§m§1"),
];

/// Drapeau de titre de la disposition `layout`, ou `None` si elle est inconnue.
pub fn layout_flag(layout: &str) -> Option<&'static str> {
    FORM_LAYOUTS
        .iter()
        .find(|(name, _)| *name == layout)
        .map(|(_, flag)| *flag)
}

/// Origine de l’image d’un bouton de formulaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageType {
    /// Texture d’un pack (vanilla, `mcrs_ui` ou Menu Forge), sans extension.
    Path,
    /// Image téléchargée par le client.
    Url,
}

/// Image d’un bouton, telle qu’envoyée au client (`image` d’un bouton de `ModalFormRequest`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ButtonImage {
    #[serde(rename = "type")]
    pub image_type: ImageType,
    pub data: String,
}

impl ButtonImage {
    /// Image `path` (icône d’un slot de menu coffre).
    pub fn path(data: impl Into<String>) -> Self {
        Self {
            image_type: ImageType::Path,
            data: data.into(),
        }
    }

    /// Valeur du champ `type` envoyé au client.
    pub fn type_name(&self) -> &'static str {
        match self.image_type {
            ImageType::Path => "path",
            ImageType::Url => "url",
        }
    }
}

/// Bouton d’un formulaire Bedrock : `text` est le texte envoyé (préfixe du
/// rôle et sous-titre compris), variables interpolées au moment d’envoyer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormButtonDescriptor {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub image: Option<ButtonImage>,
    #[serde(default)]
    pub on_click: Vec<Action>,
    #[serde(default)]
    pub visible_when: Option<Condition>,
}

/// Formulaire Bedrock : envoyé tel quel, dans la disposition `layout` du pack
/// `mcrs_ui` choisie par `flag` (titre envoyé : `flag`, une espace, `title`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormDescriptor {
    pub id: String,
    pub name: String,
    pub layout: String,
    pub flag: String,
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub state: BTreeMap<String, StateDefinition>,
    #[serde(default)]
    pub buttons: Vec<FormButtonDescriptor>,
}

/// Écran du catalogue : menu coffre (disposition générée par le studio) ou
/// formulaire Bedrock (disposition de `mcrs_ui`). Les deux partagent les
/// identifiants, la pile `open` / `back` et l’état.
#[derive(Debug, Clone)]
pub enum Screen {
    Chest(Arc<MenuDescriptor>),
    Form(Arc<FormDescriptor>),
}

impl Screen {
    pub fn id(&self) -> &str {
        match self {
            Self::Chest(menu) => &menu.id,
            Self::Form(form) => &form.id,
        }
    }

    pub fn state(&self) -> &BTreeMap<String, StateDefinition> {
        match self {
            Self::Chest(menu) => &menu.state,
            Self::Form(form) => &form.state,
        }
    }

    /// Menus visés par les actions `open` de l’écran, avec l’élément (slot ou bouton) qui les porte.
    fn opened_menus(&self) -> Vec<(&str, &str)> {
        let elements: Vec<(&str, &[Action])> = match self {
            Self::Chest(menu) => menu
                .slots
                .iter()
                .map(|slot| (slot.id.as_str(), slot.on_click.as_slice()))
                .collect(),
            Self::Form(form) => form
                .buttons
                .iter()
                .map(|button| (button.id.as_str(), button.on_click.as_slice()))
                .collect(),
        };
        elements
            .into_iter()
            .flat_map(|(element, actions)| {
                actions.iter().filter_map(move |action| match action {
                    Action::Open { menu, .. } => Some((element, menu.as_str())),
                    _ => None,
                })
            })
            .collect()
    }
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
    /// Formulaires Bedrock ; absents d’un descripteur plus ancien.
    #[serde(default)]
    forms: Vec<FormDescriptor>,
}

/// Menus chargés d’un descripteur, validés.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pack: Option<PackInfo>,
    warnings: Vec<String>,
    screens: Vec<Screen>,
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
        let chests = descriptor
            .menus
            .into_iter()
            .map(|menu| validate_menu(&menu).map(|()| Screen::Chest(Arc::new(menu))));
        let forms = descriptor
            .forms
            .into_iter()
            .map(|form| validate_form(&form).map(|()| Screen::Form(Arc::new(form))));
        for screen in chests.chain(forms) {
            let screen = screen?;
            if catalog.index.contains_key(screen.id()) {
                return Err(format!("menu « {} » présent deux fois", screen.id()));
            }
            catalog
                .index
                .insert(screen.id().to_string(), catalog.screens.len());
            catalog.screens.push(screen);
        }

        let known: BTreeSet<&str> = catalog.index.keys().map(String::as_str).collect();
        let mut missing = Vec::new();
        for screen in &catalog.screens {
            for (element, target) in screen.opened_menus() {
                if !known.contains(target) {
                    missing.push(format!(
                        "{} : « {element} » ouvre le menu « {target} », absent",
                        screen.id()
                    ));
                }
            }
        }
        catalog.warnings.extend(missing);
        Ok(catalog)
    }

    /// Menu coffre `id` (pas un formulaire).
    pub fn menu(&self, id: &str) -> Option<Arc<MenuDescriptor>> {
        match self.screen(id) {
            Some(Screen::Chest(menu)) => Some(menu),
            _ => None,
        }
    }

    /// Formulaire Bedrock `id`.
    pub fn form(&self, id: &str) -> Option<Arc<FormDescriptor>> {
        match self.screen(id) {
            Some(Screen::Form(form)) => Some(form),
            _ => None,
        }
    }

    /// Écran `id`, menu coffre ou formulaire.
    pub fn screen(&self, id: &str) -> Option<Screen> {
        self.index.get(id).map(|&index| self.screens[index].clone())
    }

    /// Nombre de formulaires Bedrock.
    pub fn form_count(&self) -> usize {
        self.screens
            .iter()
            .filter(|screen| matches!(screen, Screen::Form(_)))
            .count()
    }

    /// Identifiants des menus, triés.
    pub fn menu_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.index.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn len(&self) -> usize {
        self.screens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.screens.is_empty()
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
        let forms = match self.form_count() {
            0 => String::new(),
            count => format!(" dont {count} formulaire(s) Bedrock"),
        };
        format!(
            "{} menu(s) chargé(s){forms}{pack} : {}",
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

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Formulaire Bedrock : identifiant, disposition connue et drapeau du pack
/// `mcrs_ui`, boutons d’identifiants distincts, images non vides.
fn validate_form(form: &FormDescriptor) -> Result<(), String> {
    let id = &form.id;
    if !valid_id(id) {
        return Err(format!("identifiant de menu invalide : {id:?}"));
    }
    match layout_flag(&form.layout) {
        None => {
            let layouts: Vec<&str> = FORM_LAYOUTS.iter().map(|(name, _)| *name).collect();
            return Err(format!(
                "{id} : disposition « {} » inconnue (attendu : {})",
                form.layout,
                layouts.join(", ")
            ));
        }
        Some(flag) if flag != form.flag => {
            return Err(format!(
                "{id} : drapeau {:?} de la disposition {} différent de celui du pack mcrs_ui ({flag:?})",
                form.flag, form.layout
            ));
        }
        Some(_) => {}
    }
    let mut buttons = BTreeSet::new();
    for button in &form.buttons {
        if !buttons.insert(button.id.as_str()) {
            return Err(format!("{id} : deux boutons « {} »", button.id));
        }
        if button
            .image
            .as_ref()
            .is_some_and(|image| image.data.trim().is_empty())
        {
            return Err(format!(
                "{id} : image vide pour le bouton « {} »",
                button.id
            ));
        }
    }
    Ok(())
}
