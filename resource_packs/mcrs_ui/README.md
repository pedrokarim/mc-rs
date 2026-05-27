# mc-rs UI Pack

Custom Bedrock resource pack servi par le serveur **mc-rs**. Implémente un
dispatcher d'interfaces qui transforme un `ModalFormRequest` standard en
un layout custom selon un **drapeau invisible dans le titre**.

> Pour la documentation complète, voir :
> - [Pipeline & architecture serveur](../../new-docs/16-RESOURCE-PACK-PIPELINE.md)
> - [Catalogue des 8 layouts + tutoriel](../../new-docs/17-UI-LAYOUTS.md)

---

## Aperçu rapide

**8 layouts custom** activables via un préfixe `§m§<flag>` dans le titre
d'un form Bedrock standard :

| Flag | Layout | Cas d'usage |
|------|--------|-------------|
| `§m§a` | `grid` | Menu d'actions (par défaut) |
| `§m§b` | `left_button` | Settings, navigation à 2 panneaux |
| `§m§c` | `bottom_button` | Présentation d'un mode de jeu |
| `§m§d` | `image_grid` | Sélecteur de map / arène |
| `§m§e` | `square_image` | Annonce avec image carrée |
| `§m§f` | `motd` | Message du jour |
| `§m§0` | `store` | Boutique avec onglets |
| `§m§1` | `wrapped` | Récap avec visuels défilants |

**4 overrides vanilla** : `chest_screen`, `hud_screen`, `scoreboards`,
`ui_common` (notifications custom, scoreboard sidebar, etc.).

---

## Structure

```
mcrs_ui/
├── manifest.json              UUID, version (à bumper après chaque édition JSON)
├── pack_icon.png              Icône affichée dans Settings client
├── font/glyph_E1.png          Glyphes custom (range U+E100..U+E1FF)
├── textures/                  Assets (icônes, boutons, bordures…)
│   └── ui/mcrs/panels/        Textures spécifiques aux layouts
└── ui/
    ├── _global_variables.json Définit les 8 flags $flag_* + palette mc-rs
    ├── _ui_defs.json          Liste les JSON UI custom à charger
    ├── server_form.json       Dispatcher : route un form vers un layout
    ├── chest_screen.json      Override large-chest (marker @@@@)
    ├── hud_screen.json        Override notifications + titres
    ├── scoreboards.json       Override sidebar
    ├── ui_common.json         Override scrollbar
    └── mcrs/server_form/      8 layouts custom (1 par flag)
```

---

## Tester en jeu

Une fois le serveur lancé :

```
/menu              ouvre le menu hub (layout grid)
/menu showcase     liste les 8 layouts cliquables
/menu motd         ouvre directement la démo `motd`
/menu store        ouvre directement la démo `store`
…                  autocomplete propose les 10 valeurs
```

---

## Ajouter un layout

1. Choisir un flag libre (`§m§2`, `§m§3`, …)
2. Écrire le panel JSON sous `ui/mcrs/server_form/<nom>_panel.json`
3. Déclarer dans `_ui_defs.json`
4. Ajouter une variante dans `server_form.json` (matching binding + fallback)
5. Builder Rust côté `connection/forms.rs`
6. Mapper le nom dans `Connection::DEMO_PANEL_NAMES` + `build_demo_panel_batch`
7. Bumper la version dans `manifest.json` (sinon caching client)

→ Détail pas-à-pas : [17 — UI Layouts §10](../../new-docs/17-UI-LAYOUTS.md#10--tutoriel--créer-son-propre-layout)

---

## Conventions de format

Le `text` d'un bouton peut transporter plusieurs infos :

| Pattern | Effet |
|---------|-------|
| `"Mon bouton"` | Bouton normal |
| `"Titre\tSous-titre"` | Sépare titre / description / prix (selon layout) |
| `"§m§a Mon texte"` | Entrée non cliquable (bannière, catégorie, image) |
| `"§m§b Mon texte"` | Bouton style spécial (violet, layouts `left_button`/`bottom_button`) |

Image attachée à un bouton :

```json
{ "text": "Forêt\tFacile",
  "image": { "type": "path", "data": "textures/ui/mcrs/panels/loading_grid" } }
```

→ Référence complète des bindings exposés : [17 — UI Layouts §5](../../new-docs/17-UI-LAYOUTS.md#5-référence-des-bindings-exposés-au-moteur)

---

## Origine

Le mécanisme du title-flag dispatcher est une technique éprouvée de la
communauté UI Bedrock. Le pack mcrs_ui propose une implémentation maison
adaptée à mc-rs.
