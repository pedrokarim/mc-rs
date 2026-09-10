# 31 – Protocole 2169 (Bedrock 1.26.45)

Montée de version réalisée le 2026-09-10 : protocole **1001 (1.26.30) → 2169 (1.26.45)**, en passant par 2168
(1.26.40, « r/26_u4 »). 2169 est un hotfix de 2168 (« r/26_u4 hotfix 5 ») ; les données de jeu sont
identiques entre 1.26.40 et 1.26.45.

La version est définie une seule fois : `CURRENT_PROTOCOL` et `MINECRAFT_VERSION` dans
`crates/mc-rs-proto/src/lib.rs`. Serveur, MOTD, query, StartGame, pile de packs et simulateur les utilisent.
Seuls les tests et valeurs par défaut de `mc-rs-raknet` (qui ne dépend pas de `mc-rs-proto`) portent la valeur
en dur.

## 1. Contexte : fin de support de PocketMine-MP

PocketMine-MP a annoncé la **fin de son support le 2026-07-09** (README de la branche `stable`, commit
`6a7cc02e9`). Sa dernière version, 5.44.3, s’arrête au protocole 1001, et `pmmp/BedrockData` à 1.26.30.
PMMP reste la référence pour la **logique serveur** (exécuteur d’inventaire, handlers…) et pour tout ce qui n’a
pas changé depuis 1001 ; il ne peut plus servir de référence pour les **formats** au-delà de 1001.

Références utilisées pour 2168/2169, toujours croisées entre elles :

| Référence | Emplacement | Rôle |
|---|---|---|
| Changelogs Mojang | `.reference/bedrock-protocol-docs/legacy_changelogs/changelog_2168_07_07_26.md`, `changelog_2169_08_18_26.md` | Liste officielle des changements |
| Schémas JSON Mojang | `.reference/bedrock-protocol-docs` au tag `v1.26.45` (`json/*.json`) | Format final, champ par champ |
| gophertunnel | `.reference/gophertunnel`, `git diff v1.57.0 origin/master` (v1.57.0 = 1001, master = 1.26.45) | Implémentation Go éprouvée en production (dragonfly) |
| CloudburstProtocol | `.reference/CloudburstProtocol/bedrock-codec/.../codec/v2168/` | Implémentation Java ; un serializer n’y est recréé que pour les paquets modifiés |
| BedrockProtocol 58.0.1 | `.reference/BedrockProtocol` | Base exacte 1001 |

Le dépôt local `.reference/gophertunnel` contient des modifications personnelles : ne pas y faire de `git pull`.
Pour lire master, utiliser `git show origin/master:<fichier>` ou un worktree temporaire.

### Lire les schémas JSON Mojang

- Entier avec `x-serialization-options: ["Compression"]` : VarInt (zigzag si signé). Sans : entier fixe
  little-endian. Exception : sur les champs 8 bits, `Compression` n’a pas d’effet (un octet brut chez
  gophertunnel, Cloudburst et PMMP).
- `oneOf` avec `x-control-value-type: uint32` : variante taguée, index VarUInt32 puis charge utile.
- Un champ hors de `required` et sans `default` est optionnel (booléen de présence). Certains optionnels sont
  « doubles » (booléen externe toujours vrai, booléen de présence, valeur) : le JSON ne les distingue pas, on
  suit gophertunnel et Cloudburst quand ils concordent.
- Une énumération sans `Enum-as-Value` est écrite sous forme de **chaîne**.
- Pour les paquets non convertis en Cereal (ChangeDimension, LevelSoundEvent, SpawnParticleEffect…), le JSON
  oublie des booléens de présence que les trois implémentations écrivent : on suit les implémentations.

## 2. Changements appliqués

### 2.1 Conversion « Cereal » (2168)

| Paquet | Changement |
|---|---|
| StartGame | `isLoggingChat` retiré ; permissions joueur en `u8` ; offre Education en VarUInt32 ; règles entières en `u32` LE |
| GameRule (StartGame et GameRulesChanged) | Un seul encodage : index de variante, entier `u32` LE |
| ResourcePacksInfo | Nombre de packs en VarUInt32 (u16 auparavant) |
| ResourcePackClientResponse | Variante taguée (0 cancel, 1 downloading, 2 downloadingfinished, 3 resourcepackstackfinished) + nom en chaîne + packs pour « downloading » seulement. Numérotation décalée d’un cran par rapport au statut 1001 |
| NetworkItemStackDescriptor | Plus d’index de variante devant le net id (booléen + VarInt32) |
| NetworkItemInstanceDescriptor (créatif, recettes) | Plus de court-circuit sur l’air (6 octets) ; block runtime id en VarInt32 zigzag |
| Métadonnées d’entité | Type écrit deux fois : VarUInt32 (variante) puis `u8` (champ `Type`) |
| AddItemActor, AddPlayer | Item au format NetworkItemStackDescriptor (AddPlayer : sans net id) |
| LevelChunk | Limite de requête client optionnelle + liste de blobs de cache toujours présente |
| MovePlayer | Données de téléportation optionnelles |
| PlayerList | Entrées en variante taguée (variante Add = 1 / Remove = 0 puis octet d’action Add = 0 / Remove = 1), plus de liste finale « skin vérifiée » |
| SerializedSkin | Compteurs en VarUInt32, bras en `u8`, couleur ARGB `i32`, drapeau de confiance (chaîne) et `ProfileHash` |
| PlayerAuthInput | Drapeaux en liste d’identifiants (0 à 65) ; modèle d’interaction en VarInt32 ; cinq optionnels doubles ; actions de bloc avec position toujours présente |
| ItemStackRequest | Index de variante + id legacy ; net id en `i32` LE ; CraftRecipeAuto sans doublon ; ingrédients au format `StackRequestItemDescriptorCount` |
| ItemStackResponse | Conteneurs et net id de slot en optionnels doubles ; nom filtré optionnel |
| CraftingData | 8 vecteurs de recettes typés, puis potions, mélanges, réducteurs, `clearRecipes` ; recettes façonnées avec nombre d’ingrédients ; condition de déblocage ; ingrédients par nom |
| CreativeContent | Catégorie de groupe en `u8` |
| Transfer | GatheringsConfiguration optionnelle |
| PlaySound | Nombre de boucles |
| UpdateAbilities | Nombre de couches en VarUInt32 |

### 2.2 Paquets restés en retard avant 1001 et corrigés

La montée 975 → 1001 n’avait porté que StartGame. Ont été corrigés au passage :

- InventoryTransaction (format pré-985 : ClickAir était lu comme 2, la face 1 comme -1) ;
- InventoryContent (ancien format d’item, storage item faux) ;
- LevelSoundEvent (nom de son en chaîne depuis 993, nous envoyions un identifiant numérique) ;
- BossEvent (tous les champs écrits à chaque événement depuis 984 ; blanc = 7, 6 = violet Rebecca) ;
- ActorEvent (position de tir optionnelle manquante), BlockActorData rediffusé (préfixe de longueur en trop) ;
- AddPlayer (PropertySyncData manquant, ordre des abilities, clés de métadonnées) ;
- PlayStatus/Disconnect : `Kicked` = 55, `Shutdown` = 68 (`DisconnectFailReason`) ;
- mode spectateur : 3 n’existe plus dans l’énumération ; comme PMMP, le spectateur est envoyé en créatif (1).

## 3. Données de jeu 1.26.40

`pmmp/BedrockData` s’arrêtant à 1.26.30, les données viennent de **dragonfly v0.11.4** (commit `0c2c4045`,
même format que PMMP) :

| Fichier (`crates/mc-rs-server/data/`) | Source | Remarque |
|---|---|---|
| `canonical_block_states.nbt` | dragonfly `server/world/block_states.nbt` | 17 499 états, 1 379 blocs |
| `vanilla_items.nbt` | dragonfly `server/world/vanilla_items.nbt` | Lu directement par `item_registry.rs` |
| `creative_items.nbt` | dragonfly `server/item/creative/creative_items.nbt` | Lu directement par `creative_content.rs` |
| `entity_identifiers.nbt` | CloudburstMC/Data `entity_identifiers.dat` @ `040a6cc` | `minecraft:cushion` absent de toutes les sources disponibles |

Les recettes, biomes, entités et tables de butin viennent toujours de bedrock-samples 1.26.10.4 : leur mise à
jour vers `v1.26.40.05` est un chantier séparé (données de gameplay, sans lien avec le format réseau).

### Migration des données sauvegardées

Les sous-chunks sont persistés avec des runtime IDs bruts ; une nouvelle palette change donc le sens de chaque
bloc sauvegardé.

- `generate_block_registry` émet `PALETTE_FINGERPRINT` (FNV-1a 64 du fichier de palette).
- `generate_data_migrations` génère une table ancien ID → ID courant appariée sur `(nom, états)`
  (`world/palette_migration_data.rs`) et la table d’items héritée (`item_migration_data.rs`).
- À l’ouverture, `WorldStorage` compare le marqueur LevelDB `mcrs:palette_fingerprint` à la palette
  courante, réécrit si besoin les palettes de tous les sous-chunks (couche par couche, sans toucher aux mots de
  bits), puis pose le marqueur. Un monde sans marqueur est considéré comme écrit en 1.26.20.
- Les sauvegardes joueurs portent `data_version` ; la version 0 est convertie (ID d’item par nom, block
  runtime ID par la table de palette).

## 4. Points à confirmer sur un vrai client 1.26.45

Le simulateur et les décodeurs ont été écrits d’après les mêmes spécifications : leur accord valide la
cohérence interne et le flux complet (login, packs, spawn, entrées), pas la compatibilité avec le client
officiel. Points où les références divergent, choix retenu entre parenthèses :

1. Ingrédients de CraftingData : gophertunnel + Cloudburst (variante 0/1, type en chaîne, quantité VarInt32)
   contre le JSON Mojang (forme ItemStackRequest). Un mauvais choix empêche le client de lire tout le paquet.
2. Net id des slots d’ItemStackRequest : `i32` LE (gophertunnel + Cloudburst) contre la mention d’une variante
   taguée dans le changelog.
3. Présence dans ItemStackResponse : conteneurs ssi succès, net id ssi quantité non nulle (invariants Mojang).
4. Booléen de présence en tête des drapeaux PlayerAuthInput (gophertunnel + Cloudburst, invisible dans le JSON).
5. Couleur de skin : `"#rrggbb"` converti en ARGB comme Cloudburst (`Color.getRGB()`).

En cas de problème, `connection/movement.rs` journalise la première erreur de décodage PlayerAuthInput du
processus en `WARN`.

## 5. Procédure pour la prochaine version

1. `update-refs`, puis identifier la dernière release dans `bedrock-protocol-docs` (tags sans `preview`) et
   sa version de protocole (`x-protocol-version` des JSON au tag).
2. Lister les paquets modifiés : changelog Mojang, `Bedrock_vNNNN.java` de Cloudburst (liste des
   `updateSerializer`), diff gophertunnel entre les tags correspondants.
3. Comparer **notre code** au format cible (pas au format précédent supposé).
4. Mettre à jour `CURRENT_PROTOCOL` / `MINECRAFT_VERSION`.
5. Si la palette change : importer le nouveau fichier, relancer `generate_block_registry`, puis
   `generate_data_migrations <ancienne_palette> <ancienne_table_items> <libellé>` en ajoutant la nouvelle
   migration à `PALETTE_MIGRATIONS` (les anciennes restent valides pour les mondes plus vieux).
6. Valider avec `mc-rs-sim` (charge et `--mode probe`), puis avec un vrai client.
