use std::collections::HashMap;

use mc_rs_proto::packets::player::{ItemStack, PlayerAttribute, UpdateAttributes};

use crate::entity::{health_attributes, living_metadata, EntityBase};
use crate::item_registry;
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;

const SERVER_TICKS_PER_SECOND: f32 = 100.0;
const BASELINE_TICKS_PER_SECOND: f32 = 20.0;
const GRAVITY_PER_TICK: f32 = 0.08 * (BASELINE_TICKS_PER_SECOND / SERVER_TICKS_PER_SECOND);
const AIR_DRAG: f32 = 0.996;
const MAX_FALL_SPEED: f32 = -0.784;

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

    pub fn max_health(self) -> f32 {
        // Source autoritaire : `mob_hp::mob_max_hp` (couvre ~60 entités vanilla).
        crate::mob_hp::mob_max_hp(self.actor_type())
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
        Self { base, kind }
    }

    pub fn add_actor_packet(&self) -> Vec<u8> {
        self.base.add_actor_packet()
    }

    pub fn remove_packet(&self) -> Vec<u8> {
        self.base.remove_packet()
    }
}

pub struct MovementUpdate {
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
}

pub struct MobEntityManager {
    entities: HashMap<u64, MobEntity>,
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
        }
    }

    pub fn spawn(&mut self, kind: MobKind, position: [f32; 3]) -> MobEntity {
        let entity = MobEntity::spawn(kind, position);
        self.entities
            .insert(entity.base.entity_runtime_id, entity.clone());
        entity
    }

    pub fn remove(&mut self, entity_runtime_id: u64) -> Option<MobEntity> {
        self.entities.remove(&entity_runtime_id)
    }

    pub fn all(&self) -> impl Iterator<Item = &MobEntity> {
        self.entities.values()
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

    pub fn tick(&mut self, chunk_cache: &mut ChunkCache) -> TickResult {
        let mut movement_updates = Vec::new();
        let ids = self.entities.keys().copied().collect::<Vec<_>>();

        for entity_id in ids {
            let Some(entity) = self.entities.get_mut(&entity_id) else {
                continue;
            };

            let old_position = entity.base.position;
            let old_velocity = entity.base.velocity;

            entity.base.velocity[1] =
                (entity.base.velocity[1] - GRAVITY_PER_TICK).max(MAX_FALL_SPEED);
            entity.base.velocity[1] *= AIR_DRAG;

            let mut next_y = entity.base.position[1] + entity.base.velocity[1];
            let world_x = entity.base.position[0].floor() as i32;
            let world_z = entity.base.position[2].floor() as i32;
            let support_y = (next_y - 0.01).floor() as i32;
            let support_block = chunk_cache.get_block(world_x, support_y, world_z);
            let on_ground = is_supporting_block(support_block);

            if on_ground {
                let floor_y = support_y as f32 + 1.0;
                if next_y <= floor_y {
                    next_y = floor_y;
                    entity.base.velocity[1] = 0.0;
                }
            }

            entity.base.position[1] = next_y;

            let position_changed = (entity.base.position[1] - old_position[1]).abs() > 0.0001;
            let velocity_changed = (entity.base.velocity[1] - old_velocity[1]).abs() > 0.0001;

            if position_changed || velocity_changed {
                movement_updates.push(MovementUpdate {
                    move_packet: entity
                        .base
                        .move_absolute_packet(on_ground && entity.base.velocity[1] == 0.0, false),
                    motion_packet: entity.base.motion_packet(),
                });
            }
        }

        TickResult { movement_updates }
    }
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
    use super::{is_supporting_block, MobEntityManager, MobKind};
    use crate::item_registry;
    use crate::world::block_registry::BLOCKS;
    use crate::world::chunk_cache::ChunkCache;

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
        let mut cache = ChunkCache::new(&test_dir, 42, "normal");
        cache.set_block(0, 64, 0, BLOCKS.stone);

        let mut mobs = MobEntityManager::new();
        let entity = mobs.spawn(MobKind::Zombie, [0.5, 70.0, 0.5]);
        let entity_id = entity.base.entity_runtime_id;

        for _ in 0..200 {
            let _ = mobs.tick(&mut cache);
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
        assert!(
            is_supporting_block(cache.get_block(0, support_y, 0)),
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
