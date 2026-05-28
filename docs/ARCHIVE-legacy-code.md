# Archive — `old_crates/` et `docs_initial/` (code & doc legacy)

> **Statut : SUPPRIMÉS du repo le 2026-05-28.** Ce document remplace les deux dossiers.
> Le contenu reste récupérable dans l'historique git (dernier état au commit `788b429`
> *« chore(archive): move legacy crates to old_crates »* et antérieurs).

## Pourquoi ce document existe

`old_crates/` et `docs_initial/` étaient la **première tentative** de MC-RS (serveur Minecraft
Bedrock en Rust) — un design « big bang » très ambitieux, écrit avant la réécriture actuelle
basée strictement sur PocketMine-MP. Ce code **ne compilait plus** avec le nouveau workspace et
**polluait** la base de code. Il a été supprimé. Cette page conserve la mémoire de ce que c'était
afin que personne ne le « redécouvre » et ne soit tenté de le réutiliser.

**RÈGLE : ce code ne doit jamais être réutilisé ni servir de référence d'implémentation.**
La seule référence canonique est PocketMine-MP dans `.reference/PocketMine-MP/`. Voir le feedback
mémoire « Ne JAMAIS toucher au vieux code ».

---

## `old_crates/` — l'ancien code

- **Ampleur** : 12 crates, **195 fichiers `.rs`**, **~60 400 lignes**.
- **Origine git** : déplacé en l'état par le commit `788b429`. N'a jamais été dans `members` du
  workspace — il était dans `exclude` de `Cargo.toml` (donc jamais compilé par le build courant).

### Crates communes avec l'actuel (versions abandonnées)
Ces noms existent aussi aujourd'hui dans `crates/`, mais le code legacy était une implémentation
distincte et obsolète :

| Crate | Fichiers | Rôle (legacy) |
|---|---|---|
| `mc-rs-proto` | 86 | Binary IO, batch/codec, compression, item_stack, jwt, ~75 paquets |
| `mc-rs-raknet` | 14 | Transport RakNet : frame/offline/online, reliability, ordering, fragmentation, session, server |
| `mc-rs-crypto` | 5 | aes, ecdh, jwt_sign, key_derive |
| `mc-rs-nbt` | 6 | NBT : io, le, network, tag |
| `mc-rs-command` | 2 | lib + selector (très minimal vs l'actuel) |
| `mc-rs-server` | 19 | main + boucle, `connection/` splitté (combat, inventory, login, movement, plugins, portal, projectile, spawn, survival, world_tick), permissions, persistence, plugin_manager, query, **rcon** |

### Crates UNIQUES au legacy (sans équivalent dans `crates/` aujourd'hui)
C'est ici que se trouvait l'essentiel de la valeur historique — beaucoup de gameplay jamais
re-porté depuis :

- **`mc-rs-game`** (27 fichiers) — moteur de gameplay type ECS :
  - `ai/` : behavior, behaviors, brain, mob_behaviors, pathfinding, spatial, spawning, system
  - combat, enchanting, breeding, food, xp, smelting, anvil, grindstone, loom
  - inventory, recipe, components, block_entity, projectile, mob_registry, game_world
  - `bin/inspect_canonical.rs` (outil d'inspection des block states canoniques)
- **`mc-rs-world`** (22 fichiers) — monde complet :
  - générateurs : `overworld_generator`, `nether_generator`, `end_generator`, `flat_generator`, `noise`
  - registres : `block_registry`, `block_state_registry`, `item_registry`, `network_runtime_ids`, `block_hash`
  - simulation : `redstone`, `piston`, `fluid`, `gravity`, `physics`, `block_tick`
  - `chunk`, `biome`, `serializer`, `storage`, `bds_compat` (compat Bedrock Dedicated Server)
- **`mc-rs-behavior-pack`** (8 fichiers) — chargement de behavior packs : block, entity, item, loot_table, recipe, manifest, loader
- **`mc-rs-plugin-api`** (1 fichier) — API d'extension serveur
- **`mc-rs-plugin-lua`** (2 fichiers) — plugins Lua (lib + manifest)
- **`mc-rs-plugin-wasm`** (3 fichiers) — plugins WASM (host_functions, lib, manifest)

> **Note** : la richesse fonctionnelle du legacy (redstone, IA/pathfinding, enchanting, plugins
> Lua/WASM, nether/end) dépasse largement l'état actuel. Mais elle a été jugée non réutilisable
> telle quelle. Si une de ces features est reprise un jour, **on repart de PMMP**, pas de ce code.

---

## `docs_initial/` — la doc legacy

Spec de conception d'origine : **14 documents Markdown, ~6 142 lignes (228 Ko)**. C'était une
documentation « exhaustive » écrite *avant* implémentation (design top-down), correspondant à
l'architecture de `old_crates/`.

Sommaire (tel qu'il était) :

| # | Document | Sujet |
|---|---|---|
| 01 | overview | Vision, positionnement, archi haut niveau, structure workspace |
| 02 | protocol | Protocole Bedrock : types, séquence de connexion, table des paquets |
| 03 | architecture | Concurrence, boucle de tick, structure des crates, patterns |
| 04 | networking | RakNet : offline/online, fiabilité, fragmentation, compression, chiffrement |
| 05 | world | LevelDB, sub-chunks, palettes, biomes, level.dat, génération |
| 06 | entities | ECS : composants, metadata, attributs, IA, pathfinding, spawn |
| 06b | protocol-upgrade-notes | Notes de montée de version protocole |
| 07 | gameplay | Physique, combat, faim, XP, commandes, formulaires, météo |
| 08 | plugins | API Rust, WASM, Lua, Behavior Packs, événements |
| 09 | security | Auth Xbox Live, chiffrement AES, anti-triche, validation |
| 10 | performance | Optimisations réseau/chunks/entités/mémoire, benchmarks |
| 11 | rust-crates | Dépendances justifiées (mapping vers les crates legacy) |
| 12 | roadmap | Plan en 5 phases, milestones, checklists |

La doc « courante » du projet vit ailleurs : `CLAUDE.md` (racine), `docs/`, `new-docs/`.

---

## Récupération

Tout est dans git. Pour consulter (ne pas restaurer) :

```bash
git show 788b429 --stat                       # voir l'archivage
git ls-tree -r 788b429 -- old_crates           # lister les fichiers legacy
git show 788b429:old_crates/mc-rs-game/src/ai/pathfinding.rs   # lire un fichier précis
```
