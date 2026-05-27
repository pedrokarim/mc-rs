# 17 — UI Layouts (catalogue + tutoriel)

> Documentation détaillée des **8 layouts custom** du pack `mcrs_ui`, et
> tutoriel pas-à-pas pour **créer son propre layout** from scratch.
>
> Prérequis : lire d'abord [16 — Resource Pack Pipeline](16-RESOURCE-PACK-PIPELINE.md)
> pour comprendre le title-flag dispatcher.

---

## 1. Vue d'ensemble

Tous les layouts custom partagent la même mécanique :

```
serveur                           pack mcrs_ui (client)
   │                                       │
   │ ModalFormRequest {                    │
   │   title: "§m§<flag> §6Mon titre"      │
   │   buttons: [ {text, image?}, ... ]    │
   │ }                                     │
   ├──────────────────────────────────────►│
   │                                       │
   │                                       │ server_form.json
   │                                       │   binding "(#title_text - $flag_X) ≠ #title_text"
   │                                       │     ↳ rend mcrs_X_modal.main_panel
   │                                       │
   │                                       │ mcrs_X_modal lit la collection form_buttons
   │                                       │ et applique le rendu spécifique au layout
```

**Côté serveur**, choisir un layout = choisir un préfixe de titre. Le contenu
JSON (`buttons[]`) reste celui d'un `ActionForm` Bedrock standard.

**Côté client**, chaque layout a son `mcrs_<nom>_modal.main_panel` qui :
- itère sur la collection globale `form_buttons` exposée par le moteur
- utilise `binding_collection_name: "form_buttons"` et `#form_button_text` /
  `#form_button_texture` pour lire chaque entrée
- décompose souvent `#form_button_text` avec des markers spéciaux
  (séparateur `\t`, drapeau `§m§a`, etc.) pour gérer plusieurs sous-rôles
  dans le même tableau

---

## 2. Conventions de format `#form_button_text`

Plusieurs layouts utilisent **le même string** pour transporter plusieurs
infos. Conventions communes :

### 2.1 Séparateur `\t` (titre / sous-titre / prix)

```
"Épée légendaire\t1500 coins"
       ↑                ↑
       │                └─ description / prix
       └─ titre principal
```

Les bindings côté JSON utilisent `'%.100s' * #form_button_text` pour extraire
les 100 premiers caractères (titre), puis soustraient pour obtenir le reste.

### 2.2 Drapeau `§m§a` en début de string (entrée "bannière")

```
"§m§a Mode Bedwars"
   ↑
   └─ Indique au layout que ce n'est pas un bouton cliquable normal mais
      un en-tête / bannière / catégorie / image décorative.
```

Le binding teste `((#form_button_text - "§m§a") ≠ #form_button_text)` pour
distinguer les deux types d'entrées dans la même collection.

⚠ Attention : ce flag `§m§a` à l'intérieur de `form_button_text` est
**séparé** du flag `§m§a` utilisé dans `#title_text` pour le dispatcher.
Ils utilisent la même séquence de caractères mais sont matchés contre
des champs différents.

### 2.3 Drapeau `§m§b` (bouton "spécial")

Certains layouts (left_button, bottom_button) supportent un deuxième style
de bouton — couleur violet/spéciale. Préfixer le texte par `§m§b` :

```
"§m§b ▶ Acheter Premium"
```

### 2.4 Images attachées (`image: { type, data }`)

L'`ActionForm` standard de Bedrock permet d'attacher une image à un bouton :

```json
{
  "text": "Forêt\tDifficulté facile",
  "image": {
    "type": "path",        // ou "url" pour DL depuis internet
    "data": "textures/ui/mcrs/panels/loading_grid"
  }
}
```

Les layouts `image_grid`, `square_image`, `motd`, `store`, `wrapped` ont
des slots dédiés pour afficher cette image. Les autres l'ignorent.

---

## 3. Catalogue des 8 layouts

> **Convention de lecture** : pour chaque layout, on donne le flag à mettre
> dans le titre, le namespace UI (utile pour debug), et l'aspect visuel.

### 3.1 `grid` — Grille verticale

- **Flag** : `§m§a`
- **Namespace** : `mcrs_grid_modal`
- **Fichier pack** : `ui/mcrs/server_form/button_grid_panel.json`
- **Quand l'utiliser** : menu principal, sélecteur d'actions, list de modes
- **Aspect** :

```
┌─────────────────────────────────────┐
│ MC-RS HUB                       [x] │
├─────────────────────────────────────┤
│ §7Choisis une action :              │
│                                     │
│ ┌──────────┐ ┌──────────┐           │
│ │ Action 1 │ │ Action 2 │           │
│ └──────────┘ └──────────┘           │
│ ┌──────────┐ ┌──────────┐           │
│ │ Action 3 │ │ Action 4 │           │
│ └──────────┘ └──────────┘           │
└─────────────────────────────────────┘
```

- **Format buttons** : simple `[{ "text": "..." }, ...]`. Une image
  optionnelle est affichée en haut du bouton, le texte en bas.
- **Conventions text** : `\t` pour titre/sous-titre, `§m§a` non utilisé ici.

### 3.2 `left_button` — Boutons à gauche, description à droite

- **Flag** : `§m§b`
- **Namespace** : `mcrs_left_button_modal`
- **Quand l'utiliser** : navigation principale d'un sous-système (settings,
  jeux, etc.), où chaque bouton nécessite un texte explicatif
- **Aspect** :

```
┌─────────────────────────────────────┐
│ Settings                        [x] │
├──────────────┬──────────────────────┤
│ ▶ Graphique │ Description du bouton│
│ ▶ Audio     │ sélectionné. Texte    │
│ ▶ Contrôles │ multi-lignes affiché  │
│ ▶ Compte    │ à droite quand on     │
│             │ survole un bouton.    │
└──────────────┴──────────────────────┘
```

- **Format buttons** :
  - Boutons normaux : `{ "text": "Titre du bouton" }`
  - Bannières (rendues spécialement) : `{ "text": "§m§a Titre" }`
  - Boutons spéciaux (violet) : `{ "text": "§m§b Texte" }`

### 3.3 `bottom_button` — Bannière en haut, boutons en bas

- **Flag** : `§m§c`
- **Namespace** : `mcrs_bottom_button_modal`
- **Quand l'utiliser** : mise en scène d'un mode de jeu (visuel imposant
  en haut, actions en bas)
- **Aspect** :

```
┌─────────────────────────────────────┐
│ Mode BedWars                    [x] │
├─────────────────────────────────────┤
│ ┌─────────────┐ Description en      │
│ │  BANNIÈRE   │ panneau à droite    │
│ │   (image)   │ avec scroll si      │
│ └─────────────┘ besoin.             │
├─────────────────────────────────────┤
│ ┌─────────────────────────────────┐ │
│ │ ▶ Rejoindre solo                │ │
│ │ ▶ Rejoindre duo                 │ │
│ │ ▶ Rejoindre squad               │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

- **Format buttons** : premier élément `§m§a` (bannière haute, avec image
  si fournie), puis les boutons cliquables. Drapeaux normal/`§m§b` (spécial)
  supportés comme dans `left_button`.

### 3.4 `image_grid` — Grille d'images

- **Flag** : `§m§d`
- **Namespace** : `mcrs_image_grid_modal`
- **Quand l'utiliser** : sélecteur de map, sélecteur d'arène, n'importe
  quoi où l'image est l'info principale et le texte est secondaire
- **Aspect** :

```
┌─────────────────────────────────────┐
│ Sélectionne une map             [x] │
├─────────────────────────────────────┤
│ ┌───────┐ ┌───────┐ ┌───────┐       │
│ │ IMG 1 │ │ IMG 2 │ │ IMG 3 │       │
│ │  ↓    │ │  ↓    │ │  ↓    │       │
│ │ Titre │ │ Titre │ │ Titre │       │
│ └───────┘ └───────┘ └───────┘       │
└─────────────────────────────────────┘
```

- **Format buttons** : chaque entrée doit avoir une `image`. Le `text` peut
  contenir `Titre\tTag` — le `Tag` est affiché en pastille en haut du bouton
  (utile pour "DIFFICILE", "NOUVEAU", etc.).

### 3.5 `square_image` — Image carrée centrale

- **Flag** : `§m§e`
- **Namespace** : `mcrs_square_image_modal`
- **Quand l'utiliser** : annonce visuelle plein-écran (event, récap,
  célébration), description en bas
- **Aspect** :

```
┌─────────────────────────────────────┐
│      Titre centré          [x]      │
│                                     │
│      ┌─────────────────┐            │
│      │                 │            │
│      │   IMAGE CARRÉE  │            │
│      │      large      │            │
│      │                 │            │
│      └─────────────────┘            │
│                                     │
│ Description courte au-dessous.      │
└─────────────────────────────────────┘
```

- **Format buttons** : un seul élément avec `§m§a` (bannière) + `image`.
  Le `content` du form est utilisé comme description.

### 3.6 `motd` — Bannière + texte + 2 boutons

- **Flag** : `§m§f`
- **Namespace** : `mcrs_motd_modal`
- **Quand l'utiliser** : message du jour, écran d'accueil, annonce
- **Aspect** :

```
┌───────────────────────────┐
│ Titre du message      [x] │
│ ┌───────────────────────┐ │
│ │     BANNIÈRE          │ │
│ └───────────────────────┘ │
│ Texte multi-lignes        │
│ avec scroll vertical      │
│ si dépassement.           │
│  ┌──────────┐┌──────────┐ │
│  │ Bouton 1 ││ Bouton 2 │ │
│  └──────────┘└──────────┘ │
└───────────────────────────┘
```

- **Format buttons** : un `§m§a` (bannière en haut, image obligatoire)
  + N boutons normaux (rendus en ligne horizontale en bas).
- **Champ `content`** : utilisé pour le texte central.

### 3.7 `store` — Onglets de catégories + grille de produits

- **Flag** : `§m§0`
- **Namespace** : `mcrs_store_modal`
- **Quand l'utiliser** : boutique, kits, cosmétiques
- **Aspect** :

```
┌─────────────────────────────────────┐
│           BOUTIQUE              [x] │
│ ┌─────┐ ┌─────┐ ┌─────┐             │
│ │Pop. │ │Nouv.│ │Promo│ ← onglets   │
│ └─────┘ └─────┘ └─────┘             │
│                                     │
│ ┌────────┐ ┌────────┐ ┌────────┐    │
│ │ Item 1 │ │ Item 2 │ │ Item 3 │    │
│ │ 1500c  │ │  800c  │ │ 2000c  │    │
│ └────────┘ └────────┘ └────────┘    │
└─────────────────────────────────────┘
```

- **Format buttons** : entrées `§m§a` (catégories en haut) + entrées
  normales (produits dans la grille). Le `text` du produit suit le format
  `Nom\tPrix`.
- **Champ `content`** : doit contenir le nombre de catégories préfixé
  d'une lettre, exemple `§m§a3populaire` (lu par le binding
  `((#form_text - 'a') * 1)` pour faire la séparation catégories/produits).

### 3.8 `wrapped` — Scroll d'images + URL + boutons

- **Flag** : `§m§1`
- **Namespace** : `mcrs_wrapped_modal`
- **Quand l'utiliser** : "Wrapped" annuel, récap de stats avec visuels
  défilants, partage d'un lien
- **Aspect** :

```
┌───────────────────────────┐
│      TON WRAPPED 2026 [x] │
│  https://mcrs.io/recap    │
│ ┌───────────────────────┐ │
│ │     IMAGE 1           │ │
│ │     (scroll vertical) │ │
│ │     IMAGE 2           │ │
│ │     IMAGE 3           │ │
│ └───────────────────────┘ │
│      ┌──────────────┐     │
│      │ Continuer    │     │
│      └──────────────┘     │
└───────────────────────────┘
```

- **Format buttons** : entrées `§m§a` (images dans le scroll) + entrées
  normales (boutons en bas).
- **Champ `content`** : URL affichée en haut (rendue cliquable).

---

## 4. Quel layout choisir ?

| Cas d'usage | Layout |
|---|---|
| Menu principal d'actions | `grid` |
| Settings / navigation à 2 panneaux | `left_button` |
| Présentation d'un mode de jeu | `bottom_button` |
| Sélecteur visuel (maps, arènes) | `image_grid` |
| Annonce plein-écran avec 1 image | `square_image` |
| Message d'accueil avec 2-3 actions | `motd` |
| Boutique avec catégories | `store` |
| Récap saisonnier avec visuels | `wrapped` |
| Confirmation simple oui/non | Form vanilla `modal` (pas custom) |
| Saisie de texte | Form vanilla `custom_form` (pas custom) |

⚠ Les deux derniers cas n'ont **pas de layout custom** car ils correspondent
à `ModalFormRequest` de type `modal` ou `custom_form` (pas `form`), qui ont
leur propre rendu côté pack (fallback vanilla actif par défaut).

---

## 5. Référence des bindings exposés au moteur

Pour chaque entrée de la collection `form_buttons`, le moteur expose :

| Binding | Type | Origine |
|---|---|---|
| `#form_button_text` | string | champ `text` du bouton |
| `#form_button_texture` | string | champ `image.data` du bouton |
| `#form_button_texture_file_system` | enum | champ `image.type` (`path`, `url`, `disk_file`) |
| `#form_button_click` | event | déclenché au clic, payload = index |
| `#collection_length` | int | nombre de boutons |
| `#form_button_length` | int | alias de `#collection_length` |

Bindings globaux (au niveau `main_panel`) :

| Binding | Type | Origine |
|---|---|---|
| `#title_text` | string | champ `title` du form |
| `#form_text` | string | champ `content` du form |
| `#form_button_length` | int | nombre de boutons |

---

## 6. Modifier l'apparence d'un layout

Tous les layouts utilisent le même fond `whitetransparency` (alpha 0.85, color
`[0.06, 0.06, 0.06]`) et la même bande dorée `[0.933, 0.819, 0.039]`. Pour
changer la palette :

1. Édite `_global_variables.json` :
   ```json
   {
     "$mcrs_panel_bg_color": [0.05, 0.05, 0.10],
     "$mcrs_button_default_color": [0.12, 0.12, 0.18],
     "$mcrs_button_hover_color": [0.22, 0.22, 0.32]
   }
   ```

2. Référence ces variables dans le panel JSON :
   ```json
   { "type": "image", "color": "$mcrs_panel_bg_color" }
   ```

Pour changer une **texture** spécifique (ex. la bordure dorée), édite le PNG
dans `textures/ui/mcrs/panels/`. Pas besoin de bump version manifest pour ça
**si** tu déconnectes/reconnectes — mais bump version dès que tu changes
un JSON (caching client agressif).

---

## 7. Couleurs et codes spéciaux disponibles

### 7.1 Codes Bedrock pour `text`

| Code | Effet |
|---|---|
| `§0` à `§f` | Couleurs (vanilla) |
| `§g` | Or (Bedrock-only) |
| `§h` à `§u` | Couleurs étendues 1.19+ |
| `§l` | Gras |
| `§o` | Italique |
| `§n` | Souligné |
| `§m` | Magique (anim aléatoire) — utilisé pour les flags |
| `§r` | Reset |

### 7.2 Drapeaux mc-rs internes

| Drapeau | Champ ciblé | Effet |
|---|---|---|
| `§m§a` … `§m§f`, `§m§0`, `§m§1` | `title` | Sélectionne le layout |
| `§m§a` (au début) | `form_button.text` | "Bannière" / entrée non cliquable |
| `§m§b` (au début) | `form_button.text` | Bouton style spécial (violet) |
| `\t` | `form_button.text` | Séparateur titre / sous-titre / prix |

---

## 8. Trois exemples complets (Rust → JSON envoyé)

### 8.1 Menu hub simple

```rust
let title = format!("{FLAG_GRID} §l§6mc-rs§r §eHUB");
let json = format!(r#"{{
  "type":"form",
  "title":"{title}",
  "content":"§7Choisis une action :",
  "buttons":[
    {{"text":"§a▶ Téléporter au spawn"}},
    {{"text":"§b▶ Mode Créatif"}}
  ]
}}"#);
```

### 8.2 MOTD avec bannière

```rust
let title = format!("{FLAG_MOTD} §6MOTD");
let json = format!(r#"{{
  "type":"form",
  "title":"{title}",
  "content":"§7Bienvenue sur mc-rs.\n§eAppuie pour continuer.",
  "buttons":[
    {{"text":"§m§a Banner", "image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},
    {{"text":"§eContinuer"}},
    {{"text":"§cQuitter"}}
  ]
}}"#);
```

### 8.3 Boutique avec 2 catégories + 3 produits

```rust
let title = format!("{FLAG_STORE} §6Store");
let json = format!(r#"{{
  "type":"form",
  "title":"{title}",
  "content":"§m§a2populaire",
  "buttons":[
    {{"text":"§m§a Populaire"}},
    {{"text":"§m§a Nouveautés"}},
    {{"text":"Épée\t§a1500c","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},
    {{"text":"Pioche\t§a800c","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},
    {{"text":"Cape\t§a3500c","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}}
  ]
}}"#);
```

---

## 9. Tester pendant le développement

La commande `/menu <panel>` ouvre directement un layout avec des données de
démo. Utile pour itérer rapidement sur le JSON :

```
/menu hub           ouvre le menu racine
/menu showcase      ouvre la liste des layouts
/menu grid          ouvre la démo `grid`
/menu motd          ouvre la démo `motd`
/menu store         ouvre la démo `store`
/menu wrapped       ouvre la démo `wrapped`
```

Workflow recommandé pour itérer sur un layout :

1. Modifier le JSON du panel dans `resource_packs/mcrs_ui/ui/mcrs/server_form/`
2. **Bumper la version** dans `manifest.json` (sinon le client sert son cache)
3. Restart serveur : `cargo build --release && /restart`
4. Se déconnecter/reconnecter (le pack se re-télécharge)
5. `/menu <layout>` pour voir le rendu

---

## 10. Tutoriel — Créer son propre layout

Imaginons qu'on veut un **9ᵉ layout** : un menu de pause avec un grand
encadré central et une liste de raccourcis clavier.

### 10.1 Choisir un flag libre

Les 8 flags actuels sont `§m§a..§m§f`, `§m§0`, `§m§1`. On prend `§m§2`
pour notre nouveau layout `pause_menu`.

### 10.2 Écrire le panel JSON

Crée `resource_packs/mcrs_ui/ui/mcrs/server_form/pause_menu_panel.json` :

```json
{
  "namespace": "mcrs_pause_modal",

  "main_panel": {
    "type": "panel",
    "size": ["100%", "100%"],
    "controls": [
      { "backdrop": {
          "type": "image",
          "texture": "textures/ui/mcrs/panels/whitetransparency",
          "color": [0, 0, 0], "alpha": 0.7,
          "size": ["100%", "100%"]
      }},
      { "frame": {
          "type": "panel",
          "anchor_from": "center", "anchor_to": "center",
          "size": [320, 220],
          "controls": [
            { "bg": {
                "type": "image",
                "texture": "textures/ui/mcrs/panels/mcrs_rounded_corners",
                "color": [0.06, 0.06, 0.06], "alpha": 0.95,
                "size": ["100%", "100%"]
            }},
            { "title": {
                "type": "label", "text": "#title_text",
                "color": [1.0, 0.85, 0.20], "font_scale_factor": 1.4,
                "anchor_from": "top_middle", "anchor_to": "top_middle",
                "offset": [0, 10],
                "bindings": [{ "binding_name": "#title_text" }]
            }},
            { "content_text": {
                "type": "label", "text": "#form_text",
                "color": "white", "size": ["90%", "default"],
                "anchor_from": "top_middle", "anchor_to": "top_middle",
                "offset": [0, 40], "text_alignment": "center",
                "bindings": [{ "binding_name": "#form_text", "binding_type": "global" }]
            }},
            { "buttons": {
                "type": "stack_panel", "orientation": "vertical",
                "anchor_from": "bottom_middle", "anchor_to": "bottom_middle",
                "offset": [0, -10], "size": ["80%", "100%c"],
                "factory": {
                  "name": "button_list_factory",
                  "control_name": "mcrs_pause_modal.button_entry"
                },
                "collection_name": "form_buttons",
                "bindings": [{ "binding_name": "#form_button_length",
                               "binding_name_override": "#collection_length" }]
            }}
          ]
      }}
    ]
  },

  "button_entry": {
    "type": "panel",
    "size": ["100%", 24],
    "controls": [{
      "btn@common.button": {
        "size": ["100%", "100% - 4px"],
        "$pressed_button_name": "button.form_button_click",
        "bindings": [{ "binding_type": "collection_details",
                       "binding_collection_name": "form_buttons" }],
        "controls": [
          { "default@mcrs_pause_modal.button_face": { "$state": "default" }},
          { "hover@mcrs_pause_modal.button_face":   { "$state": "hover" }},
          { "pressed@mcrs_pause_modal.button_face": { "$state": "pressed" }}
        ]
      }
    }]
  },

  "button_face": {
    "type": "image",
    "texture": "textures/ui/mcrs/panels/mcrs_rounded_corners",
    "$bg_color|default": [0.12, 0.12, 0.18],
    "variables": [
      { "requires": "($state = 'hover')",   "$bg_color": [0.22, 0.22, 0.32] },
      { "requires": "($state = 'pressed')", "$bg_color": [0.30, 0.25, 0.05] }
    ],
    "color": "$bg_color", "size": ["100%", "100%"],
    "controls": [{
      "label": {
        "type": "label", "text": "#form_button_text",
        "anchor_from": "center", "anchor_to": "center",
        "bindings": [{ "binding_name": "#form_button_text",
                       "binding_type": "collection",
                       "binding_collection_name": "form_buttons" }]
      }
    }]
  }
}
```

### 10.3 Déclarer le panel dans `_ui_defs.json`

```json
{
  "ui_defs": [
    "ui/mcrs/server_form/left_button_panel.json",
    "ui/mcrs/server_form/bottom_button_panel.json",
    "ui/mcrs/server_form/button_grid_panel.json",
    "ui/mcrs/server_form/button_image_grid_panel.json",
    "ui/mcrs/server_form/square_image_panel.json",
    "ui/mcrs/server_form/motd_panel.json",
    "ui/mcrs/server_form/store_panel.json",
    "ui/mcrs/server_form/wrapped_panel.json",
    "ui/mcrs/server_form/pause_menu_panel.json"   ← ajout
  ]
}
```

### 10.4 Ajouter le flag dans `server_form.json`

Dans `mcrs_switching_long_form.controls`, ajouter une variante :

```json
{
  "mcrs_pause_menu@mcrs_pause_modal.main_panel": {
    "enabled": false, "visible": false,
    "$flag_pause": "§m§2",
    "bindings": [
      { "binding_type": "global", "binding_condition": "none",
        "binding_name": "#title_text", "binding_name_override": "#title_text" },
      { "source_property_name": "(not ((#title_text - $flag_pause) = #title_text))",
        "binding_type": "view", "target_property_name": "#visible" },
      { "source_property_name": "(not ((#title_text - $flag_pause) = #title_text))",
        "binding_type": "view", "target_property_name": "#enabled" }
    ]
  }
}
```

Et **étendre la négation du fallback `long_form@long_form`** pour inclure
`$flag_pause` dans le `and` :

```
(((#title_text - $flag_grid) = #title_text) and ... and ((#title_text - $flag_pause) = #title_text))
```

### 10.5 Ajouter le builder Rust

Dans `crates/mc-rs-server/src/connection/forms.rs` :

```rust
const FLAG_PAUSE: &str = "\u{00A7}m\u{00A7}2";

#[derive(Debug, Clone, Copy)]
pub enum PendingFormKind {
    // … existants
    DemoPause,
}

pub fn build_demo_pause_batch(&mut self) -> Vec<u8> {
    let title = format!("{FLAG_PAUSE} §6Pause");
    let json = format!(r#"{{
      "type":"form",
      "title":"{title}",
      "content":"§7Le jeu est en pause.",
      "buttons":[
        {{"text":"§aReprendre"}},
        {{"text":"§eOptions"}},
        {{"text":"§cQuitter"}}
      ]
    }}"#);
    self.encode_form(PendingFormKind::DemoPause, &json)
}
```

### 10.6 Brancher dans le routing

Dans `handle_modal_form_response`, traite `PendingFormKind::DemoPause` :

```rust
PendingFormKind::DemoPause => {
    let cmd = match index {
        0 => None,                              // reprendre = juste fermer
        1 => Some("menu showcase".into()),      // options = ouvrir un autre menu
        2 => Some("stop".into()),               // quitter
        _ => None,
    };
    if let Some(c) = cmd { self.pending_commands.push(c); }
    Vec::new()
}
```

### 10.7 Exposer en autocomplete `/menu`

Ajouter `"pause"` à `Connection::DEMO_PANEL_NAMES` et mapper dans
`build_demo_panel_batch` :

```rust
pub const DEMO_PANEL_NAMES: &'static [&'static str] = &[
    "hub", "showcase", "grid", "left_button", "bottom_button",
    "image_grid", "square_image", "motd", "store", "wrapped",
    "pause",   // ← ajout
];

pub fn build_demo_panel_batch(&mut self, panel: &str) -> Option<Vec<u8>> {
    Some(match panel {
        // … existants
        "pause" => self.build_demo_pause_batch(),
        _ => return None,
    })
}
```

### 10.8 Tester

```
cargo build --release
/restart
# Reconnecte-toi en jeu
/menu pause
```

Le menu de pause apparaît avec les 3 boutons.

### 10.9 Checklist de debug si rien ne s'affiche

1. **Pack mis à jour ?** Bumpe `version` dans `manifest.json`, sinon le client
   sert son cache.
2. **JSON valide ?** `python -m json.tool resource_packs/mcrs_ui/ui/mcrs/server_form/pause_menu_panel.json`
3. **Flag dans le binding ?** Vérifie que la négation du fallback inclut
   ton flag — sinon le `long_form` vanilla s'affiche par-dessus.
4. **Texture existe ?** Toutes les `texture` référencées doivent pointer
   vers un fichier qui existe (`.png` ou `.tga`).
5. **Logs serveur** : pas d'erreur côté logs ? Sinon la mauvaise piste,
   le souci est côté client (Settings → Storage → My Packs → vérifie le
   pack actif).

---

## 11. Récap des fichiers à connaître

| Fichier | Rôle |
|---|---|
| `resource_packs/mcrs_ui/ui/_global_variables.json` | Définit les 8 flags `$flag_*` et la palette de couleurs |
| `resource_packs/mcrs_ui/ui/_ui_defs.json` | Liste les panels custom à charger |
| `resource_packs/mcrs_ui/ui/server_form.json` | Dispatcher (route un form vers un layout selon le flag du titre) |
| `resource_packs/mcrs_ui/ui/mcrs/server_form/*.json` | 8 panels custom, un par layout |
| `resource_packs/mcrs_ui/textures/ui/mcrs/panels/` | Textures partagées (rounded corners, special_button, etc.) |
| `crates/mc-rs-server/src/connection/forms.rs` | Builders Rust + routing des `ModalFormResponse` |
| `crates/mc-rs-server/src/commands/menu.rs` | Commande `/menu <panel>` avec autocomplete |
| `new-docs/16-RESOURCE-PACK-PIPELINE.md` | Pipeline serveur (zip, chunks, SHA) — prérequis |
