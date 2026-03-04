# FIX LOG (mc-rs)

Ce fichier sert de mémoire des correctifs déjà appliqués pour éviter de tourner en rond.
À chaque nouvelle demande de fix, vérifier d'abord ce log.

## 2026-03-04 - Erratum NBT: encodage brut (pas de préfixe VarUInt32)

### Contexte
- Le correctif du 2026-03-03 avait introduit `write_byte_array` (VarUInt32(length) + bytes) pour les blobs NBT dans StartGame, ItemRegistry et BlockActorData.
- En réalité, PocketMine PHP `ByteBufferWriter::writeByteArray($bytes)` écrit des **octets bruts** (sans préfixe de longueur). La preuve : `CommonTypes::putString()` écrit le préfixe de longueur séparément puis appelle `writeByteArray` pour les octets.

### Symptôme
- Client déconnecte immédiatement après le burst pre-spawn (avant RequestChunkRadius).
- Un octet parasite (VarUInt32(3) pour NBT vide) décalait tous les champs suivants dans StartGame et ItemRegistry, corrompant le parsing côté client.

### Correctif appliqué
Fichiers modifiés :
- `crates/mc-rs-proto/src/packets/start_game.rs`
- `crates/mc-rs-proto/src/packets/item_registry.rs`
- `crates/mc-rs-proto/src/packets/block_actor_data.rs`

Changements :
1. **StartGame** : `block_properties[*].nbt` et `property_data` → `buf.put_slice(...)` au lieu de `codec::write_byte_array(...)`.
2. **ItemRegistry** : `component_nbt` → `buf.put_slice(...)` au lieu de `codec::write_byte_array(...)`.
3. **BlockActorData** : encode `buf.put_slice(&self.nbt_data)` ; decode : lire le reste du buffer comme NBT brut (`buf.copy_to_bytes(buf.remaining())`).

### Vérifications
- `cargo test -p mc-rs-proto` -> OK (271 tests)
- `cargo build --release -p mc-rs-server` -> OK

### Règle anti-boucle
- Pour les champs NBT "compound" du protocole Bedrock (StartGame block palette / property_data, ItemRegistry component_nbt, BlockActorData nbt), utiliser **octets bruts** (`put_slice`), pas `write_byte_array` (qui ajoute un préfixe VarUInt32).
- Ne pas réintroduire de préfixe de longueur sur ces champs sans preuve binaire (dump PocketMine vs client).

---

## 2026-03-04 - Crash client après pre-spawn: `PlayerList(Add)` incomplet

### Symptôme
- Le client affiche `Une erreur s'est produite` puis se déconnecte.
- Logs serveur: déconnexion immédiate juste après envoi du burst pre-spawn, avant `RequestChunkRadius`.

### Cause confirmée
- Écart protocolaire sur `PlayerList` (`0x3F`, action Add):
  - champ `color` (ARGB `u32 LE`) manquant dans chaque entrée.
- Référence PocketMine: `PlayerListPacket::encodePayload()` écrit ce `color` avant les flags `isVerified`.

### Correctif appliqué
Fichiers modifiés :
- `crates/mc-rs-proto/src/packets/player_list.rs`
- `crates/mc-rs-server/src/connection/login.rs`
- `crates/mc-rs-server/src/connection/spawn.rs`
- `crates/mc-rs-server/src/connection/portal.rs`

Changements :
1. Ajout du champ `color_argb: u32` dans `PlayerListAdd`.
2. Encodage `buf.put_u32_le(entry.color_argb)` après `is_sub_client`.
3. Valeur par défaut utilisée côté serveur: `0xFFFF_FFFF` (blanc opaque, comme PMMP fallback).

### Extrait de code
```rust
buf.put_u8(entry.is_teacher as u8);
buf.put_u8(entry.is_host as u8);
buf.put_u8(entry.is_sub_client as u8);
buf.put_u32_le(entry.color_argb);
```

### Vérifications
- `cargo test -p mc-rs-proto player_list` -> OK
- `cargo check -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK

### Règle anti-boucle
- Ne jamais encoder `PlayerList(Add)` sans le champ `color` ARGB.
- Toute comparaison future avec PMMP doit inclure la queue de `PlayerList` (color + verified flags).

---

## 2026-03-03 - Correctif protocolaire `writeByteArray` (StartGame/ItemRegistry/BlockActorData)

### Symptôme
- Client bloqué sur "Création du monde / Recherche du serveur".
- Le serveur allait jusqu'à `InGame`, mais le client restait figé.

### Cause confirmée
- Écart d'encodage avec PocketMine/BedrockProtocol :
  - certains blobs NBT étaient envoyés en **raw bytes** au lieu de **byte-array** Bedrock (`VarUInt32(length) + bytes`).
- Impact direct dans le flux de login :
  - `StartGame.block_properties[*].nbt`
  - `StartGame.property_data`
  - `ItemRegistry.component_nbt`
  - `BlockActorData.nbt_data`

### Correctif appliqué
Fichiers modifiés :
- `crates/mc-rs-proto/src/codec.rs`
- `crates/mc-rs-proto/src/packets/start_game.rs`
- `crates/mc-rs-proto/src/packets/item_registry.rs`
- `crates/mc-rs-proto/src/packets/block_actor_data.rs`

Changements :
1. Ajout helpers protocole :
   - `codec::write_byte_array(...)`
   - `codec::read_byte_array(...)`
2. `StartGame` :
   - remplacement de `put_slice(&bp.nbt)` par `write_byte_array(...)`
   - remplacement de `put_slice(&property_data)` par `write_byte_array(...)`
3. `ItemRegistry` :
   - `component_nbt` désormais encodé en byte-array length-prefixé.
4. `BlockActorData` :
   - encode/decode alignés byte-array.
5. Tests ajoutés pour verrouiller la régression.

### Extrait de code
```rust
// StartGame
codec::write_string(buf, &bp.name);
codec::write_byte_array(buf, &bp.nbt);
...
codec::write_string(buf, &self.game_engine);
codec::write_byte_array(buf, &self.property_data);

// ItemRegistry
codec::write_byte_array(buf, &entry.component_nbt);

// Codec helper
pub fn write_byte_array(buf: &mut impl BufMut, bytes: &[u8]) {
    VarUInt32(bytes.len() as u32).proto_encode(buf);
    buf.put_slice(bytes);
}
```

### Vérifications
- `cargo test -p mc-rs-proto start_game` -> OK
- `cargo test -p mc-rs-proto item_registry` -> OK
- `cargo test -p mc-rs-proto block_actor_data` -> OK
- `cargo check -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK
- binaire: `target/release/mc-rs-server.exe` mis à jour (`03/03/2026 10:49:24`)

### Règle anti-boucle
- Toute donnée "NBT encodée réseau" doit passer par `write_byte_array` / `read_byte_array`.
- Ne plus réintroduire de `put_slice()` direct sur des champs où PocketMine utilise `writeByteArray()`.

---

## ERRATUM 2026-03-03
- L'hypothèse précédente "serializer chunk: `writeByteArray` pour les words de subchunk/biome" est **fausse** pour ce format-là.
- Dans `ChunkSerializer` PocketMine, ces `words` sont écrits en bytes bruts (pas de préfixe).
- Conclusion: ne pas re-appliquer ce faux correctif sans preuve binaire.

---

## 2026-03-03 - Limitation rayon initial de spawn (anti-stall chargement)

### Symptôme
- Client bloqué sur "Création du monde / Recherche du serveur".
- Côté serveur, la séquence de login allait jusqu'à `InGame`, mais sans progression client visible.

### Hypothèse traitée
- Burst initial trop lourd de `LevelChunk` au moment du spawn (289 chunks pour rayon 8), pouvant bloquer le client pendant l'initialisation.

### Correctif appliqué
Fichier modifié :
- `crates/mc-rs-server/src/connection/spawn.rs`

Changement :
1. Rayon accepté pendant `Spawning` réduit temporairement :
   - avant : `clamp(1, 8)`
   - après : `clamp(1, 2)`
2. En `InGame`, cap conservé à `8`.

### Extrait de code
```rust
let accepted_radius = if state == LoginState::Spawning {
    request.chunk_radius.clamp(1, 2)
} else {
    request.chunk_radius.clamp(1, 8)
};
```

### Vérifications
- `cargo check -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK
- `target/release/mc-rs-server.exe` mis à jour (`03/03/2026 09:48:25`)

### Règle anti-boucle
- Si ce fix ne débloque pas, ne pas ré-augmenter immédiatement le rayon initial.
- Garder ce mode "spawn léger" pour diagnostiquer, puis ajuster progressivement (2 -> 4 -> 8).

---

## 2026-03-03 - Alignement pre-spawn PocketMine (SetTime / SetDifficulty / SetSpawnPosition)

### Symptôme
- Le client restait bloqué sur "Création du monde".
- Le serveur envoyait les chunks + `PlayStatus(PlayerSpawn)`, recevait `ServerboundLoadingScreen(type=1)`, mais jamais `SetLocalPlayerAsInitialized`.

### Cause probable traitée
- Séquence pre-spawn incomplète par rapport à PocketMine `onEnterWorld()`.
- Packets manquants côté `mc-rs` avant phase chunks :
  - `SetTime` (`0x0A`)
  - `SetDifficulty` (`0x3C`)
  - `SetSpawnPosition` (`0x2B`, mode world spawn)

### Correctif appliqué
Fichiers modifiés :
- `crates/mc-rs-proto/src/packets/set_time.rs`
- `crates/mc-rs-proto/src/packets/set_difficulty.rs`
- `crates/mc-rs-proto/src/packets/set_spawn_position.rs`
- `crates/mc-rs-proto/src/packets/mod.rs`
- `crates/mc-rs-server/src/connection/mod.rs`
- `crates/mc-rs-server/src/connection/login.rs`

Changements :
1. `SetTime` corrigé au format **VarInt signed** (et pas `i32_le`).
2. Ajout packet `SetDifficulty` (VarUInt).
3. Ajout packet `SetSpawnPosition` (ordre PMMP: `spawnType`, `spawnPosition`, `dimension`, `causingBlockPosition`).
4. Envoi pre-spawn juste après `StartGame` :
   - `SetTime`
   - `SetDifficulty`
   - `SetSpawnPosition::world_spawn(...)`
5. Marqueur log ajouté :
   - `BUILD_MARKER spawn-r11-2026-03-03`

### Extrait de code ajouté
```rust
self.send_packet(addr, packets::id::SET_TIME, &SetTime {
    time: self.world_time as i32,
}).await;

self.send_packet(addr, packets::id::SET_DIFFICULTY, &SetDifficulty {
    difficulty: difficulty as u32,
}).await;

self.send_packet(
    addr,
    packets::id::SET_SPAWN_POSITION,
    &SetSpawnPosition::world_spawn(self.spawn_block, self.dimension_id),
).await;
```

### Vérifications réalisées
- `cargo test -p mc-rs-proto set_time` -> OK
- `cargo test -p mc-rs-proto set_difficulty` -> OK
- `cargo test -p mc-rs-proto set_spawn_position` -> OK
- `cargo test -p mc-rs-world serializer::` -> OK
- `cargo test -p mc-rs-proto start_game` -> OK
- `cargo build --release -p mc-rs-server` -> OK

### Règle anti-boucle
- Ne pas réintroduire `SetTime` en `i32_le`.
- Garder `SetTime/SetDifficulty/SetSpawnPosition` dans la séquence pre-spawn tant que le client 924 est ciblé.
- Pour `NetworkChunkPublisherUpdate`, conserver `saved_chunks.len()` encodé en **u32 LE** (parité PMMP actuelle).

---

## 2026-03-03 - Fallback spawn-ready sur `ServerboundLoadingScreen(type=1)`

### Symptôme
- Malgré les packets pre-spawn alignés (`SetTime`, `SetDifficulty`, `SetSpawnPosition`), certains clients restent en `Spawning`.
- Logs observés : `ServerboundLoadingScreen(type=1)` reçu, mais pas de `SetLocalPlayerAsInitialized`.

### Correctif appliqué
Fichiers modifiés :
- `crates/mc-rs-server/src/connection/spawn.rs`
- `crates/mc-rs-server/src/connection/login.rs`

Changements :
1. Extraction de la finalisation de spawn dans `finalize_spawn_ready(...)`.
2. `handle_set_local_player_as_initialized()` appelle désormais ce point unique.
3. Fallback ajouté :
   - si `ServerboundLoadingScreen(type=1)` arrive en état `Spawning`,
   - on déclenche `finalize_spawn_ready(...)` avec le runtime ID serveur.
4. Marqueurs logs ajoutés :
   - `BUILD_MARKER spawn-r12-2026-03-03: using ServerboundLoadingScreen(type=1) fallback`
   - `BUILD_MARKER spawn-r12-2026-03-03: finalize_spawn_ready ...`

### Extrait de code ajouté
```rust
if pkt.loading_screen_type == 1 {
    let runtime_id = self
        .connections
        .get(&addr)
        .and_then(|c| (c.state == LoginState::Spawning).then_some(c.entity_runtime_id));
    if let Some(runtime_id) = runtime_id {
        self.finalize_spawn_ready(
            addr,
            runtime_id,
            "ServerboundLoadingScreen(type=1)",
        ).await;
    }
}
```

### Vérification
- `cargo check -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK

### Règle anti-boucle
- Si `SetLocalPlayerAsInitialized` est absent mais `ServerboundLoadingScreen(type=1)` est présent, conserver ce fallback actif pour éviter le blocage infini.

---

## 2026-03-03 - Alignement post-ready client (SetSpawnPosition(player) + MovePlayer(reset))

### Contexte
- Le fallback `spawn-r12` faisait passer la session serveur en `InGame`, mais certains clients restaient visuellement bloqués sur l'écran de création/chargement.

### Correctif appliqué
Fichier modifié :
- `crates/mc-rs-server/src/connection/spawn.rs`

Changements :
1. Après `finalize_spawn_ready(...)`, envoi explicite au joueur local :
   - `SetSpawnPosition` type joueur (`spawn_type = 0`)
   - `MovePlayer::reset(...)` avec la position/rotation serveur courante
2. Marqueur ajouté :
   - `BUILD_MARKER spawn-r13-2026-03-03`

### Extrait de code
```rust
self.send_packet(
    addr,
    packets::id::SET_SPAWN_POSITION,
    &SetSpawnPosition {
        spawn_type: 0,
        spawn_position: player_spawn,
        dimension,
        causing_block_position: player_spawn,
    },
).await;

self.send_packet(
    addr,
    packets::id::MOVE_PLAYER,
    &MovePlayer::reset(runtime_id, position, pitch, yaw, head_yaw, on_ground, tick),
).await;
```

### Vérification
- `cargo check -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK

---

## 2026-03-02 - Blocage "Création du monde" (Bedrock client)

### Symptôme
- Le client reste bloqué sur l'écran "Création du monde / Recherche du serveur".
- Le serveur reçoit `ServerboundLoadingScreen type=1` mais jamais `SetLocalPlayerAsInitialized`.

### Cause principale identifiée
- Format de sérialisation `LevelChunk` incomplet/incompatible côté payload subchunks/biomes.
- Les arrays de mots (`words`) de stockage palette n'étaient pas encodés comme byte-array Bedrock (il manquait le préfixe longueur VarUInt).

### Correctif appliqué
Fichier modifié :
- `crates/mc-rs-world/src/serializer.rs`

Changements :
1. Subchunk block storage :
- Ajout du préfixe `VarUInt(length)` pour les mots de storage.
- Cas `bits_per_block = 0` : envoi d'un byte-array vide (`length=0`) avant la palette.

2. Biome storage :
- Ajout du préfixe `VarUInt(length)` pour les mots de storage biome.
- Cas mono-biome (`bits=0`) : envoi d'un byte-array vide (`length=0`) avant la palette.

3. Tests serializer mis à jour pour refléter ce format.

### Extrait de code ajouté
```rust
// Subchunk: bpb=0 -> words byte-array vide
buf.put_u8(0x01);
write_varuint32(buf, 0);

// Subchunk: bpb>0 -> words avec préfixe longueur
let mut words = BytesMut::with_capacity(word_count * 4);
for word_idx in 0..word_count {
    let mut word: u32 = 0;
    for slot in 0..blocks_per_word {
        let block_idx = word_idx * blocks_per_word + slot;
        if block_idx < 4096 {
            let palette_index = sub_chunk.blocks[block_idx] as u32;
            word |= palette_index << (bpb as u32 * slot as u32);
        }
    }
    words.put_u32_le(word);
}
write_varuint32(buf, words.len() as u32);
buf.extend_from_slice(&words);

// Biome: bpe=0 -> words byte-array vide
section_buf.put_u8(0x01);
write_varuint32(&mut section_buf, 0);

// Biome: bpe>0 -> words avec préfixe longueur
let mut words = BytesMut::with_capacity(word_count * 4);
for word_idx in 0..word_count {
    let mut word: u32 = 0;
    for slot in 0..entries_per_word {
        let entry_idx = word_idx * entries_per_word + slot;
        if entry_idx < 64 {
            let sx = entry_idx % 4;
            let sz = (entry_idx / 4) % 4;
            let biome_id = biome_4x4[sx * 4 + sz];
            let palette_idx = biome_to_palette[biome_id as usize] as u32;
            word |= palette_idx << (bpe as u32 * slot as u32);
        }
    }
    words.put_u32_le(word);
}
write_varuint32(&mut section_buf, words.len() as u32);
section_buf.extend_from_slice(&words);
```

### Vérification technique
- `cargo test -q -p mc-rs-world serializer::` -> OK
- `cargo check -q` -> OK

### Marqueurs logs utiles à surveiller après redémarrage
- `Sent spawn stream ... PlayStatus(PlayerSpawn)`
- `post-init chunk sync sent`
- `Player <name> is now in-game`

### Notes
- Avant ce correctif, des ajustements login/pre-spawn avaient aussi été faits (`StartGame.game_engine`, `CreativeContent`, `PlayerList local`) mais le blocage persistait.
- Ce fix `LevelChunk` est le correctif structurel principal à retester en priorité.

---

## 2026-03-02 - Correctif définitif: `writeByteArray` = raw bytes (pas length-prefixed)

### Problème de diagnostic précédent
- Une hypothèse incorrecte avait été appliquée: `writeByteArray()` aurait ajouté un préfixe de longueur.
- Cette hypothèse a conduit à ajouter des `VarUInt(length)` devant les `words` subchunk/biome et un faux `tiles length=0` après `border block array count`.

### Réalité PocketMine/pmmp
- `ByteBufferWriter::writeByteArray(string $value)` écrit des octets bruts.
- Donc, dans `ChunkSerializer`, les `words` sont écrits en brut (sans longueur supplémentaire).
- Quand il n'y a pas de tiles, le payload de chunk se termine au byte `border block array count = 0x00`.

### Fix appliqué
Fichier modifié:
- `crates/mc-rs-world/src/serializer.rs`

Actions:
1. Suppression des `VarUInt(length)` ajoutés devant les `words` subchunk.
2. Suppression des `VarUInt(length)` ajoutés devant les `words` biome.
3. Suppression du `write_varuint32(0)` après `border block array count`.
4. Ajout de tests de régression:
- `subchunk_words_are_not_length_prefixed`
- `biome_words_are_not_length_prefixed`
- `payload_ends_with_border_count_eof_when_no_tiles`

### Extrait code (version correcte)
```rust
// Subchunk bpb>0: words bruts, sans longueur
for word_idx in 0..word_count {
    let mut word: u32 = 0;
    for slot in 0..blocks_per_word {
        let block_idx = word_idx * blocks_per_word + slot;
        if block_idx < 4096 {
            let palette_index = sub_chunk.blocks[block_idx] as u32;
            word |= palette_index << (bpb as u32 * slot as u32);
        }
    }
    words.put_u32_le(word);
}
buf.extend_from_slice(&words);

// Fin de payload chunk sans tiles:
buf.put_u8(0x00); // border block array count
```

### Règle anti-boucle
- Si le blocage réapparaît, ne PAS réintroduire de préfixe de longueur devant les `words`.
- La prochaine investigation doit se faire par dump hex comparatif `StartGame` + 1er `LevelChunk` contre PocketMine.

---

## 2026-03-02 - Correctif hash Bedrock canonique (infiniburn_bit)

### Cause identifiée (comparaison PocketMine/BedrockData)
- Le runtime ID hash de `minecraft:bedrock` envoyé par `mc-rs` n'était pas canonique.
- `mc-rs` utilisait `hash_block_state("minecraft:bedrock")` (states vides) => `0x0FF59FE6`.
- Canon PocketMine (`canonical_block_states.nbt`) pour l'état par défaut:
  - `minecraft:bedrock` + `states { infiniburn_bit: 0 }` => `0xC411A083`.

### Pourquoi c'est critique
- En mode `StartGame.block_network_ids_are_hashes=true`, le client attend des hashes strictement identiques aux états bloc canoniques.
- Un hash divergent dans la palette chunk peut bloquer la fin d'initialisation client (pas de `SetLocalPlayerAsInitialized`).

### Fix appliqué
Fichiers modifiés:
- `crates/mc-rs-world/src/block_hash.rs`
- `crates/mc-rs-world/src/serializer.rs`
- `crates/mc-rs-world/src/block_registry.rs`
- `crates/mc-rs-world/src/block_state_registry.rs`

Changements:
1. Ajout d'un hash canonique bedrock:
   - `hash_default_bedrock()` = `hash_bedrock_state(false)` (`infiniburn_bit=0`)
2. Générateurs mis à jour pour utiliser le hash bedrock canonique.
3. Compat rétro:
   - `LEGACY_BEDROCK_HASH_EMPTY_STATES = 0x0FF59FE6`
   - normalisation réseau dans le serializer:
     - `normalize_legacy_bedrock_hash(runtime_id)` avant écriture palette chunk
4. Registry blocs:
   - mapping bedrock canonique + hash legacy vers les mêmes propriétés.
5. Registry états:
   - ajout explicite des états bedrock `infiniburn_bit=0/1`.

### Extrait code ajouté
```rust
pub const LEGACY_BEDROCK_HASH_EMPTY_STATES: u32 = 0x0FF5_9FE6;

pub fn hash_bedrock_state(infiniburn: bool) -> u32 {
    hash_block_state_with_props(
        "minecraft:bedrock",
        &[("infiniburn_bit", StateValue::Byte(if infiniburn { 1 } else { 0 }))],
    )
}

pub fn hash_default_bedrock() -> u32 {
    hash_bedrock_state(false)
}

pub fn normalize_legacy_bedrock_hash(runtime_id: u32) -> u32 {
    if runtime_id == LEGACY_BEDROCK_HASH_EMPTY_STATES {
        hash_default_bedrock()
    } else {
        runtime_id
    }
}
```

### Règle anti-boucle
- Ne pas revenir à `hash_block_state("minecraft:bedrock")` pour le runtime network.
- Garder la normalisation legacy tant que d'anciens chunks existent.

---

## 2026-03-02 - Correctif `NetworkChunkPublisherUpdate` (compte de tableau encodé en VarUInt)

### Cause identifiée
- Le packet `NetworkChunkPublisherUpdate` encodait `saved_chunks.len()` en `u32_le`.
- PocketMine/BedrockProtocol encode ce tableau via `PacketSerializer::putArray()`, donc **longueur en VarUInt**.
- Avec `saved_chunks=[]`, le serveur envoyait `00 00 00 00` au lieu de `00`.

### Impact
- Divergence wire-format sur un packet clé du spawn (`0x79`), envoyé juste avant les chunks.
- Risque de parser client désaligné / packet rejeté pendant la phase “Création du monde”.

### Fix appliqué
Fichier modifié :
- `crates/mc-rs-proto/src/packets/network_chunk_publisher_update.rs`

Changement :
1. Remplacement de `put_u32_le(len)` par `VarUInt32(len).proto_encode(...)`.
2. Test ajusté pour valider `saved_chunks=[] => 0x00` (VarUInt32).

### Extrait code
```rust
self.position.proto_encode(buf);
VarUInt32(self.radius).proto_encode(buf);
// BedrockProtocol PacketSerializer::putArray() uses VarUInt32 for array length.
VarUInt32(self.saved_chunks.len() as u32).proto_encode(buf);
for chunk in &self.saved_chunks {
    chunk.proto_encode(buf);
}
```

### Vérification
- `cargo test -p mc-rs-proto network_chunk_publisher_update` -> OK
- `cargo test -p mc-rs-world serializer::` -> OK
- `cargo test -p mc-rs-proto start_game` -> OK
- `cargo build --release -p mc-rs-server` -> OK

---

## 2026-03-02 - Instrumentation spawn (trace RX/TX + hex preview)

### Objectif
- Arrêter le debug “à l’aveugle” sur le blocage “Création du monde”.
- Obtenir des logs exploitables pour chaque paquet critique du pré-spawn/spawn.

### Ajouts
Fichiers modifiés :
- `crates/mc-rs-server/src/connection/mod.rs`
- `crates/mc-rs-server/src/connection/login.rs`
- `crates/mc-rs-server/src/connection/spawn.rs`

Changements principaux :
1. Trace sortante `send_packet()`:
- état session, packet id + nom, tailles (`sub_len`, `body_len`), chiffrement, preview hex.
2. Trace entrante `handle_packet()`:
- état session, packet id + nom, tailles, preview hex du body.
- logs enrichis en cas `decode_batch` / packet id invalide.
3. Handlers critiques:
- `RequestChunkRadius`: log détaillé `requested/max/accepted`.
- `SetLocalPlayerAsInitialized`: log détaillé en cas d’état inattendu ou decode fail.
- `ServerboundLoadingScreen`: message explicite que `loading_screen_id=None` est en général normal.
4. Outils internes:
- `packet_name()`, `hex_preview()`, `should_trace_packet()`.
- toggle env: `MC_RS_TRACE_SPAWN=0` pour couper le trace.

### Vérification
- `cargo fmt` -> OK
- `cargo build -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK

---

## 2026-03-02 - Fix ItemRegistry canonique (version + component_nbt réels)

### Constat
- `ItemRegistryPacket` était envoyé avec:
  - `version = 0` pour tous les items
  - `component_nbt = {}` vide pour tous les items
- Or `item_list.json` contient de vrais `version` et des `component_nbt` (base64) pour les items `component_based=true`.
- PocketMine envoie ces champs canoniques depuis BedrockData.

### Correctif appliqué
Fichiers modifiés:
- `crates/mc-rs-world/src/item_registry.rs`
- `crates/mc-rs-world/Cargo.toml`
- `crates/mc-rs-server/src/connection/login.rs`

Changements:
1. Décodage base64 de `component_nbt` au chargement du registry.
2. Transport de `version` + `component_nbt` dans `ItemTableEntry`.
3. Envoi réel de ces champs dans `ItemRegistryPacket` (au lieu de `0` + NBT vide).
4. Tri déterministe des entrées item table (`numeric_id`, puis `string_id`).
5. Test ajouté: `component_based_item_keeps_component_nbt`.

### Extrait de code ajouté
```rust
let component_nbt = entry
    .component_nbt
    .as_deref()
    .map(|b64| {
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap_or_else(|e| panic!("invalid component_nbt base64 for item {name}: {e}"))
    })
    .unwrap_or_default();

// login.rs: envoi ItemRegistryPacket
.map(|e| packets::ItemRegistryEntry {
    string_id: e.string_id,
    numeric_id: e.numeric_id,
    is_component_based: e.is_component_based,
    version: e.version,
    component_nbt: e.component_nbt,
})
```

### Vérification
- `cargo test -p mc-rs-world item_registry -- --nocapture` -> OK
- `cargo test -p mc-rs-proto start_game -- --nocapture` -> OK
- `cargo build --release -p mc-rs-server` -> OK

---

## 2026-03-02 - Correctif format NBT `component_nbt` (LE -> Network)

### Cause identifiée
- `required_item_list.json`/`item_list.json` stocke `component_nbt` en **NBT little-endian** (PocketMine le parse via `LittleEndianNbtSerializer`).
- Le packet `ItemRegistry` attend des compounds NBT en **format network**.
- Une version précédente envoyait le base64 décodé brut, donc mauvais format wire.

### Fix appliqué
Fichier modifié:
- `crates/mc-rs-world/src/item_registry.rs`

Changement:
1. `component_nbt`:
- decode base64
- parse via `read_nbt_le(...)`
- re-encode via `write_nbt_network(...)`
2. En cas de donnée invalide: panic explicite avec le nom d’item.

### Extrait code
```rust
let raw_le = base64::engine::general_purpose::STANDARD.decode(b64)?;
let mut cursor = raw_le.as_slice();
let root = read_nbt_le(&mut cursor)?;
let mut network = BytesMut::new();
write_nbt_network(&mut network, &root);
```

### Vérification
- `cargo test -p mc-rs-world item_registry -- --nocapture` -> OK
- `cargo build --release -p mc-rs-server` -> OK

---

## 2026-03-02 - Correctif du correctif: `NetworkChunkPublisherUpdate.saved_chunks` = `u32 LE` (pas VarUInt)

### Erreur précédente
- Une entrée précédente de ce log indiquait à tort que `saved_chunks.len()` devait être encodé en `VarUInt32`.
- Vérification PocketMine: `NetworkChunkPublisherUpdatePacket::encodePayload()` écrit ce champ via `LE::writeUnsignedInt(...)`.

### Fix appliqué (version courante)
Fichier modifié :
- `crates/mc-rs-proto/src/packets/network_chunk_publisher_update.rs`

Changements :
1. `saved_chunks.len()` encodé en `buf.put_u32_le(len)`.
2. Test de régression ajouté: validation explicite de la lecture en `u32_le`.
3. Test `saved_chunks=[]` mis à jour: suffixe attendu `00 00 00 00`.

### Extrait code
```rust
self.position.proto_encode(buf);
VarUInt32(self.radius).proto_encode(buf);
// PocketMine writes this as LE u32, not VarUInt.
buf.put_u32_le(self.saved_chunks.len() as u32);
for chunk in &self.saved_chunks {
    chunk.proto_encode(buf);
}
```

### Vérification
- `cargo test -p mc-rs-proto network_chunk_publisher_update` -> OK
- `cargo build --release -p mc-rs-server` -> OK

---

## 2026-03-02 - Alignement PocketMine sur runtime IDs blocs (fix blocage spawn)

### Cause confirmée
- Écart de stratégie entre `mc-rs` et PocketMine sur les IDs de blocs réseau:
- `mc-rs` envoyait des hashes FNV internes dans les palettes chunks (`StartGame.block_network_ids_are_hashes=true`).
- PocketMine envoie des runtime IDs canoniques Bedrock (`blockNetworkIdsAreHashes=false`) et sérialise les chunks avec ces IDs.
- Ce décalage empêchait la finalisation du chargement client (pas de `SetLocalPlayerAsInitialized`).

### Fix appliqué
Fichiers modifiés:
- `crates/mc-rs-world/src/network_runtime_ids.rs` (nouveau)
- `crates/mc-rs-world/src/serializer.rs`
- `crates/mc-rs-world/src/lib.rs`
- `crates/mc-rs-world/Cargo.toml`
- `crates/mc-rs-server/src/connection/login.rs`
- `crates/mc-rs-proto/src/packets/start_game.rs`

Changements:
1. Ajout d’un mapping `hash interne -> runtime ID canonique` construit depuis `canonical_block_states.nbt`.
2. Serializer chunk: conversion des entrées de palette blocs vers runtime IDs canoniques avant écriture VarInt.
3. `StartGame`: `block_network_ids_are_hashes=false` (comportement PocketMine).
4. Compat legacy bedrock conservée (`LEGACY_BEDROCK_HASH_EMPTY_STATES`).

### Extrait code clé
```rust
// serializer.rs
let rid = to_network_runtime_id(runtime_id_hash);
write_signed_varint32(buf, rid as i32);

// login.rs
block_network_ids_are_hashes: false,
```

### Vérification
- `cargo test -p mc-rs-world serializer::` -> OK
- `cargo test -p mc-rs-world network_runtime_ids::` -> OK
- `cargo test -p mc-rs-proto start_game` -> OK
- `cargo build --release -p mc-rs-server` -> OK

---

## 2026-03-02 - Fix critique spawn flat: mauvais fallback sur générateur Nether

### Symptôme observé
- En monde `flat`, le client reste bloqué sur "Création du monde".
- Logs: chunks envoyés avec `sub_chunks=5` (flat normal) mais `StartGame.player_position.y` autour de `35.62` (anormal pour flat).

### Cause racine
- Dans `ConnectionHandler::new`, le calcul de spawn utilisait une chaîne:
- `if overworld_generator ... else if nether_generator ... else if end_generator ...`
- En mode `flat`, `overworld_generator=None` mais `nether_generator=Some(...)` (toujours initialisé), donc spawn pris par erreur sur le générateur Nether.

### Impact technique
- Spawn joueur au-dessus des sous-chunks envoyés pour le flat (`sub_chunk_count=5`, hauteur utile jusqu'à `y=15`).
- Le client peut rester en attente de finalisation et ne jamais envoyer `SetLocalPlayerAsInitialized`.

### Correctif appliqué
Fichier modifié:
- `crates/mc-rs-server/src/connection/mod.rs`

Changement:
1. Calcul du spawn basé sur `dimension_id` (dimension active), pas sur la simple présence d'un générateur.
2. Pour l'overworld flat: spawn explicite conservé sur couche herbe (`Vec3(0.5, 5.62, 0.5)`, `BlockPos(0,4,0)`).

### Extrait de code appliqué
```rust
let (spawn_position, spawn_block) = match dimension_id {
    1 => {
        let feet_y = nether_generator.as_ref().map(|g| g.find_spawn_y()).unwrap_or(64);
        let eye_y = feet_y as f32 + 1.62;
        (Vec3::new(8.5, eye_y, 8.5), BlockPos::new(8, feet_y, 8))
    }
    2 => {
        let feet_y = end_generator.as_ref().map(|g| g.find_spawn_y()).unwrap_or(64);
        let eye_y = feet_y as f32 + 1.62;
        (Vec3::new(8.5, eye_y, 8.5), BlockPos::new(8, feet_y, 8))
    }
    _ => {
        if let Some(ref gen) = overworld_generator {
            let feet_y = gen.find_spawn_y();
            let eye_y = feet_y as f32 + 1.62;
            (Vec3::new(8.5, eye_y, 8.5), BlockPos::new(8, feet_y, 8))
        } else {
            (Vec3::new(0.5, 5.62, 0.5), BlockPos::new(0, 4, 0))
        }
    }
};
```
