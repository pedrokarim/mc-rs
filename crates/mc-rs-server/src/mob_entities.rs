use std::collections::HashMap;

use mc_rs_proto::packets::player::{ItemStack, PlayerAttribute, UpdateAttributes};

use crate::ai::{AiComponent, AiEffect, PlayerSnapshot};
use crate::entity::{health_attributes, living_metadata, EntityBase};
use crate::item_registry;
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;

const SERVER_TICKS_PER_SECOND: f32 = 100.0;
const BASELINE_TICKS_PER_SECOND: f32 = 20.0;
const GRAVITY_PER_TICK: f32 = 0.08 * (BASELINE_TICKS_PER_SECOND / SERVER_TICKS_PER_SECOND);
const AIR_DRAG: f32 = 0.996;
const MAX_FALL_SPEED: f32 = -0.784;
/// Friction horizontale appliquée chaque tick (le résidu décroît quand le mob
/// n'est plus poussé par le `WalkController`). Sol plus adhérent que l'air.
const GROUND_FRICTION: f32 = 0.6;
const AIR_FRICTION: f32 = 0.91;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobKind {
    Zombie,
    Skeleton,
    Creeper,
    Cow,
    Pig,
    Sheep,
    Chicken,
}

impl MobKind {
    pub fn parse(name: &str) -> Option<Self> {
        let normalized = name.trim().to_ascii_lowercase();
        let normalized = normalized.strip_prefix("minecraft:").unwrap_or(&normalized);
        match normalized {
            "zombie" => Some(Self::Zombie),
            "skeleton" => Some(Self::Skeleton),
            "creeper" => Some(Self::Creeper),
            "cow" => Some(Self::Cow),
            "pig" => Some(Self::Pig),
            "sheep" => Some(Self::Sheep),
            "chicken" => Some(Self::Chicken),
            _ => None,
        }
    }

    pub fn actor_type(self) -> &'static str {
        match self {
            Self::Zombie => "minecraft:zombie",
            Self::Skeleton => "minecraft:skeleton",
            Self::Creeper => "minecraft:creeper",
            Self::Cow => "minecraft:cow",
            Self::Pig => "minecraft:pig",
            Self::Sheep => "minecraft:sheep",
            Self::Chicken => "minecraft:chicken",
        }
    }

    pub fn selector_type(self) -> &'static str {
        match self {
            Self::Zombie => "zombie",
            Self::Skeleton => "skeleton",
            Self::Creeper => "creeper",
            Self::Cow => "cow",
            Self::Pig => "pig",
            Self::Sheep => "sheep",
            Self::Chicken => "chicken",
        }
    }

    /// Liste des noms d'entités supportées par `/summon` — alimente la
    /// SoftEnum d'autocomplétion côté client.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "zombie", "skeleton", "creeper", "cow", "pig", "sheep", "chicken",
        ]
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Zombie => "Zombie",
            Self::Skeleton => "Skeleton",
            Self::Creeper => "Creeper",
            Self::Cow => "Cow",
            Self::Pig => "Pig",
            Self::Sheep => "Sheep",
            Self::Chicken => "Chicken",
        }
    }

    pub fn size(self) -> (f32, f32) {
        match self {
            Self::Zombie | Self::Skeleton | Self::Creeper => (0.6, 1.9),
            Self::Cow | Self::Pig | Self::Sheep => (0.9, 1.3),
            Self::Chicken => (0.4, 0.7),
        }
    }

    /// Mob hostile (traque et attaque le joueur) ?
    pub fn is_hostile(self) -> bool {
        matches!(self, Self::Zombie | Self::Skeleton | Self::Creeper)
    }

    /// Distance de détection d'un joueur (blocs). Réf valeurs vanilla.
    pub fn sight_range(self) -> f64 {
        16.0
    }

    /// Portée d'attaque mêlée (blocs). Squelette/creeper traités en mêlée pour
    /// la v1 (arc/explosion = follow-ups documentés).
    pub fn attack_range(self) -> f64 {
        2.0
    }

    /// Dégâts d'attaque mêlée de base (difficulté normale). Réf PMMP/vanilla.
    pub fn attack_damage(self) -> f32 {
        match self {
            Self::Zombie => 3.0,
            Self::Skeleton => 2.0,
            Self::Creeper => 0.0, // dégâts via explosion (non implémentée ici)
            _ => 0.0,
        }
    }

    /// Items qui attirent ce mob passif (tempt — suivre le joueur qui les tient).
    /// Réf vanilla Bedrock (cf. `breeding_items`).
    pub fn tempting_items(self) -> &'static [&'static str] {
        match self {
            Self::Cow | Self::Sheep => &["minecraft:wheat"],
            Self::Pig => &["minecraft:carrot", "minecraft:potato", "minecraft:beetroot"],
            Self::Chicken => &[
                "minecraft:wheat_seeds",
                "minecraft:beetroot_seeds",
                "minecraft:melon_seeds",
                "minecraft:pumpkin_seeds",
            ],
            _ => &[],
        }
    }

    /// Vitesse de déplacement (blocs/tick) utilisée par le `WalkController`.
    /// Valeurs vanilla approximatives (à ajuster en jeu).
    pub fn movement_speed(self) -> f32 {
        match self {
            Self::Zombie => 0.23,
            Self::Skeleton => 0.25,
            Self::Creeper => 0.25,
            Self::Cow | Self::Pig | Self::Sheep => 0.2,
            Self::Chicken => 0.25,
        }
    }

    pub fn max_health(self) -> f32 {
        // Source autoritaire : `mob_hp::mob_max_hp` (couvre ~60 entités vanilla).
        crate::mob_hp::mob_max_hp(self.actor_type())
    }

    /// Mob qui peut se reproduire (animaux passifs avec une nourriture).
    pub fn is_breedable(self) -> bool {
        matches!(self, Self::Cow | Self::Pig | Self::Sheep | Self::Chicken)
    }

    /// Cet item (network id) met-il ce mob en mode amour ? (même set que `tempting_items`.)
    pub fn is_breeding_food(self, item_id: i32) -> bool {
        self.tempting_items()
            .iter()
            .filter_map(|n| item_registry::network_id(n))
            .any(|id| id == item_id)
    }

    pub fn default_loot(self) -> Vec<ItemStack> {
        fn item(name: &str, count: u16) -> ItemStack {
            ItemStack::new(item_registry::required_item_id(name), count, 0)
        }

        match self {
            Self::Zombie => vec![item("minecraft:rotten_flesh", 1)],
            Self::Skeleton => vec![item("minecraft:bone", 1), item("minecraft:arrow", 1)],
            Self::Creeper => vec![item("minecraft:gunpowder", 1)],
            Self::Cow => vec![item("minecraft:beef", 1), item("minecraft:leather", 1)],
            Self::Pig => vec![item("minecraft:porkchop", 1)],
            Self::Sheep => vec![
                ItemStack::new(
                    item_registry::required_item_id("minecraft:white_wool"),
                    1,
                    0,
                ),
                item("minecraft:mutton", 1),
            ],
            Self::Chicken => vec![item("minecraft:chicken", 1), item("minecraft:feather", 1)],
        }
    }
}

#[derive(Clone)]
pub struct MobEntity {
    pub base: EntityBase,
    pub kind: MobKind,
    /// Ticks passés loin de tout joueur (despawn des hostiles — voir tick).
    despawn_timer: u32,
    /// Compteur pour les dégâts de feu (sun-burning des zombies/skeletons).
    fire_tick: u32,
    /// Distance de chute accumulée (dégâts de chute à l'atterrissage).
    fall_distance: f32,
    /// Ticks restants en « mode amour » (reproduction) ; 0 = pas en amour.
    in_love: u32,
    /// Ticks restants avant de pouvoir se reproduire à nouveau.
    breed_cooldown: u32,
    /// Le mob est un bébé.
    baby: bool,
    /// Ticks écoulés depuis la naissance (croissance bébé → adulte).
    baby_age: u32,
    /// Mouton tondu (sans laine) — repousse en mangeant de l'herbe.
    sheared: bool,
    /// Ticks cumulés passés sur un bloc d'herbe (mouton tondu qui mange).
    eat_grass_timer: u32,
}

impl MobEntity {
    pub fn spawn(kind: MobKind, position: [f32; 3]) -> Self {
        let (width, height) = kind.size();
        let base = EntityBase::new(
            kind.actor_type(),
            kind.selector_type(),
            kind.display_name(),
            position,
            health_attributes(kind.max_health()),
            living_metadata(width, height, None),
        );
        Self {
            base,
            kind,
            despawn_timer: 0,
            fire_tick: 0,
            fall_distance: 0.0,
            in_love: 0,
            breed_cooldown: 0,
            baby: false,
            baby_age: 0,
            sheared: false,
            eat_grass_timer: 0,
        }
    }

    /// Spawn d'un bébé (issu de la reproduction) : flag `BABY` + échelle réduite.
    pub fn spawn_baby(kind: MobKind, position: [f32; 3]) -> Self {
        use mc_rs_proto::packets::player::entity_flags;
        let mut mob = Self::spawn(kind, position);
        mob.baby = true;
        mob.base.set_entity_flag(entity_flags::BABY, true);
        mob.base.set_scale(0.5);
        mob
    }

    pub fn add_actor_packet(&self) -> Vec<u8> {
        self.base.add_actor_packet()
    }

    pub fn remove_packet(&self) -> Vec<u8> {
        self.base.remove_packet()
    }
}

pub struct MovementUpdate {
    pub entity_unique_id: i64,
    pub entity_position: [f32; 3],
    pub add_packet: Vec<u8>,
    pub move_packet: Vec<u8>,
    pub motion_packet: Vec<u8>,
}

pub struct DamageResult {
    pub update_attributes_packet: Option<Vec<u8>>,
    pub remove_packet: Option<Vec<u8>>,
    pub death_position: Option<[f32; 3]>,
    pub drops: Vec<ItemStack>,
}

pub struct TickResult {
    pub movement_updates: Vec<MovementUpdate>,
    /// Attaques mob→joueur à appliquer par `main.rs` (= `syncedActions` Allay).
    pub attack_requests: Vec<AiEffect>,
    /// Mobs hostiles despawnés (trop loin des joueurs) → `main.rs` broadcast Remove.
    pub despawned: Vec<MobEntity>,
    /// SetActorData à diffuser (changement de flag, ex ONFIRE) : (unique_id, bytes).
    pub metadata_updates: Vec<(i64, Vec<u8>)>,
    /// Dégâts environnementaux à appliquer (feu, chute) : (runtime_id, montant).
    pub damage_requests: Vec<(u64, f32)>,
    /// Changements de blocs (mouton qui mange l'herbe) : (position, runtime_id).
    pub block_changes: Vec<([i32; 3], u32)>,
}

/// Distances de despawn des hostiles (port des règles Bedrock).
const DESPAWN_INSTANT_DIST_SQ: f64 = 128.0 * 128.0;
const DESPAWN_FAR_DIST_SQ: f64 = 32.0 * 32.0;
/// Ticks loin (>32 blocs) avant despawn (~30 s à 20 TPS).
const DESPAWN_TIMER_MAX: u32 = 600;
/// Poussée d'Archimède par tick dans l'eau (le mob remonte) + vitesse max de montée.
const WATER_BUOYANCY: f32 = 0.05;
const WATER_MAX_RISE: f32 = 0.1;
/// Hauteur de chute (blocs) sans dégât ; au-delà : 1 dégât par bloc supplémentaire.
const FALL_DAMAGE_THRESHOLD: f32 = 3.0;
/// Reproduction (ticks 20 TPS) : durée du mode amour (~30 s), cooldown après
/// reproduction (~5 min), croissance bébé → adulte (~20 min). Distance d'appariement.
const LOVE_TICKS: u32 = 600;
const BREED_COOLDOWN: u32 = 6000;
const BABY_GROW_TICKS: u32 = 24000;
const BREED_RANGE_SQ: f64 = 9.0;

pub struct MobEntityManager {
    entities: HashMap<u64, MobEntity>,
    /// Composant IA par mob (runtime id) — stocké en parallèle pour garder
    /// `MobEntity` `Clone` (les behaviors sont des trait objects non-clonables).
    ai: HashMap<u64, AiComponent>,
}

impl Default for MobEntityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MobEntityManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            ai: HashMap::new(),
        }
    }

    /// Insère un mob déjà construit + son composant IA.
    fn insert_with_ai(&mut self, entity: MobEntity) {
        let id = entity.base.entity_runtime_id;
        let kind = entity.kind;
        self.entities.insert(id, entity);
        self.ai.insert(
            id,
            AiComponent::new(crate::ai::species::build_behavior_group(kind), kind.movement_speed()),
        );
    }

    pub fn spawn(&mut self, kind: MobKind, position: [f32; 3]) -> MobEntity {
        let entity = MobEntity::spawn(kind, position);
        self.insert_with_ai(entity.clone());
        entity
    }

    /// Spawn d'un bébé (flag BABY + échelle réduite). Utilisé par le spawner
    /// naturel (~5 % des animaux) et la reproduction.
    pub fn spawn_baby(&mut self, kind: MobKind, position: [f32; 3]) -> MobEntity {
        let entity = MobEntity::spawn_baby(kind, position);
        self.insert_with_ai(entity.clone());
        entity
    }

    pub fn remove(&mut self, entity_runtime_id: u64) -> Option<MobEntity> {
        self.ai.remove(&entity_runtime_id);
        self.entities.remove(&entity_runtime_id)
    }

    /// Nourrit un mob : s'il est reproductible, adulte, hors cooldown et que
    /// l'item correspond à sa nourriture → le met en mode amour. Renvoie `true`
    /// si la nourriture a été acceptée (à consommer côté joueur).
    pub fn feed_mob(&mut self, runtime_id: u64, item_id: i32) -> bool {
        if let Some(e) = self.entities.get_mut(&runtime_id) {
            if e.kind.is_breedable()
                && !e.baby
                && e.breed_cooldown == 0
                && e.kind.is_breeding_food(item_id)
            {
                e.in_love = LOVE_TICKS;
                return true;
            }
        }
        false
    }

    /// Tond un mouton non tondu : pose le flag SHEARED et renvoie `(laine,
    /// SetActorData)` à diffuser/dropper. `None` si pas un mouton tondable.
    pub fn shear_sheep(&mut self, runtime_id: u64) -> Option<(Vec<ItemStack>, Vec<u8>)> {
        use mc_rs_proto::packets::player::entity_flags;
        let e = self.entities.get_mut(&runtime_id)?;
        if e.kind != MobKind::Sheep || e.sheared || e.baby {
            return None;
        }
        e.sheared = true;
        e.base.set_entity_flag(entity_flags::SHEARED, true);
        let wool = ItemStack::new(
            item_registry::required_item_id("minecraft:white_wool"),
            1,
            0,
        );
        Some((vec![wool], e.base.actor_data_packet()))
    }

    pub fn all(&self) -> impl Iterator<Item = &MobEntity> {
        self.entities.values()
    }

    /// Applique un knockback à un mob (poussé à l'opposé de `attacker_pos`).
    /// No-op si le mob n'existe plus (mort). La vélocité est diffusée au tick
    /// suivant ; le `WalkController` ne l'écrase pas (garde anti-knockback).
    pub fn apply_knockback(&mut self, runtime_id: u64, attacker_pos: [f32; 3], force: f32) {
        if let Some(e) = self.entities.get_mut(&runtime_id) {
            let dx = e.base.position[0] - attacker_pos[0];
            let dz = e.base.position[2] - attacker_pos[2];
            let len = (dx * dx + dz * dz).sqrt();
            if len > 0.0001 {
                e.base.velocity[0] = dx / len * force;
                e.base.velocity[2] = dz / len * force;
                e.base.velocity[1] = 0.4; // composante verticale (comme PMMP)
            }
        }
    }

    pub fn apply_attack(&mut self, entity_runtime_id: u64, damage: f32) -> Option<DamageResult> {
        let entity = self.entities.get_mut(&entity_runtime_id)?;
        let max_health = entity.kind.max_health();
        let current = entity
            .base
            .attributes
            .iter()
            .find(|attribute| attribute.name == "minecraft:health")
            .map(|attribute| attribute.current)
            .unwrap_or(max_health);
        let new_health = (current - damage).max(0.0);

        if let Some(attribute) = entity
            .base
            .attributes
            .iter_mut()
            .find(|attribute| attribute.name == "minecraft:health")
        {
            attribute.current = new_health;
        }

        let runtime_entity_id = entity.base.entity_runtime_id;
        let actor_type = entity.kind.actor_type();
        let (remove_packet, death_position, drops) = if new_health <= 0.0 {
            self.ai.remove(&runtime_entity_id);
            let entity = self.entities.remove(&runtime_entity_id)?;
            // Loot tables vanilla via bedrock-samples (data-driven). Si la
            // table existe → utilisée ; sinon fallback sur default_loot()
            // hardcodé pour rester rétro-compatible.
            let drops = if crate::loot_table::has_loot_table(actor_type) {
                let ctx = crate::loot_table::LootContext {
                    killed_by_player: true,
                    looting_level: 0,
                    ..Default::default()
                };
                let rolled = crate::loot_table::roll_entity_loot(actor_type, ctx);
                rolled
                    .into_iter()
                    .filter_map(|(name, count)| {
                        crate::item_registry::network_id(&name)
                            .map(|id| ItemStack::new(id, count as u16, 0))
                    })
                    .collect()
            } else {
                entity.kind.default_loot()
            };
            (
                Some(entity.remove_packet()),
                Some(entity.base.position),
                drops,
            )
        } else {
            (None, None, Vec::new())
        };

        let update_attributes_packet = if remove_packet.is_none() {
            Some(
                UpdateAttributes {
                    runtime_entity_id,
                    attributes: vec![PlayerAttribute {
                        name: "minecraft:health".to_string(),
                        min: 0.0,
                        max: max_health,
                        current: new_health,
                        default: max_health,
                    }],
                    tick: 0,
                }
                .encode(),
            )
        } else {
            None
        };

        Some(DamageResult {
            update_attributes_packet,
            remove_packet,
            death_position,
            drops,
        })
    }

    pub fn tick(
        &mut self,
        chunk_cache: &mut ChunkCache,
        players: &[PlayerSnapshot],
        is_day: bool,
    ) -> TickResult {
        let mut movement_updates = Vec::new();
        let mut attack_requests = Vec::new();
        let mut to_despawn = Vec::new();
        let mut metadata_updates = Vec::new();
        let mut damage_requests = Vec::new();
        let mut block_changes = Vec::new();
        let ids = self.entities.keys().copied().collect::<Vec<_>>();

        for entity_id in ids {
            let Some(entity) = self.entities.get_mut(&entity_id) else {
                continue;
            };

            // --- Despawn des hostiles trop loin de tout joueur (règles Bedrock) ---
            // (Si aucun joueur connecté, on ne despawn pas : pas de référentiel.)
            if entity.kind.is_hostile() && !players.is_empty() {
                let nearest_sq = players
                    .iter()
                    .map(|p| {
                        let dx = (p.position[0] - entity.base.position[0]) as f64;
                        let dy = (p.position[1] - entity.base.position[1]) as f64;
                        let dz = (p.position[2] - entity.base.position[2]) as f64;
                        dx * dx + dy * dy + dz * dz
                    })
                    .fold(f64::MAX, f64::min);
                if nearest_sq > DESPAWN_INSTANT_DIST_SQ {
                    to_despawn.push(entity_id); // trop loin → despawn immédiat
                    continue;
                } else if nearest_sq > DESPAWN_FAR_DIST_SQ {
                    entity.despawn_timer += 1;
                    if entity.despawn_timer >= DESPAWN_TIMER_MAX {
                        to_despawn.push(entity_id);
                        continue;
                    }
                } else {
                    entity.despawn_timer = 0;
                }
            }

            // --- Timers de reproduction + croissance des bébés ---
            if entity.in_love > 0 {
                entity.in_love -= 1;
            }
            if entity.breed_cooldown > 0 {
                entity.breed_cooldown -= 1;
            }
            if entity.baby {
                entity.baby_age += 1;
                if entity.baby_age >= BABY_GROW_TICKS {
                    use mc_rs_proto::packets::player::entity_flags;
                    entity.baby = false;
                    entity.base.set_entity_flag(entity_flags::BABY, false);
                    entity.base.set_scale(1.0);
                    metadata_updates
                        .push((entity.base.entity_unique_id, entity.base.actor_data_packet()));
                }
            }

            // --- IA : sensors → behaviors → controllers (pose vélocité/yaw) ---
            // Tournée AVANT la physique pour que le WalkController fixe la
            // vélocité que la physique intègre ensuite. `self.ai` et
            // `self.entities` sont des champs disjoints → emprunts mut OK.
            if let Some(ai) = self.ai.get_mut(&entity_id) {
                ai.tick(
                    &mut entity.base,
                    entity.kind,
                    players,
                    chunk_cache,
                    &mut attack_requests,
                );
            }

            let old_position = entity.base.position;
            let old_velocity = entity.base.velocity;
            let (_width, height) = entity.kind.size();
            let feet_in_water = in_water(chunk_cache, entity.base.position);

            // --- Gravité (vertical) ---
            entity.base.velocity[1] =
                (entity.base.velocity[1] - GRAVITY_PER_TICK).max(MAX_FALL_SPEED);
            entity.base.velocity[1] *= AIR_DRAG;
            // Flottaison : dans l'eau, poussée vers le haut → le mob remonte/nage.
            if feet_in_water {
                entity.base.velocity[1] = (entity.base.velocity[1] + WATER_BUOYANCY).min(WATER_MAX_RISE);
            }

            // --- Déplacement horizontal avec collision murale + step-up ---
            move_horizontal(&mut entity.base, height, chunk_cache);

            // --- Déplacement vertical avec collision au sol ---
            // On ne clampe (et ne marque on_ground) que quand le mob descend :
            // sinon un saut serait annulé dès l'ascension par le bloc de départ.
            let mut next_y = entity.base.position[1] + entity.base.velocity[1];
            let world_x = entity.base.position[0].floor() as i32;
            let world_z = entity.base.position[2].floor() as i32;
            let mut on_ground = false;
            if entity.base.velocity[1] <= 0.0 {
                let support_y = (next_y - 0.01).floor() as i32;
                if is_supporting_block(chunk_cache.get_block(world_x, support_y, world_z)) {
                    let floor_y = support_y as f32 + 1.0;
                    if next_y <= floor_y {
                        next_y = floor_y;
                        entity.base.velocity[1] = 0.0;
                        on_ground = true;
                    }
                }
            }
            entity.base.position[1] = next_y;
            entity.base.on_ground = on_ground;

            // --- Dégâts de chute : accumule la descente, applique à l'atterrissage ---
            if feet_in_water {
                entity.fall_distance = 0.0; // l'eau annule la chute
            } else if on_ground {
                if entity.fall_distance > FALL_DAMAGE_THRESHOLD {
                    let dmg = (entity.fall_distance - FALL_DAMAGE_THRESHOLD).floor();
                    if dmg > 0.0 {
                        damage_requests.push((entity_id, dmg));
                    }
                }
                entity.fall_distance = 0.0;
            } else {
                let dropped = (old_position[1] - next_y).max(0.0);
                entity.fall_distance += dropped;
            }

            // --- Friction horizontale (sol vs air) ---
            let friction = if on_ground { GROUND_FRICTION } else { AIR_FRICTION };
            entity.base.velocity[0] *= friction;
            entity.base.velocity[2] *= friction;

            // --- Sun-burning : zombies/skeletons brûlent en plein jour à découvert ---
            if matches!(entity.kind, MobKind::Zombie | MobKind::Skeleton) {
                let burning =
                    is_day && !feet_in_water && sky_exposed(chunk_cache, entity.base.position, height);
                // Toggle du flag ONFIRE (broadcast SetActorData uniquement si changement).
                use mc_rs_proto::packets::player::entity_flags;
                if entity.base.set_entity_flag(entity_flags::ONFIRE, burning) {
                    metadata_updates
                        .push((entity.base.entity_unique_id, entity.base.actor_data_packet()));
                }
                if burning {
                    entity.fire_tick += 1;
                    if entity.fire_tick >= 20 {
                        entity.fire_tick = 0;
                        damage_requests.push((entity_id, 1.0)); // 1 HP/s appliqué par main.rs
                    }
                } else {
                    entity.fire_tick = 0;
                }
            }

            // --- Mouton tondu qui mange l'herbe → repousse sa laine (C3) ---
            if entity.kind == MobKind::Sheep && entity.sheared && !entity.baby {
                let fx = entity.base.position[0].floor() as i32;
                let fy = entity.base.position[1].floor() as i32 - 1; // bloc sous les pieds
                let fz = entity.base.position[2].floor() as i32;
                if chunk_cache.get_block(fx, fy, fz) == BLOCKS.grass_block {
                    entity.eat_grass_timer += 1;
                    if entity.eat_grass_timer >= 100 {
                        entity.eat_grass_timer = 0;
                        chunk_cache.set_block(fx, fy, fz, BLOCKS.dirt);
                        block_changes.push(([fx, fy, fz], BLOCKS.dirt));
                        // Repousse la laine.
                        use mc_rs_proto::packets::player::entity_flags;
                        entity.sheared = false;
                        entity.base.set_entity_flag(entity_flags::SHEARED, false);
                        metadata_updates
                            .push((entity.base.entity_unique_id, entity.base.actor_data_packet()));
                    }
                } else {
                    entity.eat_grass_timer = 0;
                }
            }

            let position_changed = (0..3).any(|i| {
                (entity.base.position[i] - old_position[i]).abs() > 0.0001
            });
            let velocity_changed =
                (0..3).any(|i| (entity.base.velocity[i] - old_velocity[i]).abs() > 0.0001);

            if position_changed || velocity_changed {
                movement_updates.push(MovementUpdate {
                    entity_unique_id: entity.base.entity_unique_id,
                    entity_position: entity.base.position,
                    add_packet: entity.base.add_actor_packet(),
                    move_packet: entity
                        .base
                        .move_absolute_packet(on_ground && entity.base.velocity[1] == 0.0, false),
                    motion_packet: entity.base.motion_packet(),
                });
            }
        }

        // Retire les mobs despawnés (entité + composant IA) et les renvoie pour
        // que `main.rs` broadcast leur RemoveEntity.
        let mut despawned = Vec::new();
        for id in to_despawn {
            self.ai.remove(&id);
            if let Some(e) = self.entities.remove(&id) {
                despawned.push(e);
            }
        }

        // Reproduction : apparie deux adultes en amour, de même espèce, proches.
        self.try_breed();

        TickResult {
            movement_updates,
            attack_requests,
            despawned,
            metadata_updates,
            damage_requests,
            block_changes,
        }
    }

    /// Apparie deux adultes en mode amour, de même espèce, à portée → un bébé
    /// naît entre eux ; les deux parents passent en cooldown. Une paire/tick.
    fn try_breed(&mut self) {
        let lovers: Vec<(u64, MobKind, [f32; 3])> = self
            .entities
            .values()
            .filter(|e| e.in_love > 0 && !e.baby && e.breed_cooldown == 0)
            .map(|e| (e.base.entity_runtime_id, e.kind, e.base.position))
            .collect();

        for i in 0..lovers.len() {
            for j in (i + 1)..lovers.len() {
                let (id_a, kind_a, pos_a) = lovers[i];
                let (id_b, kind_b, pos_b) = lovers[j];
                if kind_a != kind_b {
                    continue;
                }
                let dx = (pos_a[0] - pos_b[0]) as f64;
                let dz = (pos_a[2] - pos_b[2]) as f64;
                if dx * dx + dz * dz > BREED_RANGE_SQ {
                    continue;
                }
                // Naissance au milieu des parents ; reset amour + cooldown.
                for id in [id_a, id_b] {
                    if let Some(e) = self.entities.get_mut(&id) {
                        e.in_love = 0;
                        e.breed_cooldown = BREED_COOLDOWN;
                    }
                }
                let mid = [
                    (pos_a[0] + pos_b[0]) * 0.5,
                    (pos_a[1] + pos_b[1]) * 0.5,
                    (pos_a[2] + pos_b[2]) * 0.5,
                ];
                let baby = MobEntity::spawn_baby(kind_a, mid);
                self.insert_with_ai(baby);
                return; // une seule reproduction par tick
            }
        }
    }
}

/// Le mob a-t-il les pieds dans l'eau ?
fn in_water(cache: &mut ChunkCache, pos: [f32; 3]) -> bool {
    let id = cache.get_block(pos[0].floor() as i32, pos[1].floor() as i32, pos[2].floor() as i32);
    id == BLOCKS.water
}

/// Le mob est-il exposé au ciel (aucun bloc opaque au-dessus, jusqu'à 64 blocs) ?
fn sky_exposed(cache: &mut ChunkCache, pos: [f32; 3], height: f32) -> bool {
    let x = pos[0].floor() as i32;
    let z = pos[2].floor() as i32;
    let head = (pos[1] + height).floor() as i32;
    for dy in 1..=64 {
        if is_supporting_block(cache.get_block(x, head + dy, z)) {
            return false;
        }
    }
    true
}

/// Déplace le mob horizontalement selon sa vélocité, **axe par axe** (glissement
/// le long des murs), avec collision murale et franchissement automatique d'une
/// marche d'1 bloc (step-up). Collision au centre (les hitboxes de mobs font < 1
/// bloc de large), suffisante pour la v1.
fn move_horizontal(base: &mut EntityBase, height: f32, cache: &mut ChunkCache) {
    for axis in [0usize, 2usize] {
        let v = base.velocity[axis];
        if v == 0.0 {
            continue;
        }
        let feet_y = base.position[1].floor() as i32;
        let head_y = (base.position[1] + height - 0.001).floor() as i32;
        let mut cand = base.position;
        cand[axis] += v;

        if !column_blocked(cache, cand[0], cand[2], feet_y, head_y) {
            base.position[axis] = cand[axis];
        } else if base.on_ground && !column_blocked(cache, cand[0], cand[2], feet_y + 1, head_y + 1) {
            // Obstacle d'1 bloc avec espace libre au-dessus → on grimpe d'1 bloc.
            base.position[1] = (feet_y + 1) as f32;
            base.position[axis] = cand[axis];
        } else {
            // Mur trop haut → on s'arrête sur cet axe (glissement possible sur l'autre).
            base.velocity[axis] = 0.0;
        }
    }
}

/// Y a-t-il un bloc solide dans la colonne verticale `[y0, y1]` à la cellule
/// horizontale contenant `(x, z)` ?
fn column_blocked(cache: &mut ChunkCache, x: f32, z: f32, y0: i32, y1: i32) -> bool {
    let bx = x.floor() as i32;
    let bz = z.floor() as i32;
    (y0..=y1).any(|by| is_supporting_block(cache.get_block(bx, by, bz)))
}

/// Les blocs "non-solides" (passable par les items/mobs en chute).
/// Un item drop qui tombe dans un bambou / un massif de fleurs doit
/// continuer sa chute jusqu'au sol réel, pas se poser sur le bambou.
///
/// Source canonique : `.reference/Allay/data/resources/block_tags_custom.json`
/// (`minecraft:replaceable` + blocs sans collision comme bamboo/cactus/torches).
pub(crate) fn is_supporting_block(runtime_id: u32) -> bool {
    // Plantes, fluides, décorations : items passent à travers.
    let b = &*BLOCKS;
    if runtime_id == b.air
        || runtime_id == b.water
        || runtime_id == b.lava
        // Petites plantes
        || runtime_id == b.short_grass
        || runtime_id == b.tall_grass
        || runtime_id == b.fern
        || runtime_id == b.large_fern
        || runtime_id == b.dandelion
        || runtime_id == b.poppy
        || runtime_id == b.blue_orchid
        || runtime_id == b.allium
        || runtime_id == b.azure_bluet
        || runtime_id == b.oxeye_daisy
        || runtime_id == b.cornflower
        || runtime_id == b.waterlily
        || runtime_id == b.seagrass
        || runtime_id == b.brown_mushroom
        || runtime_id == b.red_mushroom
        || runtime_id == b.reeds
        // Bamboo + plants / décorations qui n'étaient pas exclues.
        || runtime_id == b.bamboo
        || runtime_id == b.deadbush
        || runtime_id == b.pumpkin
    {
        return false;
    }

    // Fallback : check par nom via block_attachment::is_solid_support.
    let name = b.name_for(runtime_id).unwrap_or("");
    crate::block_attachment::is_solid_support(name)
}

#[cfg(test)]
mod tests {
    use super::{is_supporting_block, move_horizontal, MobEntity, MobEntityManager, MobKind};
    use crate::item_registry;
    use crate::world::block_registry::BLOCKS;
    use crate::world::chunk_cache::ChunkCache;

    fn temp_cache(tag: &str) -> (ChunkCache, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mc-rs-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Générateur "flat" : air au-dessus de la couche de surface basse, donc
        // y >= 64 est dégagé → collisions/step-up déterministes (on place nous-mêmes les blocs).
        (ChunkCache::new(&dir, 1, "flat"), dir)
    }

    #[test]
    fn wall_blocks_horizontal_movement() {
        let (mut cache, dir) = temp_cache("mob-wall");
        // Mur de 2 blocs de haut en x=1.
        cache.set_block(1, 64, 0, BLOCKS.stone);
        cache.set_block(1, 65, 0, BLOCKS.stone);

        let mut zombie = MobEntity::spawn(MobKind::Zombie, [0.7, 64.0, 0.5]);
        zombie.base.on_ground = true;
        zombie.base.velocity[0] = 0.5; // viserait x=1.2 → dans le mur

        let height = MobKind::Zombie.size().1;
        move_horizontal(&mut zombie.base, height, &mut cache);

        // Bloqué : ni avance en X, ni step-up (mur trop haut).
        assert!((zombie.base.position[0] - 0.7).abs() < 1e-6, "ne doit pas traverser le mur");
        assert_eq!(zombie.base.velocity[0], 0.0, "vitesse X annulée par la collision");
        assert!((zombie.base.position[1] - 64.0).abs() < 1e-6, "pas de step-up sur mur 2 blocs");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mob_takes_fall_damage_on_landing() {
        let (mut cache, dir) = temp_cache("mob-fall");
        cache.set_block(0, 64, 0, BLOCKS.stone); // bloc isolé en l'air (monde flat)

        let mut mobs = MobEntityManager::new();
        mobs.spawn(MobKind::Zombie, [0.5, 80.0, 0.5]); // chute de ~15 blocs

        let mut total_fall_dmg = 0.0;
        for _ in 0..200 {
            // is_day=false → pas de sun-burning : le seul dégât possible = la chute.
            let r = mobs.tick(&mut cache, &[], false);
            for (_, dmg) in r.damage_requests {
                total_fall_dmg += dmg;
            }
        }
        assert!(
            total_fall_dmg > 0.0,
            "le mob doit subir des dégâts de chute (got {total_fall_dmg})"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_fed_cows_breed_into_a_baby() {
        let (mut cache, dir) = temp_cache("mob-breed");
        let mut mobs = MobEntityManager::new();
        let a = mobs.spawn(MobKind::Cow, [0.5, 64.0, 0.5]);
        let b = mobs.spawn(MobKind::Cow, [1.5, 64.0, 0.5]); // à 1 bloc → à portée
        let wheat = crate::item_registry::network_id("minecraft:wheat").expect("blé existe");

        assert!(mobs.feed_mob(a.base.entity_runtime_id, wheat), "vache A nourrie");
        assert!(mobs.feed_mob(b.base.entity_runtime_id, wheat), "vache B nourrie");

        let before = mobs.all().count();
        let _ = mobs.tick(&mut cache, &[], false);
        assert_eq!(mobs.all().count(), before + 1, "un bébé doit naître");
        assert!(mobs.all().any(|e| e.baby), "le nouveau-né est un bébé");

        // Nourrir un mob non reproductible (zombie) échoue.
        let z = mobs.spawn(MobKind::Zombie, [50.0, 64.0, 50.0]);
        assert!(!mobs.feed_mob(z.base.entity_runtime_id, wheat), "zombie non reproductible");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sheared_sheep_eats_grass_and_regrows_wool() {
        let (mut cache, dir) = temp_cache("sheep-grass");
        cache.set_block(0, 64, 0, BLOCKS.grass_block); // bloc d'herbe sous le mouton

        let mut mobs = MobEntityManager::new();
        let s = mobs.spawn(MobKind::Sheep, [0.5, 65.0, 0.5]);
        let id = s.base.entity_runtime_id;

        // Tonte : 1re fois OK, 2e fois refusée (déjà tondu).
        assert!(mobs.shear_sheep(id).is_some(), "tonte réussie");
        assert!(mobs.shear_sheep(id).is_none(), "déjà tondu");

        // En broutant l'herbe, il finit par la manger et regagne sa laine.
        let mut ate = false;
        for _ in 0..200 {
            let r = mobs.tick(&mut cache, &[], false);
            if !r.block_changes.is_empty() {
                ate = true;
            }
        }
        assert!(ate, "le mouton tondu doit manger l'herbe");
        assert_eq!(cache.get_block(0, 64, 0), BLOCKS.dirt, "grass_block → dirt");
        assert!(
            mobs.all()
                .find(|e| e.base.entity_runtime_id == id)
                .map(|e| !e.sheared)
                .unwrap_or(false),
            "la laine a repoussé (plus tondu)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn steps_up_a_single_block() {
        let (mut cache, dir) = temp_cache("mob-step");
        // Marche d'1 bloc en x=1 (rien au-dessus).
        cache.set_block(1, 64, 0, BLOCKS.stone);

        let mut zombie = MobEntity::spawn(MobKind::Zombie, [0.7, 64.0, 0.5]);
        zombie.base.on_ground = true;
        zombie.base.velocity[0] = 0.5;

        let height = MobKind::Zombie.size().1;
        move_horizontal(&mut zombie.base, height, &mut cache);

        // A grimpé d'1 bloc et avancé en X.
        assert!((zombie.base.position[1] - 65.0).abs() < 1e-6, "doit grimper d'1 bloc");
        assert!(zombie.base.position[0] > 1.0, "doit avancer par-dessus la marche");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_mob_names() {
        assert_eq!(MobKind::parse("zombie"), Some(MobKind::Zombie));
        assert_eq!(MobKind::parse("minecraft:cow"), Some(MobKind::Cow));
        assert_eq!(MobKind::parse("unknown"), None);
    }

    #[test]
    fn namespaced_types_and_short_types_share_selector_space() {
        assert!(is_supporting_block(BLOCKS.stone));
        assert!(!is_supporting_block(BLOCKS.air));
        assert!(!is_supporting_block(BLOCKS.short_grass));
    }

    #[test]
    fn mobs_fall_until_they_reach_the_ground() {
        let test_dir =
            std::env::temp_dir().join(format!("mc-rs-mob-physics-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&test_dir);
        // Monde "flat" : air au-dessus de la surface basse, donc le bloc de pierre
        // posé à (0,64,0) est isolé en l'air → l'IA de roam ne trouve aucun sol
        // alentour (pas de chemin) et le mob tombe en ligne droite, sans errer.
        let mut cache = ChunkCache::new(&test_dir, 42, "flat");
        cache.set_block(0, 64, 0, BLOCKS.stone);

        let mut mobs = MobEntityManager::new();
        let entity = mobs.spawn(MobKind::Zombie, [0.5, 70.0, 0.5]);
        let entity_id = entity.base.entity_runtime_id;

        for _ in 0..200 {
            let _ = mobs.tick(&mut cache, &[], false);
        }

        let settled = mobs
            .all()
            .find(|entity| entity.base.entity_runtime_id == entity_id)
            .expect("entity exists");
        assert!(
            settled.base.position[1] < 70.0,
            "expected zombie to fall, got {}",
            settled.base.position[1]
        );
        assert_eq!(settled.base.velocity[1], 0.0);
        let support_y = (settled.base.position[1] - 0.01).floor() as i32;
        let (mob_x, mob_z) = (
            settled.base.position[0].floor() as i32,
            settled.base.position[2].floor() as i32,
        );
        assert!(
            is_supporting_block(cache.get_block(mob_x, support_y, mob_z)),
            "expected zombie to settle on a supporting block"
        );
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn attacking_a_mob_updates_health_and_can_remove_it() {
        let mut mobs = MobEntityManager::new();
        let zombie = mobs.spawn(MobKind::Zombie, [0.5, 70.0, 0.5]);

        let first_hit = mobs
            .apply_attack(zombie.base.entity_runtime_id, 4.0)
            .expect("mob should exist");
        assert!(first_hit.update_attributes_packet.is_some());
        assert!(first_hit.remove_packet.is_none());

        for _ in 0..4 {
            let _ = mobs.apply_attack(zombie.base.entity_runtime_id, 4.0);
        }

        assert!(mobs
            .all()
            .all(|entity| entity.base.entity_runtime_id != zombie.base.entity_runtime_id));
    }

    #[test]
    fn killing_a_mob_returns_position_and_loot() {
        // La cow utilise maintenant la loot_table vanilla (bedrock-samples)
        // avec set_count(0..2) pour leather + beef → drops aléatoires 0..4
        // items. On retry jusqu'à obtenir un drop pour avoir un test stable
        // qui valide le contrat API (kill → drops + position).
        let beef_id = item_registry::required_item_id("minecraft:beef");
        let leather_id = item_registry::required_item_id("minecraft:leather");

        for _ in 0..30 {
            let mut mobs = MobEntityManager::new();
            let cow = mobs.spawn(MobKind::Cow, [4.5, 70.0, -2.5]);
            let mut last_result = None;
            for _ in 0..3 {
                last_result = mobs.apply_attack(cow.base.entity_runtime_id, 4.0);
            }
            let result = last_result.expect("expected kill result");
            assert!(result.remove_packet.is_some());
            assert_eq!(result.death_position, Some([4.5, 70.0, -2.5]));
            // Si la roll donne au moins 1 drop d'un type vanilla cow → succès.
            if result
                .drops
                .iter()
                .any(|item| item.id == beef_id || item.id == leather_id)
            {
                return;
            }
        }
        panic!("after 30 cow kills, never got beef or leather drop — loot table issue");
    }
}
