# Bugs et écarts protocole dans `old_crates/`

Liste des différences entre `old_crates/` et `crates/` qui **cassent** le fonctionnement côté client Minecraft Bedrock 1.26.10 (protocole 924). Chaque point référence le fichier ancien fautif et l'équivalent corrigé côté nouveau.

---

## 1. `TextPacket` — ordre des champs inversé (CRITIQUE)

**Ancien** (`old_crates/mc-rs-proto/src/packets/text.rs` L101-122) encode :

```rust
buf.put_u8(self.text_type as u8);        // type en 1er
buf.put_u8(self.needs_translation as u8); // needsTranslation en 2ème
// puis message, xuid, platformChatId, filteredMessage
```

**Format correct 924** (PMMP `TextPacket.php`, implémenté dans `crates/mc-rs-proto/src/packets/player.rs` L800-810) :

```rust
w.write_bool(false); // needsTranslation EN PREMIER
w.write_u8(1);       // category (0=MESSAGE_ONLY, 1=AUTHORED_MESSAGE, 2=WITH_PARAMS)
w.write_u8(1);       // type
```

**Trois problèmes cumulés** :
1. `needsTranslation` doit venir **avant** le type
2. Il manque complètement le byte `category` (obligatoire en protocole 924)
3. Le décalage binaire fait que `text_type` est interprété comme `needsTranslation` côté client

**Effet** : chat entièrement cassé, messages système corrompus, déconnexion probable dès le premier Text packet reçu.

---

## 2. `CreativeContentPacket` — format incomplet

**Ancien** (`old_crates/mc-rs-proto/src/packets/creative_content.rs` L25-34) :

```rust
fn proto_encode(&self, buf: &mut impl BufMut) {
    VarUInt32(0).proto_encode(buf);                    // 0 groupes (hardcodé)
    VarUInt32(self.items.len() as u32).proto_encode(buf);
    for item in &self.items {
        VarUInt32(item.network_id).proto_encode(buf);
        item.item.proto_encode(buf);
        // MANQUE : VarUInt32(group_id)
    }
}
```

**Format correct** (PMMP `CreativeContentPacket::encodePayload`, voir `crates/mc-rs-proto/src/packets/world.rs` L695-720) :

```
groups_count (VarU32)
for each group:
    category_id (i32 LE) + category_name (string) + icon (ItemStackWithoutStackId)
items_count (VarU32)
for each item:
    entry_id (VarU32) + item (ItemStackWithoutStackId) + group_id (VarU32)  ← MANQUE DANS L'ANCIEN
```

**Problèmes** :
- Pas de groupes → menu créatif sans catégories
- Pas de `group_id` par item → décalage binaire → le client ignore le paquet ou déconnecte
- Aucune icône de groupe

---

## 3. Packet IDs erronés

Dans `old_crates/mc-rs-proto/src/packets/mod.rs` L163-243 :

| Packet | Ancien | Correct 924 | Conséquence |
|---|---|---|---|
| `MOVE_ACTOR_ABSOLUTE` | `0x10` | `0x12` | Les mouvements d'entités ne s'affichent jamais (0x10 = `REMOVE_ENTITY` côté client, donc l'entité disparaît au lieu de bouger) |
| `SET_ENTITY_MOTION` | `0x12` | `0x28` (`SET_ACTOR_MOTION`) | Knockback et vélocités cassés |
| `TAKE_ITEM_ENTITY` | `0x11` | `0x11` mais renommé `TAKE_ITEM_ACTOR` | Nom obsolète, sémantique OK |
| `ADD_ITEM_ENTITY` | `0x0F` | `0x0F` mais renommé `ADD_ITEM_ACTOR` | Nom obsolète, sémantique OK |

Le nouveau (`crates/mc-rs-proto/src/packets/mod.rs`) corrige ces deux IDs critiques.

---

## 4. `BiomeDefinitionList` — format custom non validé

**Ancien** (`old_crates/mc-rs-proto/src/packets/biome_definition_list.rs`) :

- Construit un binaire custom : u16 LE name_index + u16 LE id + 5 floats + ARGB + bool + Optional tags + Optional chunkGenData + string table
- Dépend d'un `data/biome_definitions.json` externe chargé via `serde_json`
- Format non cross-vérifié avec un client 1.26.10 réel

**Nouveau** (`crates/mc-rs-proto/src/packets/world.rs` L391-401) :

```rust
pub struct BiomeDefinitionList {
    pub nbt_data: Vec<u8>,
}
```

Le payload canonique est pré-construit depuis `crates/mc-rs-server/src/biomes_vanilla.rs` + `biomes_registry.rs` selon le format exact de PMMP, validé en connexion 1.26.10.

---

## 5. `UpdateAbilities` — `INFINITE_RESOURCES` non inversé (lié au bug fly survie)

**Ancien** (`old_crates/mc-rs-proto/src/packets/update_abilities.rs` L101-106) :

```rust
set_bool(
    &mut set_abilities,
    &mut set_values,
    ABILITY_INFINITE_RESOURCES,
    is_creative_or_spectator,  // bit mis à true en créatif
);
```

**Le bit 11 `INFINITE_RESOURCES` a une logique inversée** dans PMMP : il signifie "le joueur subit gravité, faim et dégâts". Donc :

- En survie, le bit doit être à **1** (la gravité s'applique)
- En créatif, le bit doit être à **0** (pas de gravité/faim/dégâts)

L'ancien fait **l'inverse** → en créatif le client peut appliquer gravité, en survie le client peut croire qu'il vole.

**Nouveau** (`crates/mc-rs-proto/src/packets/player.rs` L1280) documente explicitement :

```rust
pub const INFINITE_RESOURCES: u32 = 1 << 11; // note: inverted logic
```

C'est vraisemblablement la source du `BUG_FLY_MODE.md` cité dans `CLAUDE.md`.

---

## 6. `StartGame` — `blockNetworkIdsAreHashes`

**Ancien** (`old_crates/mc-rs-proto/src/packets/start_game.rs`) utilise des valeurs heuristiques pour `blockNetworkIdsAreHashes`. La note `CLAUDE.md` impose `false` en protocole 924 (les block IDs sont des indices séquentiels dans `canonical_block_states.nbt`). Si l'ancien l'envoie à `true`, le client attend des hashes FNV et tous les blocs s'affichent en "update!".

---

## 7. Creative items — runtime IDs faux

**Ancien** (`old_crates/mc-rs-proto/src/packets/creative_content.rs` L56-149) :

```rust
pub fn default_creative_items() -> Vec<(i32, u16)> {
    vec![
        (1, 1),   // stone
        (4, 1),   // cobblestone
        (56, 1),  // diamond_ore
        (271, 1), // wooden_pickaxe (uncertain ID)
        (287, 1), // string (approximate)
        ...
    ]
}
```

Ce sont des **IDs legacy Java** (Minecraft 1.12 numérique), pas des runtime IDs Bedrock. En 1.26 avec `blockNetworkIdsAreHashes=false`, les runtime IDs doivent être des **indices dans `canonical_block_states.nbt`**.

**Effet** : le menu créatif affichera des blocs totalement erronés ou vides.

Le nouveau (`crates/mc-rs-server/src/creative_content.rs`) utilise le `BlockRegistry` pour mapper nom → runtime ID canonique.

---

## 8. `mc-rs-proto::compression` — gestion de `EmptyBatch`

**Ancien** (`old_crates/mc-rs-proto/src/batch.rs` L40-43) :

```rust
if config.compression_enabled {
    if data.is_empty() {
        return Err(ProtoError::EmptyBatch);
    }
    ...
}
```

Retourne une erreur dure sur batch vide. Un client peut légitimement envoyer un batch vide en keep-alive → le serveur ancien tue la session.

**Nouveau** (`crates/mc-rs-proto/src/batch.rs` L51-62) : boucle sur `reader.remaining() > 0`, donc un batch vide retourne un `Vec::new()` sans erreur.

---

## 9. `mc-rs-nbt` ancien — pas de variant big-endian ni network

**Ancien** (`old_crates/mc-rs-nbt/src/`) :
```
error.rs  io.rs  le.rs  lib.rs  network.rs  tag.rs
```

`le.rs` fait 1 113 octets, `network.rs` 1 249 octets — modules quasi vides. Tout passe par `io.rs` monolithique.

**Nouveau** (`crates/mc-rs-nbt/src/`) :
```
be.rs (2 278 octets — Java BE)  le.rs (2 254 — disk LE)  network.rs (3 311 — VarInt)
```

Les 3 variantes sont complètement séparées avec leurs propres encoders/decoders. L'ancien mélange les trois, donc lire un `level.dat` (LE) ou un packet (Network VarInt) peut donner des résultats différents selon le chemin emprunté.

---

## 10. `connection/mod.rs` ancien — ordonnancement du login

L'ancien `old_crates/mc-rs-server/src/connection/mod.rs` (L25-92) envoie dans `spawn.rs` beaucoup de paquets **avant** `SetLocalPlayerAsInitialized` :
- `BiomeDefinitionList`, `AvailableEntityIdentifiers`, `ItemRegistry`, `StartGame`, `CreativeContent`, `PlayerList`, `UpdateAttributes`, `UpdateAbilities`, `SetActorData`…

La séquence PMMP correcte est :

```
SessionStart → Login → Handshake → ResourcePacks → PreSpawn → SpawnResponse → InGame
```

Le client 1.26.10 ignore/reporte les paquets envoyés trop tôt, d'où des freezes au spawn.

Le nouveau (`crates/mc-rs-server/src/connection/mod.rs`) implémente la state machine explicite via `ConnectionState` et n'envoie les paquets de jeu qu'en `PreSpawn`/`InGame`.

---

## 11. `SetActorData` — `PropertySyncData` mal positionné

Note `CLAUDE.md` : « SetActorData : PropertySyncData (2x VarUInt32) AVANT tick, pas après ».

Si l'ancien `old_crates/mc-rs-proto/src/packets/set_actor_data.rs` envoie le tick avant le PropertySyncData, le client lit le tick comme 2 VarUInt32 à 0 puis le reste est décalé. À vérifier dans l'ancien — flags `BREATHING`/`HAS_GRAVITY`/`HAS_COLLISION` ne s'appliquent pas si le format est faux.

---

## 12. Dépendances du workspace

L'ancien `old_crates/mc-rs-server/Cargo.toml` déclare :

```toml
mc-rs-world = { path = "../mc-rs-world" }
mc-rs-game = { path = "../mc-rs-game" }
mc-rs-plugin-api = { path = "../mc-rs-plugin-api" }
mc-rs-plugin-wasm = { path = "../mc-rs-plugin-wasm" }
mc-rs-plugin-lua = { path = "../mc-rs-plugin-lua" }
mc-rs-behavior-pack = { path = "../mc-rs-behavior-pack" }
```

Ces 6 crates ne sont **pas dans le `Cargo.toml` racine** (workspace) du projet actuel. Tout activation d'`old_crates` dans le workspace casse le build immédiatement.

---

## Résumé — ce qui empêche une connexion 1.26.10 réelle

Dans l'ordre de gravité :

1. **`TextPacket`** ordre needsTranslation/category/type → chat cassé
2. **`CreativeContent`** manque `group_id` par item → kick au spawn créatif
3. **`MOVE_ACTOR_ABSOLUTE 0x10` au lieu de `0x12`** → entités invisibles/disparaissent
4. **`SET_ENTITY_MOTION 0x12` au lieu de `0x28`** → knockback cassé
5. **`UpdateAbilities INFINITE_RESOURCES` non inversé** → fly/gravité incohérents
6. **Runtime IDs creative items** en legacy Java → menu créatif corrompu
7. **`BiomeDefinitionList`** format custom non validé
8. **Ordonnancement login** → freeze au spawn
9. **`EmptyBatch` dur** → kick sur keep-alive
10. **NBT mono-variant** → lecture level.dat / packets incohérente
