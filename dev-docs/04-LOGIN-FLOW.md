# 04 - Login Flow

## PocketMine : Flux de connexion complet

### State Machine

```
SessionStart → Login → Handshake → ResourcePacks → PreSpawn → SpawnResponse → InGame
                                                                                  ↕
                                                                                Death
```

Chaque état a son propre `PacketHandler` qui ne traite que les paquets valides pour cette phase.

---

### Phase 1 : SessionStart

**Handler :** `SessionStartPacketHandler`

```
Client                              Server
  |                                   |
  |--- RequestNetworkSettings ------->|  protocol=924
  |<-- NetworkSettings ---------------|  compression_algo, compression_threshold
  |                                   |
  [Compression activée à partir d'ici]
```

**RequestNetworkSettingsPacket :**
- `protocol: i32` (BE) → doit être 924

**NetworkSettingsPacket :**
- `compression_threshold: u16_le`
- `compression_algorithm: u16_le` (0=zlib, 1=snappy)
- `client_throttle_enabled: bool`
- `client_throttle_threshold: u8`
- `client_throttle_scalar: f32_le`

---

### Phase 2 : Login

**Handler :** `LoginPacketHandler`

```
Client                              Server
  |                                   |
  |--- LoginPacket ------------------>|  protocol (BE), JWT chain, client data
  |                                   |  [Validation Xbox Live ou offline]
  |                                   |  [Fire PlayerPreLoginEvent]
  |<-- ServerToClientHandshake -------|  JWT avec clé publique serveur
```

**LoginPacket :**
```
protocol: i32 (BE)
payload_length: VarUInt32
  └─ chain_data_length: i32_le
     chain_data: JSON string
       {
         "chain": [
           "JWT1",  // Mojang root key (Xbox Live)
           "JWT2",  // Identity data
           "JWT3"   // Client identity
         ]
       }
     client_data_length: i32_le
     client_data: JWT string (skin, device info, etc.)
```

**Contenu du JWT identity :**
```json
{
  "extraData": {
    "displayName": "Player123",
    "identity": "uuid-here",
    "XUID": "xbox-user-id",
    "titleId": "title-id"
  },
  "identityPublicKey": "base64-ecdsa-key"
}
```

**Contenu du client data JWT :**
```json
{
  "DeviceModel": "...",
  "DeviceOS": 1,
  "GameVersion": "1.26.0",
  "LanguageCode": "en_US",
  "SkinId": "...",
  "SkinData": "base64...",
  "SkinGeometryData": "base64...",
  // ... +30 champs
}
```

**Validation Xbox Live :**
1. Vérifier la chaîne JWT contre les clés publiques Mojang
2. Valider les signatures ECDSA P-384
3. Extraire identité (displayName, UUID, XUID)
4. `authenticated = true` si chaîne valide

**Mode offline :**
1. JWT self-signed (pas de chaîne Mojang)
2. Extraire identité directement
3. `authenticated = false`

---

### Phase 3 : Handshake (Encryption)

**Handler :** `HandshakePacketHandler`

```
Client                              Server
  |                                   |
  |  [ServerToClientHandshake déjà envoyé en phase 2]
  |                                   |
  |--- ClientToServerHandshake ------>|  (paquet vide, signal de readiness)
  |                                   |
  [Encryption AES-256-CTR activée]    |
  |                                   |
  |<-- PlayStatus (LOGIN_SUCCESS) ----|
```

**Échange de clés :**
1. Serveur génère paire ECDSA P-384 (clé privée + publique)
2. Serveur envoie sa clé publique dans le JWT du `ServerToClientHandshake`
3. Client extrait la clé publique serveur
4. Les deux côtés calculent le secret partagé via ECDH
5. Clé AES-256 = SHA-256(secret partagé)

**Encryption AES-256-CTR (Fake GCM) :**
```
IV = key[0..12] + [0x00, 0x00, 0x00, 0x02]

Encrypt:
  checksum = SHA256(counter_LE_8bytes + payload + key)[0..8]
  encrypted = AES-CTR(payload + checksum)
  counter++

Decrypt:
  decrypted = AES-CTR(encrypted)
  payload = decrypted[0..len-8]
  checksum = decrypted[len-8..len]
  verify SHA256(counter_LE_8bytes + payload + key)[0..8] == checksum
  counter++
```

---

### Phase 4 : Resource Packs

**Handler :** `ResourcePacksPacketHandler`

```
Client                              Server
  |                                   |
  |<-- ResourcePacksInfo -------------|  Liste des packs (UUID, version, taille)
  |                                   |
  |--- ResourcePackClientResponse --->|  status=HAVE_ALL_PACKS ou SEND_PACKS
  |                                   |
  | [Si SEND_PACKS : transfert de packs par chunks de 256KB]
  |<-- ResourcePackDataInfo ----------|
  |--- ResourcePackChunkRequest ----->|
  |<-- ResourcePackChunkData ---------|
  | [Répéter pour chaque pack]        |
  |                                   |
  |<-- ResourcePackStack -------------|  Stack de packs à appliquer
  |                                   |
  |--- ResourcePackClientResponse --->|  status=COMPLETED
```

**ResourcePacksInfoPacket :**
- `must_accept: bool`
- `has_addons: bool`
- `has_scripts: bool`
- `force_server_packs: bool`
- `behavior_packs: Vec<BehaviorPackInfo>`
- `resource_packs: Vec<ResourcePackInfo>` (count = `u16_le`, PAS VarUInt32 !)

**ResourcePackInfo :**
- `uuid: String`
- `version: String`
- `size: u64_le`
- `content_key: String`
- `sub_pack_name: String`
- `content_identity: String`
- `has_scripts: bool`
- `is_addon_pack: bool`
- `is_ray_tracing_capable: bool`
- `cdn_url: String`

---

### Phase 5 : PreSpawn

**Handler :** `PreSpawnPacketHandler`

Le serveur envoie tout ce dont le client a besoin pour spawn :

```
Server → Client (dans l'ordre) :
  1. StartGamePacket              → Config monde, position joueur, game rules
  2. BiomeDefinitionListPacket    → Tous les biomes (NBT blob)
  3. AvailableActorIdentifiers    → Tous les types d'entités (NBT blob)
  4. ItemRegistryPacket           → Tous les items
  5. CraftingDataPacket           → Toutes les recettes
  6. CreativeContentPacket        → Items du mode créatif
  7. PlayerListPacket             → Joueurs en ligne
  8. SetSpawnPositionPacket       → Point de spawn
  9. SetTimePacket                → Heure du monde
  10. SetDifficultyPacket         → Difficulté
  11. SetPlayerGameTypePacket     → Mode de jeu
  12. UpdateAbilitiesPacket       → Capacités joueur
  13. LevelChunkPacket (x N)     → Chunks autour du joueur
  14. PlayStatusPacket            → PLAYER_SPAWN
```

```
Client → Server :
  RequestChunkRadiusPacket        → Distance de vue demandée

Server → Client :
  ChunkRadiusUpdatedPacket        → Distance de vue acceptée
```

---

### Phase 6 : SpawnResponse

**Handler :** `SpawnResponsePacketHandler`

Le client signale qu'il est prêt → transition vers InGame.

---

### Phase 7 : InGame

**Handler :** `InGamePacketHandler` (le plus gros handler, ~42KB en PHP)

Gère tous les paquets de gameplay : mouvement, interaction, inventaire, commandes, chat, etc.

---

## Équivalent Rust

### State Machine

```rust
pub enum ConnectionState {
    SessionStart,
    Login,
    Handshake,
    ResourcePacks,
    PreSpawn,
    SpawnResponse,
    InGame,
    Death,
}

/// Chaque état a son handler
pub trait StateHandler: Send {
    fn handle_packet(
        &mut self,
        ctx: &mut SessionContext,
        packet_id: u32,
        payload: &[u8],
    ) -> Result<Option<ConnectionState>>; // None = rester, Some = transition
}
```

### Handlers par état

```rust
pub struct SessionStartHandler;
pub struct LoginHandler {
    auth_task: Option<JoinHandle<AuthResult>>,
}
pub struct HandshakeHandler;
pub struct ResourcePacksHandler {
    packs_sent: HashSet<String>,
}
pub struct PreSpawnHandler {
    chunks_sent: u32,
    chunks_needed: u32,
}
pub struct SpawnResponseHandler;
pub struct InGameHandler;
pub struct DeathHandler;
```

### Encryption

```rust
pub struct EncryptionContext {
    key: [u8; 32],
    encrypt_counter: u64,
    decrypt_counter: u64,
    // AES-256-CTR cipher instances
}

impl EncryptionContext {
    pub fn new(shared_secret: &[u8]) -> Self {
        let key = sha256(shared_secret);
        let iv = [&key[..12], &[0x00, 0x00, 0x00, 0x02]].concat();
        // Init AES-CTR with key and IV
        Self { key, encrypt_counter: 0, decrypt_counter: 0 }
    }

    pub fn encrypt(&mut self, payload: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.encrypt_counter.to_le_bytes());
        hasher.update(payload);
        hasher.update(&self.key);
        let checksum = &hasher.finalize()[..8];

        let plaintext = [payload, checksum].concat();
        let encrypted = self.aes_ctr_encrypt(&plaintext);
        self.encrypt_counter += 1;
        encrypted
    }

    pub fn decrypt(&mut self, encrypted: &[u8]) -> Result<Vec<u8>> {
        let decrypted = self.aes_ctr_decrypt(encrypted);
        let (payload, checksum) = decrypted.split_at(decrypted.len() - 8);

        let mut hasher = Sha256::new();
        hasher.update(&self.decrypt_counter.to_le_bytes());
        hasher.update(payload);
        hasher.update(&self.key);
        let expected = &hasher.finalize()[..8];

        if checksum != expected {
            return Err(DecryptionError);
        }
        self.decrypt_counter += 1;
        Ok(payload.to_vec())
    }
}
```

### Fichiers PocketMine de référence

```
src/network/mcpe/handler/SessionStartPacketHandler.php
src/network/mcpe/handler/LoginPacketHandler.php
src/network/mcpe/handler/HandshakePacketHandler.php
src/network/mcpe/handler/ResourcePacksPacketHandler.php
src/network/mcpe/handler/PreSpawnPacketHandler.php
src/network/mcpe/handler/SpawnResponsePacketHandler.php
src/network/mcpe/handler/InGamePacketHandler.php
src/network/mcpe/handler/DeathPacketHandler.php
src/network/mcpe/encryption/EncryptionContext.php
src/network/mcpe/auth/ProcessOpenIdLoginTask.php
src/network/mcpe/JwtUtils.php
```
