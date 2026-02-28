# 09 - Sécurité : Authentification, chiffrement et anti-triche

## Authentification Xbox Live

### Vue d'ensemble

Minecraft Bedrock utilise l'authentification **Xbox Live / Microsoft Account** pour vérifier l'identité des joueurs. Le flux est basé sur des **JWT (JSON Web Tokens)** signés.

### Chaîne JWT dans LoginPacket

Le `LoginPacket` contient deux éléments :
1. **Identity Chain** — Chaîne de JWT signés par Mojang/Xbox Live
2. **Client Data** — JWT signé par le client avec ses données (skin, device, etc.)

### Identity Chain

```json
{
    "chain": [
        "eyJ...(JWT 1 signé par le client)...",
        "eyJ...(JWT 2 signé par Mojang)...",
        "eyJ...(JWT 3 signé par Mojang — contient l'identité)..."
    ]
}
```

**Validation :**

```
JWT 1 (self-signed par le client) :
  Header : { "alg": "ES384", "x5u": "<clé publique du client>" }
  Payload : { "certificateAuthority": true, "identityPublicKey": "<clé publique>" }

JWT 2 (signé par Mojang) :
  Header : { "alg": "ES384", "x5u": "<clé publique Mojang>" }
  Payload : { "certificateAuthority": true, "identityPublicKey": "<clé publique>" }

JWT 3 (signé par Mojang — identité finale) :
  Header : { "alg": "ES384", "x5u": "<clé publique Mojang>" }
  Payload : {
      "extraData": {
          "XUID": "1234567890123456",
          "identity": "12345678-1234-1234-1234-123456789012",
          "displayName": "Gamertag",
          "titleId": "896928775",
          "sandboxId": "RETAIL"
      },
      "identityPublicKey": "<clé publique du client>",
      "nbf": 1700000000,
      "exp": 1700086400,
      "iat": 1700000000,
      "iss": "Mojang",
      "randomNonce": 123456789
  }
```

### Processus de validation

```rust
pub fn validate_login_chain(chain: &[String]) -> Result<PlayerIdentity> {
    // 1. La chaîne doit avoir exactement 3 JWT
    assert!(chain.len() == 3);

    // 2. Vérifier que la chaîne est signée par Mojang
    //    La clé publique racine de Mojang est :
    const MOJANG_ROOT_KEY: &str = "MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAECRXueJeTDqNRRgJi/vlRufByu/2G0i2Ebt6YMar5QX/R0DIIyrJMcUpruK4QveTfJSTp3Shlq4Gk34cD/4GUWwkv0DVuzeuB+tXija7HBxii03NHDbPAD0AKnLr2wdAp";

    // 3. Pour chaque JWT dans la chaîne :
    //    a. Décoder le header (vérifier alg = ES384)
    //    b. Vérifier la signature avec la clé publique du JWT précédent
    //       (le premier est self-signed, les suivants utilisent x5u du précédent)
    //    c. Vérifier que la chaîne finit par une clé signée par Mojang root

    let mut current_key = None;
    let mut is_signed_by_mojang = false;

    for jwt in chain {
        let header = decode_jwt_header(jwt)?;
        let x5u = header.x5u; // Clé publique du signataire

        if let Some(expected_key) = &current_key {
            // Vérifier que x5u correspond à la clé attendue
            verify_jwt_signature(jwt, expected_key)?;
        }

        if x5u == MOJANG_ROOT_KEY {
            is_signed_by_mojang = true;
        }

        let payload = decode_jwt_payload(jwt)?;
        current_key = Some(payload.identity_public_key);
    }

    // 4. Si aucun JWT n'est signé par Mojang → joueur non authentifié
    //    (mode offline possible si configuré)

    // 5. Extraire l'identité du dernier JWT
    let last_payload = decode_jwt_payload(&chain[2])?;
    Ok(PlayerIdentity {
        xuid: last_payload.extra_data.xuid,
        uuid: last_payload.extra_data.identity,
        gamertag: last_payload.extra_data.display_name,
        public_key: last_payload.identity_public_key,
        authenticated: is_signed_by_mojang,
    })
}
```

### Mode offline

Pour le développement ou les serveurs privés, on peut désactiver la vérification Xbox Live :
- Accepter les chaînes self-signed
- Générer un UUID basé sur le nom du joueur
- **Attention :** Pas de vérification d'identité, n'importe qui peut se faire passer pour un autre

```toml
# server.toml
[authentication]
online_mode = true    # false pour désactiver Xbox Live auth
```

## Chiffrement (Encryption)

### Établissement du chiffrement

Après la validation du login :

```
1. Le serveur génère une paire de clés ECDH P-384
   server_private_key, server_public_key = generate_ec_keypair(P-384)

2. Le serveur envoie ServerToClientHandshake
   {
       "salt": "<16 bytes random, base64>",
       "publicKey": "<server_public_key, base64 DER>"
   }
   (Ce JWT est signé avec la clé privée du serveur)

3. Les deux côtés calculent le secret partagé
   shared_secret = ECDH(server_private_key, client_public_key)
   // client_public_key vient de l'identityPublicKey dans le login JWT

4. Dérivation de la clé AES
   // Concatener salt + shared_secret
   key_material = SHA256(salt + shared_secret)
   // Les 32 premiers octets = clé AES-256
   aes_key = key_material[0..32]
   // IV initial = les 16 premiers octets de la clé
   initial_iv = aes_key[0..16]

5. Le client envoie ClientToServerHandshake (confirme)

6. Tous les paquets suivants sont chiffrés en AES-256-CFB8
```

### AES-256-CFB8

```rust
use aes::Aes256;
use cfb8::Cfb8;
use cfb8::cipher::{AsyncStreamCipher, NewCipher};

pub struct PacketEncryption {
    encrypt_cipher: Cfb8<Aes256>,
    decrypt_cipher: Cfb8<Aes256>,
    send_counter: u64,
    recv_counter: u64,
}

impl PacketEncryption {
    pub fn new(key: &[u8; 32], iv: &[u8; 16]) -> Self {
        Self {
            encrypt_cipher: Cfb8::<Aes256>::new_from_slices(key, iv).unwrap(),
            decrypt_cipher: Cfb8::<Aes256>::new_from_slices(key, iv).unwrap(),
            send_counter: 0,
            recv_counter: 0,
        }
    }

    pub fn encrypt(&mut self, data: &mut [u8]) {
        // Ajouter le checksum avant chiffrement
        // checksum = SHA256(counter_le_bytes + data + key)[0..8]
        self.encrypt_cipher.encrypt(data);
        self.send_counter += 1;
    }

    pub fn decrypt(&mut self, data: &mut [u8]) {
        self.decrypt_cipher.decrypt(data);
        // Vérifier le checksum
        self.recv_counter += 1;
    }
}
```

### Checksum des paquets

Chaque paquet chiffré inclut un checksum SHA-256 de 8 octets :

```
Avant chiffrement, le paquet est :
  [compressed_payload] + [checksum: 8 bytes]

Où checksum = SHA256(
    counter.to_le_bytes() +    // Compteur de paquets (u64 LE)
    compressed_payload +       // Données compressées
    aes_key                    // Clé AES
)[0..8]                        // Premiers 8 octets du hash
```

### Stack cryptographique en Rust

```toml
[dependencies]
# JWT
jsonwebtoken = "9"

# ECDH P-384
p384 = { version = "0.13", features = ["ecdh", "ecdsa"] }
elliptic-curve = { version = "0.13", features = ["ecdh"] }

# AES-256-CFB8
aes = "0.8"
cfb8 = "0.8"

# SHA-256
sha2 = "0.10"

# Base64
base64 = "0.22"

# Nombres aléatoires sécurisés
rand = "0.8"
```

## Anti-triche

### Mouvement server-authoritative

Le principal mécanisme anti-triche de Bedrock :

```rust
pub struct MovementValidator {
    last_position: Vec3,
    last_velocity: Vec3,
    last_tick: u64,
    violations: u32,
    on_ground: bool,
}

impl MovementValidator {
    pub fn validate(&mut self, input: &PlayerAuthInput, world: &World) -> ValidationResult {
        let dt = (input.tick - self.last_tick) as f64 * 0.05; // Secondes

        // 1. Vérifier la vitesse
        let distance = input.position.distance_to(&self.last_position);
        let max_speed = self.calculate_max_speed(input);
        if distance > max_speed * dt * 1.5 { // 50% de marge
            self.violations += 1;
            return ValidationResult::SpeedViolation;
        }

        // 2. Vérifier le vol (si pas en créatif/spectateur)
        if !self.can_fly && input.position.y > self.last_position.y + JUMP_VELOCITY * dt + 0.1 {
            if !self.on_ground && self.last_velocity.y <= 0.0 {
                self.violations += 1;
                return ValidationResult::FlyViolation;
            }
        }

        // 3. Vérifier le no-clip
        let bbox = player_bounding_box(input.position);
        if world.intersects_solid_blocks(&bbox) {
            self.violations += 1;
            return ValidationResult::NoClipViolation;
        }

        // 4. Appliquer la correction si trop de violations
        if self.violations > 5 {
            self.violations = 0;
            return ValidationResult::Correct(self.last_valid_position);
        }

        self.last_position = input.position;
        self.last_tick = input.tick;
        ValidationResult::Ok
    }
}
```

### Inventaire server-authoritative

```rust
pub struct InventoryValidator {
    /// Vérifier qu'un ItemStackRequest est valide
    pub fn validate_request(
        &self,
        request: &ItemStackRequest,
        player_inventory: &PlayerInventory,
        opened_container: Option<&Container>,
    ) -> Result<(), InventoryViolation> {
        for action in &request.actions {
            match action {
                Action::Take { source, destination, count } => {
                    // Vérifier que la source contient l'item
                    // Vérifier que la destination a de la place
                    // Vérifier que le count est valide
                }
                Action::CraftRecipe { recipe_id } => {
                    // Vérifier que la recette existe
                    // Vérifier que les ingrédients sont présents
                    // Vérifier que la station de craft est correcte
                }
                // ... autres actions
            }
        }
        Ok(())
    }
}
```

### Validation de la portée (reach)

```rust
const MAX_BREAK_DISTANCE: f64 = 6.0;  // Blocs (créatif : 13.0)
const MAX_INTERACT_DISTANCE: f64 = 6.0;
const MAX_ATTACK_DISTANCE: f64 = 6.0;

pub fn validate_block_break(
    player_pos: Vec3,
    block_pos: BlockPos,
    game_mode: GameMode,
) -> bool {
    let max_dist = match game_mode {
        GameMode::Creative => 13.0,
        _ => MAX_BREAK_DISTANCE,
    };
    player_pos.distance_to(&block_pos.center()) <= max_dist
}
```

### Validation de la vitesse de minage

```rust
/// Calculer le temps de minage minimum pour un bloc
pub fn min_break_time(
    block: &BlockState,
    held_item: &ItemStack,
    effects: &[Effect],
    on_ground: bool,
    in_water: bool,
) -> f64 {
    let hardness = block.hardness();
    if hardness < 0.0 { return f64::INFINITY; } // Incassable
    if hardness == 0.0 { return 0.0; }            // Instant

    let mut speed = 1.0;

    // Outil approprié
    if is_correct_tool(held_item, block) {
        speed = tool_speed(held_item);
        // Enchantement Efficiency
        let efficiency_level = get_enchantment_level(held_item, "efficiency");
        if efficiency_level > 0 {
            speed += (efficiency_level * efficiency_level + 1) as f64;
        }
    }

    // Effet Haste
    if let Some(haste) = effects.iter().find(|e| e.effect_type == EffectType::Haste) {
        speed *= 1.0 + 0.2 * (haste.amplifier + 1) as f64;
    }

    // Effet Mining Fatigue
    if let Some(fatigue) = effects.iter().find(|e| e.effect_type == EffectType::MiningFatigue) {
        speed *= 0.3_f64.powi((fatigue.amplifier + 1).min(3) as i32);
    }

    // Pénalité sous l'eau (sans Aqua Affinity)
    if in_water && !has_enchantment(/* helmet */, "aqua_affinity") {
        speed /= 5.0;
    }

    // Pénalité en l'air
    if !on_ground {
        speed /= 5.0;
    }

    let damage = speed / hardness;
    let is_correct = is_correct_tool(held_item, block);
    let damage_per_tick = if is_correct { damage / 30.0 } else { damage / 100.0 };

    if damage_per_tick >= 1.0 { return 0.0; } // Instant break

    (1.0 / damage_per_tick).ceil() as f64 * 0.05 // Convertir ticks en secondes
}
```

### Protections supplémentaires

| Protection | Description | Implémentation |
|-----------|-------------|----------------|
| **Rate limiting** | Limiter les paquets par seconde | Max ~200 paquets/sec par joueur |
| **Packet size** | Taille max des paquets | Rejeter les paquets > MTU * fragments raisonnables |
| **Invalid packets** | Paquets malformés | Try/catch sur la désérialisation, kick après N erreurs |
| **Chat spam** | Messages trop fréquents | Cooldown de 1 seconde entre messages |
| **Command spam** | Commandes trop fréquentes | Cooldown configurable |
| **Skin validation** | Skins invalides/offensantes | Vérifier dimensions (64×64, 128×128), taille max |
| **Login flood** | Trop de connexions | Rate limit par IP, ban temporaire |

### Configuration anti-triche

```toml
# server.toml
[anticheat]
enabled = true

[anticheat.movement]
max_violations_before_correct = 5
max_violations_before_kick = 20
speed_tolerance = 1.5          # Multiplicateur de tolérance
fly_detection = true
noclip_detection = true

[anticheat.combat]
reach_check = true
max_attack_distance = 6.0
hit_rate_limit = 20            # Max attaques par seconde

[anticheat.mining]
speed_check = true
reach_check = true
tolerance = 0.9                # 90% du temps minimum

[anticheat.network]
max_packets_per_second = 200
max_packet_size = 65536
invalid_packet_threshold = 10  # Kick après N paquets invalides

[anticheat.chat]
message_cooldown_ms = 1000
max_message_length = 256
```

## Sécurité réseau

### Protection DDoS (basique)

```rust
pub struct ConnectionRateLimit {
    connections_per_ip: HashMap<IpAddr, (u32, Instant)>,
    max_connections_per_ip: u32,
    window: Duration,
}

impl ConnectionRateLimit {
    pub fn should_allow(&mut self, ip: IpAddr) -> bool {
        let entry = self.connections_per_ip.entry(ip).or_insert((0, Instant::now()));
        if entry.1.elapsed() > self.window {
            *entry = (1, Instant::now());
            return true;
        }
        entry.0 += 1;
        entry.0 <= self.max_connections_per_ip
    }
}
```

### Whitelist / Blacklist

```toml
# server.toml
[security]
whitelist_enabled = false
blacklist_file = "banned-players.json"
banned_ips_file = "banned-ips.json"

# banned-players.json
[
    {
        "xuid": "1234567890",
        "name": "BadPlayer",
        "reason": "Cheating",
        "banned_by": "Admin",
        "date": "2024-01-01T00:00:00Z",
        "expires": null
    }
]
```
