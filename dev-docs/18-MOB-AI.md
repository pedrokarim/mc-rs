# 18 - Mob AI (framework de behaviors)

## Contexte & référence

Les mobs spawnaient et tombaient (gravité) mais **restaient inertes** : tout le code d'IA
historique (`mob_ai::MobAi`, `ai_goals`, `ai_states`, `pathfinder`) était écrit mais **jamais
branché** dans la boucle de jeu.

**PocketMine-MP n'implémente pas d'IA de mobs.** La référence canonique pour ce système est donc
**Allay** (`.reference/Allay/.../entity/ai/`), un serveur Bedrock en Java qui possède un vrai
framework d'IA. L'architecture ci-dessous en est un port idiomatique Rust.

Code : `crates/mc-rs-server/src/ai/`. Opère sur les entités vivantes
[`crate::mob_entities::MobEntity`] (7 espèces : zombie, skeleton, creeper, cow, pig, sheep, chicken).

## Arborescence

```
ai/
├── mod.rs          — BehaviorGroup (orchestrateur) + AiComponent + PlayerSnapshot + AiEffect + ctx
├── behavior.rs     — trait Behavior/Evaluator/Executor, BehaviorState, évaluateurs réutilisables
├── memory.rs       — Memory : état IA typé par champs (sorties sensors, cibles, état de route)
├── sensor.rs       — NearestPlayerSensor
├── controller.rs   — WalkController (déplacement + saut) + LookController (yaw/pitch)
├── route.rs        — A* sol (FlatAStarRouteFinder) + walkable_ground
├── executor.rs     — Melee, Roam, PanicFlee, CreeperSwell, BowAttack
└── species.rs      — build_behavior_group(kind) : assemblage par espèce
```

## Pipeline (port `BehaviorGroupImpl.tick`)

Chaque mob tick (au **game tick = 20 TPS**), `BehaviorGroup::tick` exécute, dans l'ordre :

```
sensors → eval core → eval normaux (priorité) → exec running → update_route → controllers
```

1. **Sensors** — échantillonnent le monde (joueurs proches) et écrivent dans la `Memory`.
2. **Eval core** — behaviors indépendants de la priorité (tous ceux qui évaluent vrai tournent).
3. **Eval normaux** — sélection par **priorité** : seuls les behaviors de plus haute priorité
   évalués vrai démarrent ; un behavior actif n'est interrompu que par une priorité strictement
   supérieure.
4. **Exec** — `execute()` de chaque behavior actif ; `false` → `on_stop` et arrêt.
5. **update_route** — recalcule l'A\* (périodiquement ou si la cible change de bloc) et fait avancer
   le mob de waypoint en waypoint.
6. **Controllers** — traduisent la `Memory` en mouvement/rotation réels.

### Composants

| Composant | Rôle | Réf Allay |
|-----------|------|-----------|
| `Behavior` | evaluator + executor + `priority` + `period` | `Behavior` |
| `Evaluator` | « ce behavior est-il lançable ? » (lecture seule) | `BehaviorEvaluator` |
| `Executor` | logique par tick (`on_start`/`execute`/`on_stop`/`on_interrupt`) | `BehaviorExecutor` |
| `Sensor` | capte l'environnement → `Memory` | `Sensor` |
| `Controller` | `Memory` → mouvement/rotation | `Controller` |
| `Memory` | état IA typé par champs (vs `MemoryType<T>`) | `MemoryStorage` |
| `BehaviorGroup` | orchestrateur du pipeline | `BehaviorGroupImpl` |

### Mémoire (`Memory`)

Champs typés : `nearest_player`, `move_target`, `look_target`, `movement_speed`, + l'état de route
(`route`, `node_index`, `route_update_tick/required`, `move_dir_start/end`, `should_update_move_dir`).

## Navigation A\* (`route.rs`)

Port de `FlatAStarRouteFinder` + `GroundPosEvaluator` :
- A\* **2D sur XZ**, 8 voisins (cardinaux + diagonales) avec **anti-corner-cutting**.
- Franchit **une marche** (+1) et **chute** jusqu'à `max_fall` (3) blocs.
- Heuristique **octile** (admissible, consistante en 8 directions).
- Si la cible exacte est inatteignable → **chemin partiel** vers le nœud le plus proche.
- Prédicat `walkable_ground(cache, x, y, z)` : sol solide en `y-1` + pieds/tête libres
  (réutilise `mob_entities::is_supporting_block`).

`update_route` recalcule tous les `ROUTE_UPDATE_CYCLE` (20) ticks ou quand la cible change de bloc /
la route est épuisée, puis fait suivre les waypoints au `WalkController`.

## Physique & collision (`mob_entities.rs`)

Le tick physique (après l'IA) intègre la vélocité posée par le `WalkController` :
- **Collision horizontale axe par axe** (glissement le long des murs) + **step-up** auto d'1 bloc.
- Collision verticale au sol ; `on_ground` n'est marqué **qu'en descente** (sinon les sauts seraient
  annulés). `EntityBase.on_ground` est lu par le `WalkController` pour décider de sauter.
- Friction horizontale (sol/air) pour stopper le résidu quand le mob n'est plus poussé.

## Inventaire exhaustif des comportements

> Source de vérité : le code (`ai/executor.rs`, `ai/species.rs`, `mob_entities.rs`).
> Mis à jour le 2026-07-02. Total : **12 executors d'IA**, **11 profils**, **1 sensor**,
> **3 controllers**, **~12 comportements de cycle de vie / physique**.

### A. Comportements d'IA — 12 executors (`ai/executor.rs`)

| # | Executor | Comportement |
|---|----------|--------------|
| 1 | `MeleeAttackExecutor` | Traque via A\*, frappe au contact (cooldown), émet `AiEffect::Attack` |
| 2 | `BowAttackExecutor` | Kite (recule <4, s'approche >12), vise (compensation de chute), tire une flèche OU un projectile (`with_projectile` : potion sorcière, wind charge breeze, bullet à tête chercheuse shulker) |
| 3 | `CreeperSwellExecutor` | Traque, s'amorce à portée 3 (fuse 30 ticks), émet `AiEffect::Explode` |
| 4 | `FlatRandomRoamExecutor` | Errance aléatoire au sol ; gate d'intervalle (1/120 tick, vanilla `random_stroll`) → pause entre balades ; regarde la destination |
| 5 | `PanicFleeExecutor` | Fuit en ligne droite quand blessé + joueur proche |
| 6 | `TemptExecutor` | Suit le joueur tenant sa nourriture (`tempting_items`) |
| 7 | `FlyRoamExecutor` | Errance en vol 3D (pas de gravité) |
| 8 | `FlyShootExecutor` | Vol stationnaire + tir de boules de feu (ghast/blaze/wither) — `AiEffect::ShootFireball` |
| 9 | `FlyAttackExecutor` | Vole vers le joueur et frappe au contact (phantom/vex) |
| 10 | `CreakingExecutor` | Se fige tant qu'un joueur le regarde (cône de produit scalaire via `look_dir`), sinon fonce |
| 11 | `DragonExecutor` | Phases de l'ender dragon (cercle ↔ strafe + souffle) |
| 12 | `GuardianLaserExecutor` | Se tient à distance, charge un rayon, frappe (aquatique, nage 3D) |

### B. Profils d'IA — 11 (`AiProfile`, assemblés dans `species.rs`)

`Melee` · `Bow` · `CreeperSwell` · `Passive` · `Neutral` · `Flying` · `FlyingShooter` ·
`FlyingMelee` · `Creaking` · `Dragon` · `GuardianLaser`.

Chaque profil combine sensor + behaviors priorisés + controllers. Priorités (convention Allay,
grand = prioritaire ; ordre sémantique = vanilla) : Combat/Panic **4** > Tempt **3** > Roam **1**.
Vitesses recalées sur bedrock-samples (`speed_multiplier`) : stroll ×1.0, panic ×1.25, tempt ×1.25,
melee ×1.0, via `beh_speed(attribut × SPEED_SCALE × mult)`.

### C. Framework de support

- **Sensors (1)** : `NearestPlayerSensor` (portée par espèce + raycast ligne de vue).
- **Controllers (3)** : `WalkController` (marche + saut + step-up), `FlyController` (vol 3D),
  `LookController` (yaw corps / yaw tête / pitch vers cible ou route).

### D. Comportements de cycle de vie / physique — ~12 (`mob_entities.rs` tick, hors executors)

Reproduction (nourrissage → mode amour → bébé, cooldown) · croissance bébé→adulte ·
téléportation de l'enderman blessé · brûlure au soleil (zombie/skeleton) · dégâts de chute ·
flottaison/nage (aquatiques) · split des slimes/magma cubes à la mort · despawn des hostiles
(distance) · mouton broute l'herbe + repousse la laine · tonte (shear) · knockback · flag `NO_AI`
(coupe l'IA client).

## Correspondance avec le vanilla (`minecraft:behavior.*`, bedrock-samples)

Vanilla a **~30+ composants** `minecraft:behavior.*`. Couverture actuelle :

| Composant vanilla | Chez nous | État |
|-------------------|-----------|------|
| `random_stroll` | `FlatRandomRoamExecutor` | ✅ |
| `melee_attack` / `melee_box_attack` | `MeleeAttackExecutor` | ✅ |
| `ranged_attack` | `BowAttackExecutor` | ✅ |
| `swell` (creeper) | `CreeperSwellExecutor` | ✅ |
| `panic` | `PanicFleeExecutor` | ✅ |
| `tempt` | `TemptExecutor` | ✅ |
| `nearest_attackable_target` | `NearestPlayerSensor` (joueur uniquement) | ⚠️ partiel (pas les autres entités) |
| `float` / `float_wander` | flottaison en physique / `FlyRoamExecutor` | ⚠️ pas un behavior propre |
| `breed` | repro dans le tick manager (pas de pathfinding vers le partenaire) | ⚠️ partiel |
| `follow_parent` | — | ❌ (les bébés ne suivent pas) |
| `look_at_player` / `random_look_around` | `LookController` (fixe le joueur proche) | ⚠️ pas de behavior dédié |
| `eat_block` | mouton mange l'herbe (tick manager) | ⚠️ pas un behavior propre |
| `avoid_mob_type` | — | ❌ (creeper ne fuit pas les chats) |
| `hurt_by_target` | — | ❌ (pas de riposte quand frappé sans être vu) |
| `leap_at_target` | — | ❌ (araignée ne saute pas) |
| `mount_pathing` / `controlled_by_player` | — | ❌ (montures) |
| `equip_item` / `pickup_items` | — | ❌ |
| `flee_sun` | brûlure gérée en physique | ⚠️ ne fuit pas le soleil |
| `stomp_turtle_egg` | — | ❌ |

## Intégration dans la game loop (`AiEffect`)

Un behavior ne peut pas toucher d'autres entités (le `connections: HashMap` vit dans `main.rs` ;
contrainte du borrow checker). Comme les **`syncedActions` d'Allay**, les actions inter-entités sont
émises comme `AiEffect` dans `TickResult.attack_requests`, puis **drainées par `main.rs`** :

| `AiEffect` | Handler `main.rs` | Effet |
|------------|-------------------|-------|
| `Attack` | `apply_mob_attack_to_player` | `combat::attack_entity` + hurt/knockback/respawn (calque PvP) |
| `Explode` | `apply_mob_explosion` | `explosion::Explosion` → destruction de blocs (UpdateBlock) + dégâts radiaux + retrait du creeper |
| `ShootArrow` | spawn `arrow_entity` | flèche vivante tickée chaque game tick |

`MobEntityManager` stocke l'IA dans une **map parallèle** `HashMap<u64, AiComponent>` pour que
`MobEntity` reste `Clone` (les behaviors sont des trait objects non clonables).

### Entité flèche (`arrow_entity.rs`)

Projectile vivant (≠ du module dormant `arrow.rs` qui n'est qu'un modèle de données) :
physique balistique (gravité/drag), collision bloc (se plante), détection de hit
**segment-vs-joueur** (anti-tunneling), despawn par vie/hit/bloc. Les hits sont appliqués via le
même chemin combat que les attaques mêlée.

## Constantes de réglage (à affiner en jeu)

- Vitesses : `beh_speed(attribut × SPEED_SCALE × multiplicateur)` — multiplicateurs recalés sur
  bedrock-samples (`speed_multiplier`). `SPEED_SCALE` (`species.rs`) = bouton de calibrage global.
- Errance : gate d'intervalle `ROAM_INTERVAL = 120` (1/120 par tick, vanilla `random_stroll`),
  rayon `ROAM_RANGE = 10` (vanilla `xz_dist`).
- Cooldown mêlée 20 ticks ; fuse creeper 30 ticks ; cooldown arc 30 ticks.
- Arc : recul <4, tir 4–12, vitesse de flèche 1.4 b/tick, dégâts 3.
- Client : flag `NO_AI` posé sur tous les mobs (coupe l'IA/prédiction côté client — cf PMMP
  « client-side AI interference »).

## Follow-ups non faits

- **Behaviors vanilla manquants** : voir le tableau « Correspondance avec le vanilla » ci-dessus
  (`breed`/`follow_parent`, `look_at_player` dédié, `avoid_mob_type`, `hurt_by_target`,
  `leap_at_target`, `equip_item`/`pickup_items`, `flee_sun`, `stomp_turtle_egg`, montures…).
- Son/particule d'explosion & d'impact de flèche (pas de `LevelEvent` explosion dans le proto).
- Lissage de chemin (Floyd) ; flèche ramassable ; strafe latéral du squelette.
- Calibrage in-game de `SPEED_SCALE` (jamais validé manette en main).

Voir [`99-ROADMAP.md`](99-ROADMAP.md) et [`07-ENTITY-SYSTEM.md`](07-ENTITY-SYSTEM.md).
