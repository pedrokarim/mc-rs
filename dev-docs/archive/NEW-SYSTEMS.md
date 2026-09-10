# Nouveaux systèmes — état après le grand porting PMMP (2026-04-14)

Fait pendant la session "exécution blocs 1→8" après abandon provisoire du crash client inventaire. Tous les modules portés depuis `.reference/PocketMine-MP/`.

---

## Bloc 1 — Event system (`src/event/`)

Port de `src/event/*` PMMP.

| PMMP | mc-rs |
|---|---|
| `Event.php` | `event::Event` trait (Any + Send + 'static) |
| `Cancellable.php` + `CancellableTrait` | `event::Cancellable` trait + macro `cancellable_event!` |
| `EventPriority.php` (LOWEST..MONITOR) | `event::EventPriority` enum |
| `HandlerList.php` + `HandlerListManager.php` | `event::EventManager` (HashMap<TypeId, Vec<RegisteredListener>>) |
| `RegisteredListener.php` | interne à `EventManager` : priority + handle_cancelled + callback closure |

**Events fournis** :
- `event::player::{PlayerJoinEvent, PlayerQuitEvent, PlayerChatEvent, PlayerMoveEvent, PlayerDeathEvent, PlayerRespawnEvent, PlayerGameModeChangeEvent, PlayerInteractEvent, PlayerDropItemEvent, PlayerItemHeldEvent, PlayerExperienceChangeEvent}`
- `event::block::{BlockBreakEvent, BlockPlaceEvent, BlockUpdateEvent, BlockGrowEvent}`
- `event::entity::{EntityDamageEvent, EntityDeathEvent, EntitySpawnEvent, EntityDespawnEvent, EntityRegainHealthEvent, EntityMotionEvent}`
- `event::server::{ServerStartEvent, ServerCommandEvent, PlayerCommandPreprocessEvent, DataPacketSendEvent, DataPacketReceiveEvent}`

**Intégration** : `event_manager: Arc<Mutex<EventManager>>` partagé via `Connection::events`. Firing in-game : join/quit/chat/block-break/block-place.

---

## Bloc 2 — Attribute system (`src/attribute.rs`)

Port de `src/entity/Attribute.php` + `AttributeFactory.php` + `AttributeMap.php` + `HungerManager.php` + `ExperienceManager.php`.

- `Attribute` : id, min_value, max_value, default_value, current_value, should_send, desynchronized.
- `AttributeMap::default_for_player()` : initialise tous les attrs PMMP Human.
- `AttributeMap::drain_desync()` : retourne les attrs modifiés + marque synced. Sert à batch les `UpdateAttributesPacket`.
- `HungerManager::exhaust()` / `tick()` : saturation drain → hunger drain → regen santé ou starvation damage selon difficulty.
- `ExperienceManager::xp_to_next_level(level)` : formule PMMP (2*L+7 / 5*L-38 / 9*L-158). Tests vérifiés.
- `ExperienceManager::add_xp()` / `remove_xp()` : redistribue le niveau + progress.

**Intégration** : `attributes: AttributeMap` + `hunger: HungerManager` dans `Connection`. Spawn : `drain_desync()` → `UpdateAttributesPacket` envoyé. Game tick (20 TPS) : `hunger.tick()` + `drain_desync()` pour repush les changements.

---

## Bloc 3 — Combat / Living (`src/combat.rs` + `src/combat_packets.rs`)

Port de `src/entity/Living.php::attack` + `knockBack` + `applyDamageModifiers` + i-frames.

- `combat::CombatState { attack_time, no_damage_ticks, last_damage_cause_base }`.
- `combat::attack_entity(events, target_id, pos, attrs, state, cause, base_damage, attacker_id, attacker_pos, kb_force)` :
  1. Check i-frames (skip si `no_damage_ticks > 0`) sauf `CAUSE_SUICIDE`.
  2. `applyDamageModifiers` : si coup précédent ≥ base, cancel ; sinon final = base - prev.
  3. Fire `EntityDamageEvent` (plugins peuvent cancel ou muter final_damage).
  4. Applique dégât → HEALTH attribute.
  5. Calcule knockback depuis vecteur attaquant→target et KNOCKBACK_RESISTANCE.
  6. Fire `EntityDeathEvent` si HP ≤ 0.
- `combat::heal_entity(attrs, amount)` : heal cappé au max HP.
- `combat_packets::{hurt_animation, death_animation, encode_respawn, encode_set_actor_motion}`.

**Intégration PvP** : in `main.rs` après `pending_entity_attacks` :
1. Si target = autre joueur → `attack_entity` avec base damage = tier.base_attack_points (1 pour main nue).
2. Broadcast hurt animation aux viewers.
3. SetActorMotion (knockback) envoyé à la target avec tick field 944.
4. Si mort : death animation broadcast + Respawn packet envoyé + HP/hunger reset + teleport spawn.

---

## Bloc 4 — Mobs passifs (`src/passive_entities.rs`)

Ports sélectifs de `src/entity/object/*`.

- `PrimedTntEntity` : fuse 80 ticks, tick() décrémente + retourne `Some(pos)` à l'explosion.
- `FallingBlockEntity` : gravity + collision sol via closure `is_block_at`.
- `ExperienceOrbEntity` : attraction joueur dans range 8.0, pickup range 1.425 ; `tick()` retourne `OrbTickResult::{Live, Pickup(player_id, xp), Despawn}`.
- `PassiveEntityManager` : HashMap<runtime_id, ...> pour les 3 types, spawn/remove.

**Intégration** : `_passive_entities` instancié dans `main.rs` (dead_code ok) — spawn via commandes / events laissé à faire.

---

## Bloc 5 — Durabilité outils (`src/durability.rs`)

Port de `src/item/Durable.php` + `TieredTool.php`.

- `ToolTier::{Wood, Stone, Gold, Iron, Diamond, Netherite}` avec `max_durability()`, `base_attack_points()`, `mining_speed()` alignées PMMP.
- `ToolType::{Pickaxe, Axe, Shovel, Hoe, Sword, Shears, Armor}`.
- `durable_info(item_network_id)` : lookup sur la liste vanilla (30 outils mappés) via `item_registry::required_item_id`.
- `apply_damage(stack, amount)` : stocke damage dans `stack.meta` (convention PMMP), retourne `true` si cassé.

**Intégration** : dans `movement.rs` après BlockBreakEvent, si held item is durable (non créatif), `apply_damage(held, 1)` ; si cassé → `manager.set_slot(Main, slot, AIR)`.

---

## Bloc 6 — Scheduler (`src/scheduler.rs`)

Port conceptuel de `src/scheduler/Scheduler.php`.

- `Scheduler::{after(delay, cb), repeat(interval, cb), delayed_repeat(delay, interval, cb), cancel(id)}`.
- Min-heap sur `fire_at_tick`, ticked via `Scheduler::tick()`.
- 4 tests unitaires (after, repeat, cancel, delayed_repeat).

**Partial** : bindings Lua non faits. Le scheduler Rust natif est utilisable par le core serveur mais les plugins Lua n'ont pas encore accès. Ajouter `schedule_after` en Lua nécessite : `PluginAction::ScheduleAfter { delay, callback_key: RegistryKey }` + drain dans main loop + invoquer `lua.registry_value(key)` à l'échéance. ~100 lignes à faire plus tard.

**Events Lua également deferred** : pour exposer les events typés Rust aux plugins Lua, il faut un convertisseur Rust→Lua table par event type, un registre string→TypeId, et un dispatch cross-thread. ~300 lignes.

---

## Bloc 7 — Visuels (`src/visuals.rs`)

Ports de `SpawnParticleEffectPacket` + `BossEventPacket`.

- `SpawnParticleEffect::at(pos, name)` → 1 paquet prêt à envoyer.
- `boss_show(boss_id, title, hp_pct, color)` / `boss_hide` / `boss_update_health` / `boss_update_title`.
- Constantes `boss_color::{PINK..WHITE}`, `boss_event_type::{SHOW..QUERY}`.

**Intégration** : packets prêts, pas encore d'API serveur high-level. Un plugin peut les appeler directement via `encode_compressed_packet(SPAWN_PARTICLE_EFFECT, &bytes)`.

---

## Bloc 8 — Intégration globale (Connection + main tick loop)

- `Connection::events: Arc<Mutex<EventManager>>` (passé au constructeur depuis main).
- `Connection::{attributes, combat, hunger, game_tick_accum}`.
- `Connection::tick_game_state()` : combat.tick() + hunger.tick() + drain_desync() → UpdateAttributesPacket.
- Main loop : à chaque server tick (100 TPS), incrémente `game_tick_accum` ; si ≥ 5, appelle `tick_game_state` (= 20 TPS game tick).
- `process_peer_events` étendu avec `event_manager: &Arc<Mutex<EventManager>>` pour fire events depuis le packet handling.
- Event firings : `PlayerJoinEvent` au join, `PlayerQuitEvent` au quit, `PlayerChatEvent` (cancellable, éditable) au chat, `BlockBreakEvent` et `BlockPlaceEvent` dans movement.
- PvP full : attack_entity + hurt animation broadcast + SetActorMotion (knockback) + Respawn packet à la mort.

---

## Pas dans ce bloc (= future work)

- **Inventaire** : le crash client E-key persiste malgré byte-perfect encoding selon gophertunnel 944. Suspicion : un autre paquet spawn met le client en état instable. À reprendre avec un sniffer packet-level (Wireshark / proxy MitM).
- **Lua bindings** pour le scheduler et les events : pure Rust done, Lua à coller.
- **Block entities** (chest, furnace, sign, bed) : bloqué par le crash inventaire.
- **Crafting / Enchanting** : idem.
- **Mobs IA** : base entity présente, behavior tree/IA pas porté.

---

## Compteurs

- **Tests** : 213 passent, 0 failed.
- **Modules ajoutés** : event/, attribute, combat, combat_packets, durability, passive_entities, visuals, scheduler.
- **Commits checkpoint** : 4 durant la session (inventory + foundation + passive+visuals+integration + PvP).
