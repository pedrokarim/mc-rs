# 16 — Resource Pack Pipeline & Server-Driven UI

> Documentation complète du système qui sert un resource pack au client Bedrock
> et qui pilote un menu (ActionForm) stylé via un dispatcher UI côté pack.
>
> Sujets : pipeline serveur (zip, chunks, SHA, encryption), structure d'un pack
> côté disque, mécanisme du **title-flag dispatcher**, boucle
> `/menu` ↔ `ModalFormResponse`, outils annexes (`dump_pack`, `decrypt_pack`).
>
> **Pour la documentation détaillée de chaque layout custom (`grid`, `motd`,
> `store`, etc.) et le tutoriel "comment créer son propre layout", voir
> [17 — UI Layouts](17-UI-LAYOUTS.md).**

---

## 1. Vue d'ensemble

```
                         ┌──────────────────────┐
                         │  resource_packs/     │
                         │   └── mcrs_ui/       │  ← pack monté sur disque
                         │       ├── manifest.json
                         │       ├── pack_icon.png
                         │       ├── ui/...
                         │       └── textures/...
                         └──────────┬───────────┘
                                    │ discover_packs() au boot
                                    ▼
┌────────────────────┐    Arc<Vec<ResourcePack>>    ┌──────────────────────┐
│  mc-rs-server      │ ──────────────────────────── │  Connection (per cli)│
│  main.rs           │                              │  send_resource_packs │
│                    │                              │  send_pack_data_info │
└────────────────────┘                              │  serve_chunks(...)   │
                                                    └──────────┬───────────┘
                                                               │ Bedrock proto
                                                               ▼
                                                    ┌──────────────────────┐
                                                    │  Client Bedrock      │
                                                    │  télécharge, vérifie │
                                                    │  SHA-256, applique   │
                                                    │  le pack à la session│
                                                    └──────────────────────┘
```

Le serveur :
1. **charge** les packs depuis `resource_packs/` au boot
2. **zippe** chaque pack en mémoire (deflate)
3. **annonce** la liste dans `ResourcePacksInfo` au login
4. **sert** les chunks à la demande (`ResourcePackChunkRequest` → `ResourcePackChunkData`)
5. **active** les packs via `ResourcePackStack` quand le client confirme

Le client :
1. Reçoit `ResourcePacksInfo`, voit les UUIDs
2. Compare avec son cache local — si absent ou version différente, demande le DL
3. Boucle de chunks reliable, vérifie le SHA-256 à la fin
4. Confirme via `ResourcePackClientResponse(HAVE_ALL_PACKS)`
5. Reçoit `ResourcePackStack` (l'ordre d'application)
6. Confirme via `ResourcePackClientResponse(COMPLETED)`
7. Le serveur enchaîne sur le flow `PreSpawn`

---

## 2. Architecture côté serveur

### 2.1 Chargement des packs (`resource_pack.rs`)

Fichier : `crates/mc-rs-server/src/resource_pack.rs`

```rust
pub struct ResourcePack {
    pub manifest: ResourcePackManifest,
    pub data: Vec<u8>,        // ZIP raw, ce que le client télécharge
    pub sha256: [u8; 32],     // hash brut (PMMP hash_file ..., true)
}

pub fn load_pack(path: &Path) -> std::io::Result<ResourcePack>;
pub fn discover_packs(root: &Path) -> Vec<ResourcePack>;
```

**Comportement** :

- Si `path` est un dossier → zip récursif en mémoire (compression `Deflated`)
- Si `path` est un `.mcpack`/`.zip` → lit le fichier raw
- Le `manifest.json` est lu pour extraire `uuid`, `version`, `name`
- Le `SHA-256` est calculé sur les bytes du ZIP final (32 bytes raw, **pas hex**)

Helpers exposés :
- `pack.uuid() -> &str`
- `pack.version_string() -> String` (formate `[1,0,7]` en `"1.0.7"`)
- `pack.size() -> u64`
- `pack.chunk(index, chunk_size) -> &[u8]` (découpe à la demande)
- `pack.sha256_hex() -> String` (utilitaire, **pas** utilisé sur le wire)

**Découverte au boot** dans `main.rs` :

```rust
let resource_packs = Arc::new(crate::resource_pack::discover_packs(
    &crate::resource_pack::pack_path(),
));
info!("Loaded {} resource pack(s)", resource_packs.len());
```

L'`Arc<Vec<ResourcePack>>` est ensuite cloné dans chaque `Connection::new(...)` pour partager les bytes entre toutes les sessions sans copie.

### 2.2 Pipeline ResourcePack (`connection/login.rs`)

Fichier : `crates/mc-rs-server/src/connection/login.rs`

#### `send_resource_packs_info()`

Envoyé juste après `ClientToServerHandshake`. Format binaire conforme à PMMP 5.43.1 (`bedrock-protocol 57.1.0+bedrock-1.26.20`) :

```
write_bool(true);   // must_accept  ← force le pack à être prioritaire
write_bool(false);  // has_addons
write_bool(false);  // has_scripts
write_bool(false);  // force_disable_vibrant_visuals
write_i64_le(0);    // worldTemplateId UUID (nil)
write_i64_le(0);
write_string("");   // worldTemplateVersion
write_u16_le(N);    // resource_packs count
for pack in packs:
    write_uuid_pmmp(pack.uuid)   // 2 × i64_le, bytes reversés par moitié
    write_string(version)
    write_u64_le(size)
    write_string("")  // encryptionKey  (vide → pas de chiffrement)
    write_string("")  // subPackName
    write_string("")  // contentId
    write_bool(false) // hasScripts
    write_bool(false) // isAddonPack
    write_bool(false) // isRtxCapable
    write_string("")  // cdnUrl
```

⚠ Le `UUID` est encodé via le format PMMP `CommonTypes::putUUID` :

```rust
fn write_uuid_pmmp(writer: &mut ProtoWriter, uuid_str: &str) {
    let bytes = uuid::Uuid::parse_str(uuid_str)
        .map(|u| *u.as_bytes())
        .unwrap_or([0u8; 16]);
    let mut p1 = [0u8; 8]; let mut p2 = [0u8; 8];
    p1.copy_from_slice(&bytes[0..8]);
    p2.copy_from_slice(&bytes[8..16]);
    p1.reverse(); p2.reverse();
    writer.write_raw(&p1);
    writer.write_raw(&p2);
}
```

#### `handle_resource_pack_client_response(reader)`

Switch sur le `status` reçu :

| Status | Nom | Action serveur |
|---|---|---|
| 1 | `REFUSED` | Le client refuse → log + déco |
| 2 | `SEND_PACKS` | Pour chaque pack demandé, envoie `ResourcePackDataInfo` |
| 3 | `HAVE_ALL_PACKS` | Envoie `ResourcePackStack` |
| 4 | `COMPLETED` | Transition → `PreSpawn`, envoie les paquets de pré-spawn |

Le client envoie `packIds` au format `"<uuid>_<version>"` ; on matche uniquement le préfixe UUID :

```rust
let uuid_part = id.split('_').next().unwrap_or(&id);
```

#### `encode_resource_pack_data_info(pack)`

Envoyé quand le client demande le DL (status=2). Annonce métadonnées + nombre de chunks :

```rust
ResourcePackDataInfo {
    pack_id: pack.uuid().to_string(),
    max_chunk_size: 1 MB,
    chunk_count: ceil(size / 1MB),
    compressed_pack_size: pack.size(),
    sha256: pack.sha256,        // 32 bytes RAW (pas hex !)
    is_premium: false,
    pack_type: 0,               // 0 = Resources
}
```

⚠ Erreur classique évitée : le champ `sha256` est une string Bedrock de **32 bytes raw**, jamais hex 64 chars. PMMP utilise `hash_file("sha256", path, true)` (raw_output=true).

#### `handle_resource_pack_chunk_request(reader)`

Réponse à un `ResourcePackChunkRequest` :

```rust
ResourcePackChunkData {
    pack_id: pack.uuid().to_string(),
    chunk_index: req.chunk_index,
    offset: chunk_index * CHUNK_SIZE,
    data: pack.chunk(chunk_index, CHUNK_SIZE).to_vec(),
}
```

#### `send_resource_pack_stack()`

Envoyé à `HAVE_ALL_PACKS`. Définit l'ordre d'application :

```
write_bool(true);            // must_accept (force la priorité de la stack)
write_var_u32(N);            // pack count
for pack in packs:
    write_string(pack.uuid)
    write_string(version)
    write_string("")         // subPackName
write_string("1.26.20");     // baseGameVersion
write_u32_le(0);             // experiments count
write_bool(false);           // hasPreviouslyUsedExperiments
write_bool(false);           // useVanillaEditorPacks
```

⚠ Ici le `pack_id` est une **string** simple (pas le format UUID binaire de `ResourcePacksInfo`).

### 2.3 Forms / ModalForm (`connection/forms.rs`)

Fichier : `crates/mc-rs-server/src/connection/forms.rs`

Le module supporte aujourd'hui **un arbre de menus à 2 niveaux** : un menu racine
(`HubMenu`), un sous-menu listant les layouts (`UiShowcase`), et 8 démos
individuelles (`DemoGrid`, `DemoMotd`, etc.). Chaque entrée de l'arbre est
typée via `PendingFormKind` pour router la réponse au bon handler.

#### Énumération des formes

```rust
pub enum PendingFormKind {
    HubMenu,       // racine /menu
    UiShowcase,    // sous-menu "UI Showcase"
    DemoGrid, DemoLeftButton, DemoBottomButton,
    DemoImageGrid, DemoSquareImage,
    DemoMotd, DemoStore, DemoWrapped,
}
```

#### Émission d'un form (`ModalFormRequest`, S→C, paquet `0x64`)

Tous les builders passent par `encode_form(kind, json)` qui :
1. alloue un `form_id` croissant
2. mémorise la `PendingFormKind` dans `self.pending_form`
3. construit le `ModalFormRequest { form_id, form_data: json }`
4. retourne le batch compressé prêt à passer dans `prepare_for_send`

```rust
fn encode_form(&mut self, kind: PendingFormKind, json: &str) -> Vec<u8> {
    let form_id = self.allocate_form_id();
    self.pending_form = Some(PendingForm { form_id, kind });
    let req = ModalFormRequest { form_id, form_data: json.to_string() };
    self.encode_compressed_packet(packet_id::MODAL_FORM_REQUEST, &req.encode())
}

pub fn build_hub_form_batch(&mut self) -> Vec<u8> {
    let title = format!("{FLAG_GRID} §l§6mc-rs§r §eHUB");
    let json = format!(r#"{{"type":"form","title":"{title}",
        "content":"§7Choisis une action :",
        "buttons":[
            {{"text":"§a▶ Téléporter au spawn"}},
            {{"text":"§b▶ Mode Créatif"}},
            {{"text":"§e▶ Mode Survie"}},
            {{"text":"§d▶ Régler sur Jour"}},
            {{"text":"§9▶ Régler sur Nuit"}},
            {{"text":"§7▶ Infos biome"}},
            {{"text":"§6▶ §lUI Showcase"}}
        ]}}"#);
    self.encode_form(PendingFormKind::HubMenu, &json)
}
```

Le champ `title` contient un **préfixe `§m§<flag>`** qui active le layout
correspondant côté pack (voir §4). Les 8 flags sont exposés en constantes :

```rust
const FLAG_GRID:          &str = "§m§a";
const FLAG_LEFT_BUTTON:   &str = "§m§b";
const FLAG_BOTTOM_BUTTON: &str = "§m§c";
const FLAG_IMAGE_GRID:    &str = "§m§d";
const FLAG_SQUARE_IMAGE:  &str = "§m§e";
const FLAG_MOTD:          &str = "§m§f";
const FLAG_STORE:         &str = "§m§0";
const FLAG_WRAPPED:       &str = "§m§1";
```

#### API publique pour ouvrir un panel par son nom

```rust
pub const DEMO_PANEL_NAMES: &'static [&'static str] = &[
    "hub", "showcase", "grid", "left_button", "bottom_button",
    "image_grid", "square_image", "motd", "store", "wrapped",
];

pub fn build_demo_panel_batch(&mut self, panel: &str) -> Option<Vec<u8>> {
    Some(match panel {
        "hub" => self.build_hub_form_batch(),
        "showcase" => self.build_ui_showcase_batch(),
        "grid" => self.build_demo_grid_batch(),
        // …
        _ => return None,
    })
}
```

Cette API est utilisée par `/menu <panel>` (HardEnum côté commande) pour
ouvrir un layout sans passer par l'arbre de menus.

#### Réception (`ModalFormResponse`, C→S, paquet `0x65`)

```rust
pub(super) fn handle_modal_form_response(&mut self, reader: &mut ProtoReader)
    -> Vec<Vec<u8>>
{
    let resp = ModalFormResponse::decode(reader)?;
    let pending = self.pending_form.take()?;
    if pending.form_id != resp.form_id { return Vec::new(); }
    if resp.cancel_reason.is_some() { return Vec::new(); }
    let data = resp.response_data?;
    let index: usize = data.trim().parse().ok()?;

    match pending.kind {
        PendingFormKind::HubMenu     => self.handle_hub_menu(index),
        PendingFormKind::UiShowcase  => self.handle_ui_showcase(index),
        PendingFormKind::DemoBottomButton => self.handle_demo_bottom_button(index),
        // toutes les autres démos → "↩ Retour" = remonter au showcase
        _ => self.handle_demo_back_to_showcase(),
    }
}
```

**Trois patterns de routing différents** :

1. **Bouton → commande** (HubMenu) : on push une commande dans
   `pending_commands`, consommée par `main.rs` au tick suivant via
   `dispatch_command_line` (sans le `/` initial).
2. **Bouton → sous-menu** (HubMenu bouton 6, ou n'importe quelle démo) :
   on retourne directement le batch d'un autre form via `encode_form`.
   Le moteur RakNet l'envoie comme réponse à la `ModalFormResponse`.
3. **Combinaison** : un handler peut faire les deux (push commande +
   ouvrir un autre form).

⚠ Ne pas appeler `prepare_for_send` dans le handler : la main loop le fait
quand elle reçoit les `Vec<Vec<u8>>` de `handle_raw_batch`.

### 2.4 Commande `/menu` (`commands/menu.rs`)

Fichier : `crates/mc-rs-server/src/commands/menu.rs`

La commande prend un argument optionnel `panel` (HardEnum) qui permet d'ouvrir
directement n'importe quel layout. L'autocomplete client liste les 10 valeurs
exposées par `Connection::DEMO_PANEL_NAMES`.

```rust
pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut menu = CommandDefinition::new("menu", "Open the hub menu or a specific UI panel");
    menu.usage = "/menu [panel]".into();
    menu.permissions = vec!["server.command.menu".into()];
    menu.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "panel", "menu_panel",
            Connection::DEMO_PANEL_NAMES, // ["hub","showcase","grid",...]
            true,                          // optionnel
        )],
    });
    register_command(
        permissions, map, menu, PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            match invocation.arg(0) {
                None => { runtime.open_sender_menu(); Ok(()) }
                Some(panel) => runtime.open_sender_panel(panel)
                    .map_err(CommandDispatchError::Message),
            }
        },
    );
}
```

Le trait `ServerCommandRuntime` expose deux entrées :

```rust
fn open_sender_menu(&mut self);
fn open_sender_panel(&mut self, panel: &str) -> Result<(), String>;
```

Implémentation côté `commands/mod.rs` :

```rust
fn open_sender_menu(&mut self) {
    let addr = self.source_addr().expect("must be a player");
    let connection = self.connections.get_mut(&addr).unwrap();
    let batch = connection.build_hub_form_batch();
    let prepared = connection.prepare_for_send(batch);
    self.raknet.send_to_session(&addr, prepared, Reliability::ReliableOrdered, true);
}

fn open_sender_panel(&mut self, panel: &str) -> Result<(), String> {
    let addr = self.source_addr().ok_or("Console cannot open menu")?;
    let connection = self.connections.get_mut(&addr).ok_or("conn missing")?;
    let batch = connection.build_demo_panel_batch(panel)
        .ok_or_else(|| format!("Unknown panel: {panel}"))?;
    let prepared = connection.prepare_for_send(batch);
    self.raknet.send_to_session(&addr, prepared, Reliability::ReliableOrdered, true);
    Ok(())
}
```

#### Arbre de navigation final

```
/menu                  /menu <panel>
  │                          │
  ▼                          ▼
HubMenu                build_demo_panel_batch(panel)
  ├─ 0..5 commandes vanilla    │
  └─ 6 → UiShowcase          (ouvre directement)
            ├─ 0 grid
            ├─ 1 left_button
            ├─ 2 bottom_button
            ├─ 3 image_grid
            ├─ 4 square_image
            ├─ 5 motd
            ├─ 6 store
            ├─ 7 wrapped
            └─ 8 ↩ retour HubMenu
```

### 2.5 Champ Connection lié au pipeline

Fichier : `crates/mc-rs-server/src/connection/mod.rs`

```rust
pub struct Connection {
    // ...
    pub(super) resource_packs: Arc<Vec<crate::resource_pack::ResourcePack>>,
    pub(super) next_form_id: u32,
    pub(super) pending_form: Option<forms::PendingForm>,
    // ...
}
```

L'`Arc<Vec<ResourcePack>>` est passé en argument à `Connection::new()` depuis `main.rs` au moment où un nouveau peer RakNet est accepté.

---

## 3. Structure du pack côté disque

Tout pack est dans `resource_packs/<id>/`. Le nom du dossier n'est pas significatif côté wire (le client identifie par UUID). Notre pack actuel :

```
resource_packs/mcrs_ui/
├── manifest.json                              # métadonnées : UUID, version, min_engine_version
├── pack_icon.png                              # icône affichée dans Settings > My Packs
├── font/
│   └── glyph_E1.png                           # glyphes custom (range U+E100..U+E1FF)
├── textures/
│   ├── gui/newgui/mob_effects/                # overrides icônes effets vanilla
│   ├── items/                                 # overrides icônes items vanilla (friends, locker)
│   └── ui/
│       ├── title.png                          # logo mc-rs (écran titre)
│       ├── village_hero_effect.png            # override effet vanilla
│       └── mcrs/                              # tous nos assets UI
│           ├── button_default.png / _hover / _pressed
│           ├── panel_bg.png / _solid
│           ├── strip_gold.png / strip_orange.png
│           └── panels/                        # textures spécifiques aux layouts custom
│               ├── mcrs_rounded_corners.png   # 9-slice 12×12, base_size 24×24
│               ├── mcrs_inside_corners.png
│               ├── mcrs_rounded_border.png
│               ├── mcrs_normal_button*.png    # variantes default/hover/uv
│               ├── mcrs_special_button*.png   # idem, version "spéciale" (violet)
│               ├── mcrs_green_button.png
│               ├── mcrs_scoreboard_entry.png
│               ├── featured_bg_slice.png
│               ├── loading_grid.png           # placeholder pendant chargement d'image
│               ├── shop_close_button_default.png
│               ├── whitetransparency.png      # base pour overlays semi-opaques
│               └── whitetransparencygradient.png
└── ui/
    ├── _ui_defs.json                          # liste les fichiers UI custom à charger
    ├── _global_variables.json                 # override les variables UI globales ($vars)
    ├── server_form.json                       # dispatcher Form (le plus important)
    ├── chest_screen.json                      # override vanilla (large chest marker @@@@)
    ├── hud_screen.json                        # override vanilla (notifications + titres custom)
    ├── scoreboards.json                       # override vanilla (sidebar stylée mc-rs)
    ├── ui_common.json                         # override vanilla (scrollbar)
    └── mcrs/
        └── server_form/                       # 8 layouts custom (1 par flag)
            ├── button_grid_panel.json         # layout `grid` — grille de boutons rect.
            ├── button_image_grid_panel.json   # layout `image_grid` — grille d'images
            ├── left_button_panel.json         # layout `left_button` — boutons à gauche, description à droite
            ├── bottom_button_panel.json       # layout `bottom_button` — bannière en haut, boutons en bas
            ├── square_image_panel.json        # layout `square_image` — image centrale carrée
            ├── motd_panel.json                # layout `motd` — bannière + texte + 2 boutons
            ├── store_panel.json               # layout `store` — onglets de catégories + grille produits
            └── wrapped_panel.json             # layout `wrapped` — scroll d'images + URL
```

Voir [17 — UI Layouts](17-UI-LAYOUTS.md) pour la description détaillée de
chaque layout, le flag qui le déclenche, et le format JSON attendu côté
`ModalFormRequest`.

### 3.1 `manifest.json`

```json
{
  "format_version": 2,
  "header": {
    "name": "mc-rs UI",
    "description": "Custom UI pack served by mc-rs server.",
    "uuid": "d2b5a26f-85d7-45b6-ab1b-27be83c89779",
    "version": [1, 0, 0],
    "min_engine_version": [1, 21, 0]
  },
  "modules": [
    {
      "type": "resources",
      "uuid": "91ab2dba-d4c8-4ade-a2ad-c43c44c2d28f",
      "version": [1, 0, 0]
    }
  ]
}
```

Règles :
- **`uuid` (header)** doit être différent du **`uuid` (modules[0])**
- Bumper **`version`** force le client à re-DL (sinon il sert son cache)
- Changer l'`uuid` du header = pack vu comme totalement neuf par le client (utile en debug pour casser tout cache résiduel)
- `min_engine_version` doit être ≤ à la version client. `[1, 21, 0]` couvre Bedrock 1.21+

### 3.2 `ui/_ui_defs.json`

Déclare au moteur Bedrock **quels fichiers UI custom existent** dans le pack (les fichiers qui ne portent pas un nom de screen vanilla). Sans cette déclaration, les fichiers dans `ui/mcrs/server_form/*.json` sont ignorés silencieusement.

```json
{
  "ui_defs": [
    "ui/mcrs/server_form/button_grid_panel.json"
  ]
}
```

Les fichiers à nom standard (`server_form.json`, `hud_screen.json`, etc.) sont **auto-chargés**, pas besoin de les lister ici.

### 3.3 `ui/_global_variables.json`

Override les variables globales du système UI Bedrock. Format `{ "$var_name": valeur }`. Affecte tout le client tant que le pack est actif.

Notre pack définit :
- Les **flags de dispatcher** (`$flag_grid`, `$flag_left_button`, `$flag_bottom_button`) — voir §4
- Une **palette de couleurs** (`$mcrs_panel_bg_color`, `$mcrs_button_default_color`, etc.) utilisée par les layouts

Exemple :

```json
{
    "$flag_grid":  "§m§a",
    "$mcrs_panel_bg_color":      [0.05, 0.05, 0.10],
    "$mcrs_title_text_color":    [1.0, 0.85, 0.20],
    "$mcrs_button_default_color":[0.12, 0.12, 0.18]
}
```

D'autres variables existent dans le système Bedrock (`$7_color_format` pour redéfinir la couleur du code `§7`, `$transition_time_push` pour le temps d'animation, etc.) qu'on peut surcharger ici.

### 3.4 `ui/server_form.json` — le dispatcher

Le screen vanilla qui rend les `ModalFormRequest`. **Notre version override** ce screen pour rediriger vers nos propres panels selon un flag dans le titre.

Structure générale :

```json
{
  "namespace": "server_form",

  "third_party_server_screen@common.base_screen": {
    "$screen_content": "server_form.mcrs_main_screen_content",
    "button_mappings": [ ... menu_cancel → menu_exit ... ]
  },

  "mcrs_main_screen_content": {
    "type": "panel",
    "controls": [{
      "server_form_factory": {
        "type": "factory",
        "control_ids": {
          "long_form":   "@server_form.mcrs_switching_long_form",
          "custom_form": "@server_form.custom_form"
        }
      }
    }]
  },

  "mcrs_switching_long_form": {
    "type": "panel",
    "controls": [
      { "vanilla_fallback@long_form": { /* visible si AUCUN flag matche */ } },
      { "mcrs_grid_layout@mcrs_grid_modal.main_panel": { /* visible si #title_text contient $flag_grid */ } }
      // → ajouter d'autres entrées pour les autres flags
    ]
  }
}
```

#### Le binding qui matche le flag

Chaque variante a 2 bindings :

```json
{
  "binding_type": "global",
  "binding_name": "#title_text",
  "binding_name_override": "#title_text"
},
{
  "binding_type": "view",
  "source_property_name": "(not ((#title_text - $flag_grid) = #title_text))",
  "target_property_name": "#visible"
}
```

L'opérateur `-` sur des strings est une **soustraction de substring** en JSON UI Bedrock : `"abc-def" - "-"` donne `"abcdef"`. Si la soustraction change la string, le flag était présent → on rend la variante. Sinon, elle reste invisible.

Le fallback vanilla utilise la négation de **toutes** les sous-expressions :

```
(((#title_text - $flag_grid) = #title_text) and ((#title_text - $flag_left_button) = #title_text) and ...)
```

→ visible uniquement si aucun flag n'est trouvé.

### 3.5 `ui/mcrs/server_form/button_grid_panel.json` — le layout grid

Namespace `mcrs_grid_modal` avec un `main_panel` racine. Structure :

- **Backdrop** : image full-screen, color `$mcrs_panel_bg_color`, alpha 0.95
- **Top / bottom border** : strips 2px color `$mcrs_panel_border_color`
- **Title label** : binding `#title_text`, color `$mcrs_title_text_color`, font scale 1.4
- **Grid** : bindé sur la collection `form_buttons`, dimensions auto (2 colonnes, lignes calculées)
- **Buttons** : 3 layers (`default`, `hover`, `pressed`) avec couleurs différentes ; `$pressed_button_name = "button.form_button_click"` → c'est ce qui émet le `ModalFormResponse`

Bindings critiques pour itérer sur les boutons :

```json
"bindings": [
  {
    "binding_name": "#form_button_contents",
    "binding_name_override": "#collection_length"
  },
  {
    "binding_type": "view",
    "source_property_name": "(math.ceil((#collection_length + 1) / 2))",
    "target_property_name": "#grid_rows"
  }
]
```

→ dimensionne le grid au nombre exact de boutons reçus.

---

## 4. Mécanisme du title flag dispatcher

Concept : le serveur envoie un seul type de paquet (`ModalFormRequest` standard avec `type=form`), mais **encode dans le titre un code court** qui indique au pack quel layout afficher.

### 4.1 Codes choisis

Les codes sont des séquences `§m§<X>` :
- `§m` = format Minecraft "magic" (caractères animés/aléatoires) — invisible si le caractère qui suit n'est pas affichable
- `§<X>` = un code couleur arbitraire derrière

Visuellement le préfixe disparaît (ou est confondu avec du bruit) ; mais dans le **texte brut** stocké côté client, la sous-chaîne `§m§a` reste présente et matchable par le binding `(#title_text - "§m§a") != #title_text`.

Notre `_global_variables.json` définit **8 flags** correspondant aux 8 layouts
custom :

| Flag | Code | Layout cible | Namespace UI |
|---|---|---|---|
| `$flag_grid` | `§m§a` | Grille verticale de boutons rectangulaires | `mcrs_grid_modal` |
| `$flag_left_button_panel` | `§m§b` | Boutons à gauche, description à droite | `mcrs_left_button_modal` |
| `$flag_bottom_button_panel` | `§m§c` | Bannière en haut, liste de boutons en bas | `mcrs_bottom_button_modal` |
| `$flag_image_grid` | `§m§d` | Grille d'images avec titre superposé | `mcrs_image_grid_modal` |
| `$flag_square_image` | `§m§e` | Image carrée centrale + description | `mcrs_square_image_modal` |
| `$flag_motd` | `§m§f` | Bannière + texte + boutons (style MOTD) | `mcrs_motd_modal` |
| `$flag_store` | `§m§0` | Onglets de catégories + grille de produits | `mcrs_store_modal` |
| `$flag_wrapped` | `§m§1` | Scroll d'images + URL + boutons (style rewind) | `mcrs_wrapped_modal` |

Voir [17 — UI Layouts](17-UI-LAYOUTS.md) pour la description détaillée
de chaque layout et le format attendu côté `buttons[]`.

### 4.2 Côté serveur — émission

```rust
let title = format!("§m§a {}", "Mon Hub");      // active le layout grid
let title = format!("§m§f {}", "Bienvenue !");  // active le layout motd
```

### 4.3 Côté client — dispatch

Le `_global_variables.json` définit les 8 flags. `server_form.json` les
utilise dans les bindings pour décider quel layout rendre. Structure complète
du dispatcher (8 variantes + fallback vanilla) :

```json
"mcrs_switching_long_form": {
  "type": "panel",
  "$flag_grid": "§m§a", "$flag_left_button_panel": "§m§b",
  "$flag_bottom_button_panel": "§m§c", "$flag_image_grid": "§m§d",
  "$flag_square_image": "§m§e", "$flag_motd": "§m§f",
  "$flag_store": "§m§0", "$flag_wrapped": "§m§1",
  "controls": [
    { "long_form@long_form": { /* fallback : aucun flag présent */ } },
    { "mcrs_button_grid@mcrs_grid_modal.main_panel": { /* visible si #title_text contient $flag_grid */ } },
    { "mcrs_left_button_panel@mcrs_left_button_modal.main_panel": { /* … $flag_left_button_panel */ } },
    { "mcrs_bottom_button_panel@mcrs_bottom_button_modal.main_panel": { /* … */ } },
    { "mcrs_button_image_grid@mcrs_image_grid_modal.main_panel": { /* … */ } },
    { "mcrs_square_image@mcrs_square_image_modal.main_panel": { /* … */ } },
    { "mcrs_motd_panel@mcrs_motd_modal.main_panel": { /* … */ } },
    { "mcrs_store_panel@mcrs_store_modal.main_panel": { /* … */ } },
    { "mcrs_wrapped_panel@mcrs_wrapped_modal.main_panel": { /* … */ } }
  ]
}
```

### 4.4 Cas du fallback

Si aucun flag n'est trouvé dans le titre, le binding du `vanilla_fallback` est `true` et c'est le `long_form` standard (boutons gris vanilla) qui est rendu. Ça garantit que les forms d'autres serveurs (ou les forms vanilla) continuent à fonctionner.

---

## 5. Flow complet d'une session

```
Client                                                Serveur
  │                                                     │
  ├─ RequestNetworkSettings ──────────────────────────► │
  │                                       ◄──────────── │ NetworkSettings + state=Login
  │                                                     │
  ├─ Login (chain JWT + clientData) ──────────────────► │
  │                                       ◄──────────── │ ServerToClientHandshake (jwt)
  │                                                     │ ↑ envoyé non-chiffré, chiffrement active après
  ├─ ClientToServerHandshake ─────────────────────────► │
  │                                       ◄──────────── │ PlayStatus(LoginSuccess)
  │                                       ◄──────────── │ ResourcePacksInfo (must_accept=true,
  │                                                     │                    UUID, size, sha…)
  │                                                     │
  ├─ ResourcePackClientResponse(SEND_PACKS, [uuid_ver])►│
  │                                       ◄──────────── │ ResourcePackDataInfo (chunk_count, sha)
  │                                                     │
  ├─ ResourcePackChunkRequest (i=0) ─────────────────► │
  │                                       ◄──────────── │ ResourcePackChunkData (i=0, bytes…)
  │ … (répété chunk_count fois) …                      │
  │                                                     │
  ├─ ResourcePackClientResponse(HAVE_ALL_PACKS) ──────► │
  │                                       ◄──────────── │ ResourcePackStack (ordre d'application)
  │                                                     │
  ├─ ResourcePackClientResponse(COMPLETED) ───────────► │
  │                                                     │ state=PreSpawn → send_pre_spawn_packets()
  │                                       ◄──────────── │ StartGame, ItemRegistry, Biomes,
  │                                                     │ UpdateAttributes, AvailableCommands,
  │                                                     │ UpdateAbilities, …
  │                                                     │
  ├─ RequestChunkRadius ──────────────────────────────► │
  │                                       ◄──────────── │ ChunkRadiusUpdated + chunks + PlayStatus(PlayerSpawn)
  │                                                     │ state=SpawnResponse
  ├─ SetLocalPlayerAsInitialized ────────────────────► │ state=InGame
  │                                                     │
  │ ─── le joueur joue ───                              │
  │                                                     │
  ├─ Text "/menu"  (CommandRequest)──────────────────► │ dispatch_command_line → open_sender_menu()
  │                                       ◄──────────── │ ModalFormRequest (form_id=N, title="§m§a …")
  │                                                     │
  │ ─── client affiche le form via server_form.json ─── │
  │                                                     │
  ├─ ModalFormResponse (form_id=N, "2") ─────────────► │ handle_modal_form_response()
  │                                                     │ pending_form vérifié → push pending_commands
  │                                                     │ main loop tick → dispatch "gamemode survival"
```

---

## 6. Outils annexes du repo

### 6.1 `dump_pack` — sérialise un dossier en `.zip`

Fichier : `crates/mc-rs-server/src/bin/dump_pack.rs`

```bash
./target/release/dump_pack.exe <src_dir> [<dst_zip>]
./target/release/dump_pack.exe resource_packs/mcrs_ui mcrs_ui_dump.zip
```

Utile pour :
- Inspecter ce que le serveur enverra (utiliser `unzip -l` sur le résultat)
- Générer un `.mcpack` à donner manuellement au client

Le code est volontairement indépendant du module `mc_rs_server::resource_pack` pour pouvoir tester n'importe quel dossier.

### 6.2 `decrypt_pack` — déchiffre un pack marketplace AES-256-CFB8

Fichier : `crates/mc-rs-server/src/bin/decrypt_pack.rs`

```bash
./target/release/decrypt_pack.exe <pack.zip> <content_key_32chars> <out_dir>
```

Workflow :
1. Lit le ZIP en mémoire
2. Lit `contents.json` brut (256 bytes header magique + payload chiffré)
3. Décrypte le payload avec `Aes256` + `cfb8::Decryptor`, `key = ContentKey ASCII bytes`, `IV = ContentKey[..16]`
4. Parse le JSON → liste `{path, key}`
5. Pour chaque entrée listée, décrypte le fichier avec sa propre key
6. Les fichiers non listés (`manifest.json`, `pack_icon.png`) sont copiés tels quels

Dépendances workspace :
- `aes = "0.8"`
- `cfb8 = "0.8"`
- `zip = "0.6"`

Cet outil sert uniquement à inspecter des packs dont on a déjà la `ContentKey` (par exemple obtenus via le pipeline `ResourcePacksInfo` quand on les sert soi-même). À utiliser uniquement à des fins de compréhension / debug, sans redistribuer de contenu protégé.

---

## 7. Configuration `must_accept`

Deux endroits dans `connection/login.rs` :

| Position | Effet `false` | Effet `true` |
|---|---|---|
| `send_resource_packs_info()` | Le client peut refuser le pack et rentrer quand même | Le client doit accepter ou être déconnecté |
| `send_resource_pack_stack()` | Le pack est dans la stack mais peut être overridé par des packs locaux | Le pack est en priorité absolue → ses overrides UI s'appliquent vraiment |

**Recommandation** : mettre `true` aux deux quand le pack apporte des overrides UI structurels (`server_form.json`, `hud_screen.json`, etc.). Sinon Bedrock peut télécharger le pack **mais ne pas appliquer ses overrides UI** s'il considère que c'est optionnel.

---

## 8. Récap des fichiers à connaître

### Code Rust serveur

| Fichier | Rôle |
|---|---|
| `crates/mc-rs-server/src/resource_pack.rs` | Chargement / zip / SHA / chunk |
| `crates/mc-rs-server/src/connection/mod.rs` | Champ `resource_packs: Arc<...>`, `pending_form` |
| `crates/mc-rs-server/src/connection/login.rs` | Pipeline complet RP (info, stack, data_info, chunk) |
| `crates/mc-rs-server/src/connection/forms.rs` | `build_hub_form_batch()`, `handle_modal_form_response()` |
| `crates/mc-rs-server/src/commands/menu.rs` | Commande `/menu` |
| `crates/mc-rs-server/src/commands/mod.rs` | Trait `ServerCommandRuntime::open_sender_menu()` |
| `crates/mc-rs-server/src/bin/dump_pack.rs` | Outil zip d'un dossier |
| `crates/mc-rs-server/src/bin/decrypt_pack.rs` | Outil décryptage AES-256-CFB8 |

### Code Rust proto

| Fichier | Rôle |
|---|---|
| `crates/mc-rs-proto/src/packets/mod.rs` | Constantes IDs : `RESOURCE_PACKS_INFO=0x06`, `RESOURCE_PACK_STACK=0x07`, `MODAL_FORM_REQUEST=0x64`, etc. |
| `crates/mc-rs-proto/src/packets/world.rs` | `ResourcePackDataInfo`, `ResourcePackChunkData`, `ResourcePackChunkRequest` |
| `crates/mc-rs-proto/src/packets/forms.rs` | `ModalFormRequest`, `ModalFormResponse` |

### Pack côté disque

| Fichier | Rôle |
|---|---|
| `resource_packs/mcrs_ui/manifest.json` | Métadonnées pack (UUID, version) |
| `resource_packs/mcrs_ui/pack_icon.png` | Icône affichée dans Settings client |
| `resource_packs/mcrs_ui/ui/_ui_defs.json` | Déclare les JSON UI custom du pack |
| `resource_packs/mcrs_ui/ui/_global_variables.json` | Override des `$variables` UI globales (flags + palette mc-rs) |
| `resource_packs/mcrs_ui/ui/server_form.json` | Dispatcher (route le form vers un layout selon le flag du titre) |
| `resource_packs/mcrs_ui/ui/mcrs/server_form/button_grid_panel.json` | Layout `grid` (le seul implémenté à ce jour) |
| `resource_packs/mcrs_ui/textures/...` | Textures GUI custom |

---

## 9. Pièges classiques rencontrés (et résolus)

| Symptôme | Cause | Solution |
|---|---|---|
| Le client se déconnecte juste après `Resource packs completed` sans erreur | `sha256` envoyé en hex string (64 chars) au lieu de raw bytes (32 bytes) dans `ResourcePackDataInfo` | Utiliser `pack.sha256` directement, type `[u8; 32]` |
| Le client reste bloqué en "Chargement du serveur" indéfiniment | **Double encryption** des chunks via le fast-path `cached_zlib_batch` : `prepare_for_send` appelé 2× | Retirer le `prepare_for_send` dans `chunks.rs::send_chunk_batch`, c'est la main loop qui le fait |
| Le pack apparaît dans Settings client mais n'a aucun effet visuel | `must_accept = false` → Bedrock applique les `$global_variables` mais ignore les overrides UI structurels | Passer `must_accept = true` aux 2 endroits |
| Les fichiers personnalisés du pack JSON UI ne sont pas chargés | Fichiers absents de `_ui_defs.json` | Lister tous les `ui/<custom_path>.json` dans `_ui_defs.json` |
| L'override ne se prend pas même après bump version | Le client cache le pack par UUID stable | Changer l'UUID du header dans `manifest.json` pour forcer fresh install |

---

## 10. Ce qui marche aujourd'hui

- ✅ Pipeline RP complet : DL, chunks, validation SHA
- ✅ `/menu` + `/menu <panel>` avec autocomplete sur 10 valeurs
- ✅ Arbre `HubMenu → UiShowcase → 8 démos` avec navigation "↩ Retour"
- ✅ 8 layouts custom rendus côté client via title-flag dispatcher
- ✅ 4 overrides vanilla (chest, hud, scoreboard, ui_common)
- ✅ Pack chargé et appliqué côté client (codes `§` rendus, icône custom visible dans Settings)
- ✅ `must_accept = true` force la priorité du pack
- ✅ Outils `dump_pack` + `decrypt_pack` opérationnels

## 11. Pour aller plus loin

- Servir plusieurs packs simultanément (la struct supporte déjà `Vec<ResourcePack>`)
- Ajouter un cache disque des chunks pré-compressés pour les très gros packs
- Implémenter le chiffrement côté serveur (`encryptionKey` dans `ResourcePackInfoEntry`) si jamais on veut protéger un pack distribué
- Étendre la commande `/menu` pour générer dynamiquement les boutons selon les permissions du joueur
- Sortir le pack de `resource_packs/mcrs_ui/` vers un dépôt séparé une fois mature (versionning indépendant)
- Ajouter des layouts supplémentaires (modal de confirmation, formulaire de saisie, etc.) — voir le tutoriel "[Créer un nouveau layout](17-UI-LAYOUTS.md#10--tutoriel--créer-son-propre-layout)"
