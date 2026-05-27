---
layout: default
title: Resource Pack & Custom UI
nav_order: 2
permalink: /dev/resource-pack-ui/
---

# Resource Pack & Custom UI

Le serveur **mc-rs** embarque un resource pack (`resource_packs/mcrs_ui`) qui
est servi automatiquement à chaque client à la connexion. Ce pack apporte :

- **8 layouts custom** pour les `ModalForm` (menus à boutons stylés)
- **4 overrides** d'écrans vanilla (chest, hud, scoreboards, ui_common)
- Une commande `/menu [panel]` avec autocomplete pour ouvrir n'importe quel layout

Cette page est une **vue d'ensemble**. La documentation détaillée est dans :

- [`new-docs/16-RESOURCE-PACK-PIPELINE.md`](https://github.com/pedrokarim/mc-rs/blob/main/new-docs/16-RESOURCE-PACK-PIPELINE.md) — pipeline serveur, format binaire, flow complet d'une session
- [`new-docs/17-UI-LAYOUTS.md`](https://github.com/pedrokarim/mc-rs/blob/main/new-docs/17-UI-LAYOUTS.md) — catalogue détaillé + tutoriel "créer son propre layout"
- [`resource_packs/mcrs_ui/README.md`](https://github.com/pedrokarim/mc-rs/blob/main/resource_packs/mcrs_ui/README.md) — quickstart au niveau du pack

---

## Principe : title-flag dispatcher

Bedrock ne supporte que **3 types** de `ModalForm` officiels : `form`,
`modal`, `custom_form`. Pour avoir des layouts visuellement différents sans
sortir de cette API, mc-rs utilise une astuce répandue dans la scène
Bedrock :

> **Injecter un drapeau invisible dans le titre du form.** Le pack regarde
> ce drapeau et rend un layout différent en conséquence.

```
serveur                              client (pack mcrs_ui)
   │                                       │
   │  ModalFormRequest {                   │
   │    title: "§m§f §6Bienvenue !"        │
   │    content: "Texte d'accueil",        │
   │    buttons: [ ... ]                   │
   │  }                                    │
   ├──────────────────────────────────────►│
   │                                       │
   │                                       │ server_form.json
   │                                       │   "(#title_text - $flag_motd) ≠ #title_text"
   │                                       │     ↳ rend `mcrs_motd_modal.main_panel`
   │                                       │
   │                                       │ → écran "MOTD" custom avec
   │                                       │   bannière + texte + boutons
```

Le drapeau `§m§<X>` exploite le code Minecraft `§m` (animation "magique")
qui rend les caractères suivants illisibles à l'œil, mais reste matchable
au niveau du texte brut côté binding UI.

---

## Les 8 layouts

| Drapeau | Layout | Aperçu |
|---------|--------|--------|
| `§m§a` | **grid** | Grille verticale de boutons (menu d'actions) |
| `§m§b` | **left_button** | Boutons à gauche, description à droite |
| `§m§c` | **bottom_button** | Bannière haut + boutons bas |
| `§m§d` | **image_grid** | Grille d'images (sélecteur de map) |
| `§m§e` | **square_image** | Image carrée centrale + description |
| `§m§f` | **motd** | Bannière + texte + 2-3 boutons |
| `§m§0` | **store** | Onglets de catégories + grille produits |
| `§m§1` | **wrapped** | Scroll d'images + URL (style récap) |

Chaque layout a :
- Un **namespace UI** (`mcrs_<nom>_modal`)
- Un **panel JSON** dans `ui/mcrs/server_form/`
- Un **builder Rust** dans `connection/forms.rs`

---

## Tester

```sh
# Une fois connecté en jeu :
/menu                # menu hub
/menu showcase       # liste des 8 layouts (cliquables pour explorer)
/menu grid           # ouvre directement la démo `grid`
/menu motd           # ouvre la démo `motd`
/menu store          # ouvre la démo `store`
# … autocomplete propose les 10 valeurs
```

L'arbre de navigation :

```
/menu                  /menu <panel>
  │                          │
  ▼                          ▼
HubMenu                 (ouvre directement)
  ├─ 0..5 commandes vanilla
  └─ 6 → UiShowcase
            ├─ grid / left_button / bottom_button
            ├─ image_grid / square_image / motd
            ├─ store / wrapped
            └─ ↩ retour HubMenu
```

---

## Format d'un bouton

Le `text` d'un bouton est un canal de communication multi-usage :

```json
{ "text": "Mon bouton" }                       // bouton normal
{ "text": "Épée\t§a1500 coins" }                // séparateur \t = titre/sous-titre
{ "text": "§m§a Bannière non cliquable" }       // drapeau §m§a = en-tête / image
{ "text": "§m§b Bouton style spécial" }         // drapeau §m§b = bouton violet
```

Image attachée :

```json
{
  "text": "Forêt\tDifficulté facile",
  "image": { "type": "path", "data": "textures/ui/mcrs/panels/loading_grid" }
}
```

---

## Pipeline serveur (résumé)

1. **Boot** : `discover_packs(resource_packs/)` charge chaque dossier en
   zippant en mémoire (deflate), calcule un SHA-256 raw.
2. **Login** : envoie `ResourcePacksInfo` avec `must_accept=true` + métadonnées.
3. **DL** : le client demande `chunk_index` par `chunk_index`, on sert depuis
   la mémoire (1 MB par chunk).
4. **Apply** : envoie `ResourcePackStack` puis attend `COMPLETED`. Le pack
   est alors actif côté client, les overrides UI s'appliquent.
5. **Forms** : `/menu` construit un `ModalFormRequest` avec le bon drapeau
   et envoie. La réponse arrive en `ModalFormResponse` et est routée selon
   la `PendingFormKind` qu'on a mémorisée à l'émission.

---

## Créer un layout custom

Le tutoriel pas-à-pas est dans
[`17-UI-LAYOUTS.md §10`](https://github.com/pedrokarim/mc-rs/blob/main/new-docs/17-UI-LAYOUTS.md#10--tutoriel--créer-son-propre-layout).

Aperçu des étapes :

1. Choisir un flag libre (`§m§2`, `§m§3`, …)
2. Écrire le JSON `ui/mcrs/server_form/<nom>_panel.json` (namespace
   `mcrs_<nom>_modal` avec un `main_panel` qui itère sur la collection
   `form_buttons`)
3. Ajouter le JSON à `_ui_defs.json`
4. Ajouter une variante au dispatcher dans `server_form.json` :
   ```json
   "mcrs_<nom>@mcrs_<nom>_modal.main_panel": {
     "enabled": false, "visible": false,
     "$flag_<nom>": "§m§2",
     "bindings": [
       { "binding_type": "global", "binding_name": "#title_text" },
       { "source_property_name": "(not ((#title_text - $flag_<nom>) = #title_text))",
         "binding_type": "view", "target_property_name": "#visible" }
     ]
   }
   ```
   Et **étendre le `and` négatif du fallback `long_form`** pour exclure
   ton flag (sinon les deux variantes s'affichent).
5. Builder Rust :
   ```rust
   const FLAG_PAUSE: &str = "§m§2";
   pub fn build_demo_pause_batch(&mut self) -> Vec<u8> {
       let title = format!("{FLAG_PAUSE} §6Pause");
       let json = format!(r#"{{"type":"form","title":"{title}",
           "content":"§7Le jeu est en pause.",
           "buttons":[{{"text":"§aReprendre"}},{{"text":"§cQuitter"}}]}}"#);
       self.encode_form(PendingFormKind::DemoPause, &json)
   }
   ```
6. Exposer dans `DEMO_PANEL_NAMES` + `build_demo_panel_batch` pour
   l'autocomplete `/menu`.
7. **Bumper `manifest.json` version** sinon le client sert son cache.

---

## Pièges classiques

| Symptôme | Cause | Solution |
|----------|-------|----------|
| Le pack apparaît dans Settings mais aucun layout custom ne s'affiche | `must_accept=false` côté `ResourcePacksInfo` | Forcer `must_accept=true` (déjà fait dans `connection/login.rs`) |
| Modification JSON ignorée par le client | Cache client par UUID + version | Bumper `manifest.json` version, ou changer l'UUID du header pour reset complet |
| Texture introuvable → "Échec du chargement du pack" | Renomage de texture sans MAJ des refs JSON | `grep -r "ancienne_texture" resource_packs/mcrs_ui/ui/` pour trouver tous les sites d'appel |
| Le layout custom apparaît en plus du fallback vanilla | Le `and` négatif du `long_form` ne couvre pas le nouveau flag | Ajouter `and ((#title_text - $flag_<nouveau>) = #title_text)` dans la condition |
| Le client se déconnecte juste après "Resource packs completed" | `sha256` envoyé en hex au lieu de raw bytes | Utiliser `pack.sha256: [u8; 32]` directement |

---

## Origine

Le mécanisme du **title-flag dispatcher** est une technique éprouvée de la
scène UI Bedrock — utilisée par plusieurs serveurs publics, et adoptée ici
pour mc-rs.
