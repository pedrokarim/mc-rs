//! Session d’un joueur dans les menus Menu Forge : pile des menus ouverts,
//! état, composition du formulaire et exécution des clics.
//!
//! Composition (contrat, § 3 et § 4) : le titre porte le drapeau, le jeton du
//! menu et les jetons des éléments figés conditionnels visibles ; les entrées
//! sont les `9 × lignes` cases du coffre, puis les textes dynamiques. Après un
//! clic, le client a déjà fermé le formulaire : sauf fermeture, la session
//! renvoie le menu du haut de la pile, recomposé (le menu Java « reste ouvert »).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::catalog::{
    Action, ButtonImage, Catalog, CommandRunner, FormDescriptor, MenuDescriptor, Screen, SlotKind,
    StateDefinition, StateValue,
};
use super::condition::{evaluate, ConditionContext};
use super::{INERT_MARKER, MENU_FORGE_FLAG, TEXT_MARKER};
use crate::form_api::{Form, SimpleButton, SimpleButtonImage};

/// Colonnes de la grille du coffre.
pub const COLUMNS: usize = 9;
/// Profondeur maximale de la pile `open` / `back` (les plus anciens menus sont oubliés).
pub const MAX_STACK: usize = 16;
const RESET: &str = "\u{00A7}r";

/// Joueur qui voit le menu : drapeaux (`viewer.*`) et variables fournis par le serveur.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Viewer {
    pub flags: BTreeSet<String>,
    pub variables: BTreeMap<String, String>,
}

impl Viewer {
    /// Joueur Bedrock : `{viewer.name}`, `{viewer.uuid}` et le drapeau `viewer.bedrock`.
    pub fn new(name: &str, uuid: &str) -> Self {
        Self {
            flags: BTreeSet::from(["viewer.bedrock".to_string()]),
            variables: BTreeMap::from([
                ("viewer.name".to_string(), name.to_string()),
                ("viewer.uuid".to_string(), uuid.to_string()),
            ]),
        }
    }

    pub fn with_flag(mut self, flag: &str) -> Self {
        self.flags.insert(flag.to_string());
        self
    }

    pub fn with_variable(mut self, name: &str, value: impl Into<String>) -> Self {
        self.variables.insert(name.to_string(), value.into());
        self
    }
}

/// Élément d’une liste fournie par le serveur (`slots[].kind = "list"`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ListItem {
    pub label: String,
    pub icon: Option<String>,
    pub on_click: Vec<Action>,
}

/// Sources de données des listes, fournies par le serveur.
pub trait Lists {
    fn items(&self, list: &str, viewer: &Viewer) -> Vec<ListItem>;
}

/// Aucune source : les listes restent vides.
pub struct NoLists;

impl Lists for NoLists {
    fn items(&self, _list: &str, _viewer: &Viewer) -> Vec<ListItem> {
        Vec::new()
    }
}

/// Une entrée (un bouton) du formulaire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormEntry {
    pub text: String,
    /// Image du bouton (`path` ou `url`).
    pub image: Option<ButtonImage>,
}

/// Formulaire composé, prêt à envoyer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedForm {
    /// Écran composé (menu coffre ou formulaire Bedrock).
    pub menu: String,
    pub title: String,
    /// Texte de contenu (`#form_text`), vide pour un menu coffre.
    pub content: String,
    pub buttons: Vec<FormEntry>,
}

impl ComposedForm {
    pub fn to_form(&self) -> Form {
        Form::Simple {
            title: self.title.clone(),
            content: self.content.clone(),
            buttons: self
                .buttons
                .iter()
                .map(|entry| SimpleButton {
                    text: entry.text.clone(),
                    image: entry.image.as_ref().map(|image| SimpleButtonImage {
                        image_type: image.type_name().to_string(),
                        data: image.data.clone(),
                    }),
                })
                .collect(),
        }
    }

    pub fn to_json(&self) -> String {
        self.to_form().to_json()
    }
}

/// Ce que le serveur doit faire après un clic, dans l’ordre.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Envoyer ce formulaire (toujours le dernier effet, sauf fermeture).
    Show(ComposedForm),
    Command {
        line: String,
        runner: CommandRunner,
    },
    Sound {
        name: String,
        volume: f32,
        pitch: f32,
    },
    /// Action `custom`, transmise à l’adaptateur du serveur.
    Custom {
        id: String,
        args: serde_json::Value,
    },
    /// Anomalie à journaliser (menu cité absent, valeur d’état refusée…).
    Warning(String),
    /// La session est terminée.
    Closed,
}

/// Ce que désigne chaque entrée du dernier formulaire envoyé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Nothing,
    Slot(usize),
    ListItem {
        slot: usize,
        item: usize,
    },
    /// Bouton d’un formulaire Bedrock (rang dans `buttons` du descripteur).
    FormButton(usize),
}

#[derive(Debug, Clone)]
struct Frame {
    screen: Screen,
    state: BTreeMap<String, StateValue>,
}

impl Frame {
    /// État initial : valeurs par défaut, puis `initial` (ramené dans les bornes).
    fn new(screen: Screen, initial: &BTreeMap<String, StateValue>) -> (Self, Vec<String>) {
        let state = screen
            .state()
            .iter()
            .map(|(name, definition)| (name.clone(), initial_value(definition)))
            .collect();
        let mut frame = Self { screen, state };
        let refused = initial
            .iter()
            .filter(|(name, value)| !frame.set(name, (*value).clone()))
            .map(|(name, value)| format!("état « {name} » : valeur {value:?} refusée"))
            .collect();
        (frame, refused)
    }

    /// Change une variable d’état selon sa définition ; `false` si elle est refusée.
    fn set(&mut self, name: &str, value: StateValue) -> bool {
        let Some(definition) = self.screen.state().get(name) else {
            return false;
        };
        let accepted = match (definition, value) {
            (StateDefinition::Enum { values, .. }, StateValue::Text(text))
                if values.contains(&text) =>
            {
                StateValue::Text(text)
            }
            (StateDefinition::Bool { .. }, value @ StateValue::Bool(_)) => value,
            (StateDefinition::Int { min, max, .. }, StateValue::Number(number)) => {
                StateValue::Number(clamp_int(number, *min, *max))
            }
            (StateDefinition::Page { .. }, StateValue::Number(number)) => {
                StateValue::Number(number.round().max(1.0))
            }
            _ => return false,
        };
        self.state.insert(name.to_string(), accepted);
        true
    }

    /// Nom de la variable `page` qui pagine `list`.
    fn page_state(&self, list: &str) -> Option<String> {
        self.screen
            .state()
            .iter()
            .find_map(|(name, definition)| match definition {
                StateDefinition::Page { list: paged } if paged == list => Some(name.clone()),
                _ => None,
            })
    }
}

fn clamp_int(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let mut value = value.round();
    if let Some(min) = min {
        value = value.max(min);
    }
    if let Some(max) = max {
        value = value.min(max);
    }
    value
}

/// Valeur initiale (format : un `int` sans `default` part de `min`, sinon de 0).
fn initial_value(definition: &StateDefinition) -> StateValue {
    match definition {
        StateDefinition::Enum { default, .. } => StateValue::Text(default.clone()),
        StateDefinition::Bool { default } => StateValue::Bool(*default),
        StateDefinition::Int { default, min, max } => {
            StateValue::Number(clamp_int(default.or(*min).unwrap_or(0.0), *min, *max))
        }
        StateDefinition::Page { .. } => StateValue::Number(1.0),
    }
}

/// État, drapeaux, variables et listes d’un menu au moment de composer.
struct Evaluation {
    flags: BTreeSet<String>,
    variables: BTreeMap<String, String>,
    /// Liste → (page courante, éléments, capacité = cases des slots de cette liste).
    lists: BTreeMap<String, (usize, Vec<ListItem>, usize)>,
}

/// Remplace les variables `{nom}` connues (lettres, chiffres, `_ . : -`) ; les autres restent.
pub fn interpolate(template: &str, variables: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let name_end = after.find('}').filter(|&end| {
            end > 0
                && after[..end].chars().all(|char| {
                    char.is_ascii_alphanumeric() || matches!(char, '_' | '.' | ':' | '-')
                })
        });
        match name_end {
            Some(end) => {
                let name = &after[..end];
                match variables.get(name) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Libellé d’un bouton : jamais vide, jamais pris pour un marqueur.
fn button_label(label: &str, fallback: &str) -> String {
    let cleaned = label.replace('{', "(");
    if cleaned.trim().is_empty() {
        fallback.replace('{', "(")
    } else {
        cleaned
    }
}

/// Nom du menu dans le titre (repli vanilla), sans délimiteur de jeton.
fn fallback_name(name: &str) -> String {
    name.chars()
        .filter(|char| !matches!(char, '(' | ')' | '[' | ']'))
        .collect()
}

/// Session d’un joueur.
#[derive(Debug, Clone)]
pub struct Session {
    catalog: Arc<Catalog>,
    stack: Vec<Frame>,
    current: Frame,
    targets: Vec<Target>,
    list_items: BTreeMap<String, Vec<ListItem>>,
    pending_warnings: Vec<String>,
}

impl Session {
    /// Ouvre `menu_id` avec un état initial (valeurs refusées signalées au premier `compose`).
    pub fn open(
        catalog: Arc<Catalog>,
        menu_id: &str,
        initial: &BTreeMap<String, StateValue>,
    ) -> Result<Self, String> {
        let screen = catalog
            .screen(menu_id)
            .ok_or_else(|| format!("Menu Menu Forge inconnu : {menu_id}"))?;
        let (current, refused) = Frame::new(screen, initial);
        Ok(Self {
            catalog,
            stack: Vec::new(),
            current,
            targets: Vec::new(),
            list_items: BTreeMap::new(),
            pending_warnings: refused,
        })
    }

    /// Menu affiché.
    pub fn menu_id(&self) -> &str {
        self.current.screen.id()
    }

    /// Menus sous le menu affiché (retour possible).
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Avertissements accumulés depuis le dernier appel (état initial refusé…).
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_warnings)
    }

    fn evaluate(&mut self, viewer: &Viewer, lists: &dyn Lists) -> Evaluation {
        let screen = self.current.screen.clone();
        let mut flags = viewer.flags.clone();
        let mut variables = viewer.variables.clone();

        let mut collected: BTreeMap<String, (usize, Vec<ListItem>, usize)> = BTreeMap::new();
        let list_slots: Vec<_> = match &screen {
            Screen::Chest(menu) => menu
                .slots
                .iter()
                .filter(|slot| slot.kind == SlotKind::List)
                .collect(),
            Screen::Form(_) => Vec::new(),
        };
        for slot in list_slots {
            let Some(list) = slot.list.as_deref() else {
                continue;
            };
            let entry = collected
                .entry(list.to_string())
                .or_insert_with(|| (1, lists.items(list, viewer), 0));
            entry.2 += slot.cells.len();
        }
        for (list, (page, items, capacity)) in &mut collected {
            let count = items.len().div_ceil((*capacity).max(1)).max(1);
            if let Some(name) = self.current.page_state(list) {
                let wanted = match self.current.state.get(&name) {
                    Some(StateValue::Number(number)) => number.max(1.0) as usize,
                    _ => 1,
                };
                *page = wanted.clamp(1, count);
                self.current
                    .state
                    .insert(name.clone(), StateValue::Number(*page as f64));
                if *page > 1 {
                    flags.insert(format!("{name}.hasPrev"));
                }
                if *page < count {
                    flags.insert(format!("{name}.hasNext"));
                }
                variables.insert(format!("{name}.number"), page.to_string());
                variables.insert(format!("{name}.count"), count.to_string());
            }
        }
        // Page d’une liste sans aucun slot : une seule page.
        for (name, definition) in screen.state() {
            if let StateDefinition::Page { list } = definition {
                if !collected.contains_key(list) {
                    self.current
                        .state
                        .insert(name.clone(), StateValue::Number(1.0));
                    variables.insert(format!("{name}.number"), "1".to_string());
                    variables.insert(format!("{name}.count"), "1".to_string());
                }
            }
        }
        for (name, value) in &self.current.state {
            variables.insert(format!("state.{name}"), value.display());
        }
        Evaluation {
            flags,
            variables,
            lists: collected,
        }
    }

    /// Compose le formulaire du menu affiché et mémorise ce que désigne chaque entrée.
    pub fn compose(&mut self, viewer: &Viewer, lists: &dyn Lists) -> ComposedForm {
        let evaluation = self.evaluate(viewer, lists);
        match self.current.screen.clone() {
            Screen::Chest(menu) => self.compose_chest(&menu, evaluation),
            Screen::Form(form) => self.compose_form(&form, &evaluation),
        }
    }

    /// Menu coffre : une entrée par case du coffre, puis une par texte dynamique.
    fn compose_chest(&mut self, menu: &MenuDescriptor, evaluation: Evaluation) -> ComposedForm {
        let context = ConditionContext {
            state: &self.current.state,
            flags: &evaluation.flags,
        };
        let cells = COLUMNS * menu.rows;

        let mut title = format!("{MENU_FORGE_FLAG}{}", menu.token);
        for token in &menu.tokens {
            if evaluate(Some(&token.visible_when), &context) {
                title.push_str(&token.token);
            }
        }
        title.push_str(RESET);
        title.push_str(&fallback_name(&menu.name));

        let mut buttons = vec![FormEntry::default(); menu.entries];
        let mut targets = vec![Target::Nothing; menu.entries];
        let mut list_offsets: BTreeMap<&str, usize> = BTreeMap::new();
        for (index, slot) in menu.slots.iter().enumerate() {
            let visible = evaluate(slot.visible_when.as_ref(), &context);
            match slot.kind {
                SlotKind::Button | SlotKind::Decoration if visible => {
                    let active = slot.kind == SlotKind::Button
                        && evaluate(slot.enabled_when.as_ref(), &context);
                    let label =
                        button_label(&interpolate(&slot.label, &evaluation.variables), &slot.id);
                    let text = if active {
                        label
                    } else {
                        format!("{INERT_MARKER}{label}")
                    };
                    for &cell in slot.cells.iter().filter(|&&cell| cell < cells) {
                        buttons[cell] = FormEntry {
                            text: text.clone(),
                            image: slot.icon.clone().map(ButtonImage::path),
                        };
                        targets[cell] = if active {
                            Target::Slot(index)
                        } else {
                            Target::Nothing
                        };
                    }
                }
                SlotKind::List => {
                    let Some(list) = slot.list.as_deref() else {
                        continue;
                    };
                    // Une liste masquée consomme quand même sa part de la page.
                    let offset = list_offsets.entry(list).or_default();
                    let first = *offset;
                    *offset += slot.cells.len();
                    let Some((page, items, capacity)) = evaluation.lists.get(list) else {
                        continue;
                    };
                    if !visible {
                        continue;
                    }
                    let start = (page - 1) * capacity + first;
                    let enabled = evaluate(slot.enabled_when.as_ref(), &context);
                    for (position, &cell) in slot.cells.iter().enumerate() {
                        let Some(item) = items.get(start + position).filter(|_| cell < cells)
                        else {
                            continue;
                        };
                        let active = enabled && !item.on_click.is_empty();
                        let label = button_label(&item.label, &slot.id);
                        buttons[cell] = FormEntry {
                            text: if active {
                                label
                            } else {
                                format!("{INERT_MARKER}{label}")
                            },
                            image: item.icon.clone().map(ButtonImage::path),
                        };
                        targets[cell] = if active {
                            Target::ListItem {
                                slot: index,
                                item: start + position,
                            }
                        } else {
                            Target::Nothing
                        };
                    }
                }
                _ => {}
            }
        }
        for text in &menu.texts {
            if text.entry < buttons.len() && evaluate(text.visible_when.as_ref(), &context) {
                let value =
                    interpolate(&text.value, &evaluation.variables).replace(TEXT_MARKER, "(t");
                buttons[text.entry] = FormEntry {
                    text: format!("{}{value}", text.prefix),
                    image: None,
                };
            }
        }

        self.targets = targets;
        self.list_items = evaluation
            .lists
            .into_iter()
            .map(|(list, (_, items, _))| (list, items))
            .collect();
        ComposedForm {
            menu: menu.id.clone(),
            title,
            content: String::new(),
            buttons,
        }
    }

    /// Formulaire Bedrock : titre (drapeau de la disposition, une espace, titre),
    /// contenu, puis un bouton par entrée dont `visibleWhen` est vraie ; l’index
    /// d’un clic est le rang parmi ces boutons envoyés.
    fn compose_form(&mut self, form: &FormDescriptor, evaluation: &Evaluation) -> ComposedForm {
        let context = ConditionContext {
            state: &self.current.state,
            flags: &evaluation.flags,
        };
        let variables = &evaluation.variables;
        let mut buttons = Vec::new();
        let mut targets = Vec::new();
        for (index, button) in form.buttons.iter().enumerate() {
            if !evaluate(button.visible_when.as_ref(), &context) {
                continue;
            }
            buttons.push(FormEntry {
                text: interpolate(&button.text, variables),
                image: button.image.clone(),
            });
            targets.push(Target::FormButton(index));
        }
        let title = format!("{} {}", form.flag, interpolate(&form.title, variables));
        let content = interpolate(&form.content, variables);
        self.targets = targets;
        self.list_items.clear();
        ComposedForm {
            menu: form.id.clone(),
            title,
            content,
            buttons,
        }
    }

    /// Exécute le clic sur l’entrée `index` du dernier formulaire composé.
    /// Le dernier effet est `Show` (menu recomposé) ou `Closed`.
    pub fn click(&mut self, index: usize, viewer: &Viewer, lists: &dyn Lists) -> Vec<Effect> {
        let actions = match (&self.current.screen, self.targets.get(index).copied()) {
            (Screen::Chest(menu), Some(Target::Slot(slot))) => menu.slots[slot].on_click.clone(),
            (Screen::Chest(menu), Some(Target::ListItem { slot, item })) => menu.slots[slot]
                .list
                .as_ref()
                .and_then(|list| self.list_items.get(list))
                .and_then(|items| items.get(item))
                .map(|item| item.on_click.clone())
                .unwrap_or_default(),
            (Screen::Form(form), Some(Target::FormButton(button))) => {
                form.buttons[button].on_click.clone()
            }
            _ => Vec::new(),
        };

        let mut effects: Vec<Effect> = self
            .take_warnings()
            .into_iter()
            .map(Effect::Warning)
            .collect();
        let mut closed = false;
        for action in actions {
            match action {
                Action::Open { menu, state } => match self.catalog.screen(&menu) {
                    Some(target) => {
                        let (frame, refused) = Frame::new(target, &state);
                        effects.extend(refused.into_iter().map(Effect::Warning));
                        self.stack.push(std::mem::replace(&mut self.current, frame));
                        if self.stack.len() > MAX_STACK {
                            self.stack.remove(0);
                        }
                        closed = false;
                    }
                    None => effects.push(Effect::Warning(format!("menu « {menu} » introuvable"))),
                },
                Action::Back => match self.stack.pop() {
                    Some(previous) => {
                        self.current = previous;
                        closed = false;
                    }
                    None => closed = true,
                },
                Action::Close => closed = true,
                Action::SetState { state, value } => {
                    if !self.current.set(&state, value.clone()) {
                        effects.push(Effect::Warning(format!(
                            "{} : état « {state} », valeur {value:?} refusée",
                            self.current.screen.id()
                        )));
                    }
                }
                Action::NextPage { list } => self.turn_page(&list, 1.0),
                Action::PrevPage { list } => self.turn_page(&list, -1.0),
                Action::Sound {
                    sound,
                    volume,
                    pitch,
                } => effects.push(Effect::Sound {
                    name: sound,
                    volume: volume.unwrap_or(1.0),
                    pitch: pitch.unwrap_or(1.0),
                }),
                Action::Command { command, runner } => {
                    let variables = self.evaluate(viewer, lists).variables;
                    let line = interpolate(&command, &variables);
                    effects.push(Effect::Command {
                        line: line.trim().trim_start_matches('/').to_string(),
                        runner,
                    });
                }
                Action::Custom { id, args } => effects.push(Effect::Custom { id, args }),
            }
        }
        if closed {
            effects.push(Effect::Closed);
        } else {
            effects.push(Effect::Show(self.compose(viewer, lists)));
        }
        effects
    }

    /// Avance ou recule la page de `list` (bornée à la composition suivante).
    fn turn_page(&mut self, list: &str, delta: f64) {
        let Some(name) = self.current.page_state(list) else {
            return;
        };
        let page = match self.current.state.get(&name) {
            Some(StateValue::Number(number)) => *number,
            _ => 1.0,
        };
        self.current.set(&name, StateValue::Number(page + delta));
    }
}
