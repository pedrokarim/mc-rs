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

## Comportements (executors)

| Executor | Mobs | Comportement |
|----------|------|--------------|
| `MeleeAttackExecutor` | zombie | Chasse via A\*, frappe au contact (cooldown), émet `AiEffect::Attack` |
| `BowAttackExecutor` | skeleton | Kite (recule <4, s'approche >12), vise (compensation de chute), tire `AiEffect::ShootArrow` |
| `CreeperSwellExecutor` | creeper | Chasse, s'amorce à portée 3 (fuse 30 ticks), émet `AiEffect::Explode` |
| `FlatRandomRoamExecutor` | tous | Errance aléatoire (priorité basse) quand pas de cible |
| `PanicFleeExecutor` | passifs | Fuit en ligne droite quand blessé + joueur proche |

### Assemblage par espèce (`species.rs`)

| Espèce | Sensors | Behaviors (priorité) | Controllers |
|--------|---------|----------------------|-------------|
| Zombie | NearestPlayer(16) | Melee(4), Roam(1) | Walk, Look(cible+route) |
| Skeleton | NearestPlayer(16) | Bow(4), Roam(1) | Walk, Look(cible+route) |
| Creeper | NearestPlayer(16) | Swell(4), Roam(1) | Walk, Look(cible+route) |
| Passifs | NearestPlayer(16) | Flee(4), Roam(1) | Walk, Look(route) |

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

- Vitesses mob : ~0.2–0.25 blocs/tick (`MobKind::movement_speed`).
- Cooldown mêlée / fuse creeper / cooldown arc : 30 ticks (~1.5 s à 20 TPS).
- Creeper : portée d'amorçage 3, rayon d'explosion 3 (`ExplosionSource::Creeper`).
- Arc : recul <4, tir 4–12, vitesse de flèche 1.4 b/tick, dégâts 3.

## Follow-ups non faits

- Son/particule d'explosion & d'impact de flèche (pas de `LevelEvent` explosion dans le proto —
  format binaire à ne pas deviner).
- Dégâts par difficulté (easy/normal/hard) ; actuellement valeurs « normal ».
- Lissage de chemin (Floyd) ; flèche ramassable ; strafe latéral du squelette.
- Unification des deux enums `MobKind` (`mob_ai` 45 espèces registres vs `mob_entities` 7 vivantes).

Voir [`99-ROADMAP.md`](99-ROADMAP.md) et [`07-ENTITY-SYSTEM.md`](07-ENTITY-SYSTEM.md).
