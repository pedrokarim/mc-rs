//! Mob AI — port sélectif de `.reference/PocketMine-MP/src/entity/*` (comportements
//! par-mob basiques).
//!
//! Architecture PMMP : chaque mob étend `Living` ou `Animal` et override
//! `entityBaseTick()` / `attack()` avec sa logique. Ici on reproduit les
//! comportements les plus utiles : wander, flee, target, attack range.

use crate::event::entity::DamageCause;

/// Type de mob vanilla. Port `Entity::getNetworkTypeId()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobKind {
    // Passifs
    Chicken,
    Cow,
    Pig,
    Sheep,
    Rabbit,
    Squid,
    Villager,
    Horse,
    Donkey,
    Llama,
    Cat,
    Wolf,
    Ocelot,
    Parrot,
    Fox,
    Panda,
    Turtle,
    Dolphin,
    // Hostiles
    Zombie,
    Skeleton,
    Creeper,
    Spider,
    CaveSpider,
    Enderman,
    Witch,
    Blaze,
    Ghast,
    MagmaCube,
    Slime,
    Drowned,
    Husk,
    Stray,
    ZombieVillager,
    WitherSkeleton,
    // Neutres
    ZombiePigman,
    Wither,
    EnderDragon,
    Ravager,
    Vindicator,
    Pillager,
    IronGolem,
    SnowGolem,
}

impl MobKind {
    pub fn network_identifier(&self) -> &'static str {
        match self {
            Self::Chicken => "minecraft:chicken",
            Self::Cow => "minecraft:cow",
            Self::Pig => "minecraft:pig",
            Self::Sheep => "minecraft:sheep",
            Self::Rabbit => "minecraft:rabbit",
            Self::Squid => "minecraft:squid",
            Self::Villager => "minecraft:villager_v2",
            Self::Horse => "minecraft:horse",
            Self::Donkey => "minecraft:donkey",
            Self::Llama => "minecraft:llama",
            Self::Cat => "minecraft:cat",
            Self::Wolf => "minecraft:wolf",
            Self::Ocelot => "minecraft:ocelot",
            Self::Parrot => "minecraft:parrot",
            Self::Fox => "minecraft:fox",
            Self::Panda => "minecraft:panda",
            Self::Turtle => "minecraft:turtle",
            Self::Dolphin => "minecraft:dolphin",
            Self::Zombie => "minecraft:zombie",
            Self::Skeleton => "minecraft:skeleton",
            Self::Creeper => "minecraft:creeper",
            Self::Spider => "minecraft:spider",
            Self::CaveSpider => "minecraft:cave_spider",
            Self::Enderman => "minecraft:enderman",
            Self::Witch => "minecraft:witch",
            Self::Blaze => "minecraft:blaze",
            Self::Ghast => "minecraft:ghast",
            Self::MagmaCube => "minecraft:magma_cube",
            Self::Slime => "minecraft:slime",
            Self::Drowned => "minecraft:drowned",
            Self::Husk => "minecraft:husk",
            Self::Stray => "minecraft:stray",
            Self::ZombieVillager => "minecraft:zombie_villager_v2",
            Self::WitherSkeleton => "minecraft:wither_skeleton",
            Self::ZombiePigman => "minecraft:zombie_pigman",
            Self::Wither => "minecraft:wither",
            Self::EnderDragon => "minecraft:ender_dragon",
            Self::Ravager => "minecraft:ravager",
            Self::Vindicator => "minecraft:vindicator",
            Self::Pillager => "minecraft:pillager",
            Self::IronGolem => "minecraft:iron_golem",
            Self::SnowGolem => "minecraft:snow_golem",
        }
    }

    /// Santé max PMMP.
    pub fn max_health(&self) -> f32 {
        match self {
            Self::Chicken | Self::Rabbit => 4.0,
            Self::Cow | Self::Pig | Self::Sheep | Self::Squid | Self::Dolphin => 10.0,
            Self::Wolf | Self::Cat | Self::Ocelot | Self::Parrot => 8.0,
            Self::Fox => 10.0,
            Self::Panda => 20.0,
            Self::Turtle => 30.0,
            Self::Villager => 20.0,
            Self::Horse | Self::Donkey | Self::Llama => 15.0,
            Self::Zombie | Self::Husk | Self::Drowned | Self::ZombieVillager => 20.0,
            Self::Skeleton | Self::Stray | Self::WitherSkeleton => 20.0,
            Self::Creeper => 20.0,
            Self::Spider | Self::CaveSpider => 16.0,
            Self::Enderman => 40.0,
            Self::Witch => 26.0,
            Self::Blaze => 20.0,
            Self::Ghast => 10.0,
            Self::MagmaCube | Self::Slime => 16.0,
            Self::ZombiePigman => 20.0,
            Self::Wither => 300.0,
            Self::EnderDragon => 200.0,
            Self::Ravager => 100.0,
            Self::Vindicator | Self::Pillager => 24.0,
            Self::IronGolem => 100.0,
            Self::SnowGolem => 4.0,
        }
    }

    /// Attack damage (base, sans multiplier difficulty).
    pub fn attack_damage(&self) -> f32 {
        match self {
            Self::Zombie | Self::Drowned | Self::Husk | Self::ZombieVillager => 3.0,
            Self::Spider | Self::CaveSpider => 2.0,
            Self::Skeleton | Self::Stray => 0.0, // damage via arrows
            Self::Creeper => 0.0,                // damage via explosion
            Self::Enderman => 7.0,
            Self::WitherSkeleton => 5.0,
            Self::Blaze => 6.0,
            Self::Ghast => 0.0, // fireball
            Self::MagmaCube => 4.0,
            Self::Wither => 8.0,
            Self::EnderDragon => 10.0,
            Self::Ravager => 12.0,
            Self::Vindicator => 13.0,
            Self::Pillager => 0.0, // crossbow
            Self::IronGolem => 15.0,
            Self::Wolf => 3.0,
            _ => 0.0,
        }
    }

    pub fn is_hostile(&self) -> bool {
        matches!(
            self,
            Self::Zombie
                | Self::Skeleton
                | Self::Creeper
                | Self::Spider
                | Self::CaveSpider
                | Self::Enderman
                | Self::Witch
                | Self::Blaze
                | Self::Ghast
                | Self::MagmaCube
                | Self::Slime
                | Self::Drowned
                | Self::Husk
                | Self::Stray
                | Self::ZombieVillager
                | Self::WitherSkeleton
                | Self::Wither
                | Self::EnderDragon
                | Self::Ravager
                | Self::Vindicator
                | Self::Pillager
        )
    }

    pub fn is_passive(&self) -> bool {
        !self.is_hostile() && !matches!(self, Self::Wolf | Self::ZombiePigman | Self::IronGolem)
    }

    /// Distance de détection d'un joueur (sight range).
    pub fn sight_range(&self) -> f32 {
        match self {
            Self::Zombie | Self::Skeleton | Self::Creeper | Self::Drowned => 16.0,
            Self::Spider => 16.0,
            Self::Enderman => 64.0,
            Self::Wither | Self::EnderDragon => 80.0,
            Self::IronGolem => 16.0,
            _ => 16.0,
        }
    }

    /// Distance d'attaque mêlée.
    pub fn attack_range(&self) -> f32 {
        match self {
            Self::Ghast | Self::Blaze | Self::Skeleton | Self::Stray | Self::Pillager => 16.0,
            _ => 2.0,
        }
    }
}

/// Objectif IA d'un mob.
#[derive(Debug, Clone, PartialEq)]
pub enum AiGoal {
    Wander,
    FollowTarget { target_runtime_id: u64 },
    Flee { from_runtime_id: u64 },
    Attack { target_runtime_id: u64 },
    Sit, // wolves/cats ordered to sit
    Idle,
}

/// État IA par-mob.
#[derive(Debug, Clone)]
pub struct MobAi {
    pub kind: MobKind,
    pub entity_runtime_id: u64,
    pub position: [f32; 3],
    pub motion: [f32; 3],
    pub health: f32,
    pub goal: AiGoal,
    /// Ticks depuis dernière action d'IA majeure.
    pub goal_tick: u32,
    /// Cooldown d'attaque en ticks.
    pub attack_cooldown: u32,
}

impl MobAi {
    pub fn new(kind: MobKind, entity_runtime_id: u64, position: [f32; 3]) -> Self {
        Self {
            kind,
            entity_runtime_id,
            position,
            motion: [0.0, 0.0, 0.0],
            health: kind.max_health(),
            goal: AiGoal::Idle,
            goal_tick: 0,
            attack_cooldown: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    /// Choisit un goal en fonction de l'environnement.
    /// `nearby_players`: liste (runtime_id, position) des joueurs proches.
    pub fn think(&mut self, nearby_players: &[(u64, [f32; 3])]) {
        if !self.is_alive() {
            return;
        }
        self.goal_tick = self.goal_tick.wrapping_add(1);
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }

        // Hostiles target le joueur le plus proche dans sight_range.
        if self.kind.is_hostile() {
            let range = self.kind.sight_range();
            let mut closest: Option<(u64, f32)> = None;
            for (pid, ppos) in nearby_players {
                let dx = ppos[0] - self.position[0];
                let dy = ppos[1] - self.position[1];
                let dz = ppos[2] - self.position[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist < range && closest.is_none_or(|(_, d)| dist < d) {
                    closest = Some((*pid, dist));
                }
            }
            if let Some((pid, dist)) = closest {
                self.goal = if dist < self.kind.attack_range() {
                    AiGoal::Attack {
                        target_runtime_id: pid,
                    }
                } else {
                    AiGoal::FollowTarget {
                        target_runtime_id: pid,
                    }
                };
                return;
            }
        }

        // Passifs fuient si blessés récemment.
        if self.kind.is_passive() && self.health < self.kind.max_health() * 0.3 {
            if let Some((pid, _)) = nearby_players.first() {
                self.goal = AiGoal::Flee {
                    from_runtime_id: *pid,
                };
                return;
            }
        }

        // Idle/Wander par défaut.
        if self.goal_tick > 40 && matches!(self.goal, AiGoal::Idle | AiGoal::Wander) {
            self.goal = AiGoal::Wander;
            self.goal_tick = 0;
        }
    }

    /// Appliqué chaque tick pour bouger le mob selon son goal.
    /// Retourne `Some(attack_request)` si mob veut attaquer ce tick.
    pub fn tick_motion(&mut self, nearby_players: &[(u64, [f32; 3])]) -> Option<MobAttackRequest> {
        if !self.is_alive() {
            return None;
        }
        match &self.goal {
            AiGoal::FollowTarget { target_runtime_id } => {
                if let Some((_, ppos)) = nearby_players
                    .iter()
                    .find(|(pid, _)| pid == target_runtime_id)
                {
                    let dx = ppos[0] - self.position[0];
                    let dz = ppos[2] - self.position[2];
                    let dist = (dx * dx + dz * dz).sqrt().max(0.001);
                    let speed = 0.15;
                    self.motion[0] = dx / dist * speed;
                    self.motion[2] = dz / dist * speed;
                    self.position[0] += self.motion[0];
                    self.position[2] += self.motion[2];
                }
                None
            }
            AiGoal::Attack { target_runtime_id } => {
                if self.attack_cooldown == 0 {
                    self.attack_cooldown = 20;
                    return Some(MobAttackRequest {
                        target_runtime_id: *target_runtime_id,
                        damage: self.kind.attack_damage(),
                        cause: DamageCause::EntityAttack,
                    });
                }
                None
            }
            AiGoal::Flee { from_runtime_id } => {
                if let Some((_, ppos)) = nearby_players
                    .iter()
                    .find(|(pid, _)| pid == from_runtime_id)
                {
                    let dx = self.position[0] - ppos[0];
                    let dz = self.position[2] - ppos[2];
                    let dist = (dx * dx + dz * dz).sqrt().max(0.001);
                    let speed = 0.25;
                    self.motion[0] = dx / dist * speed;
                    self.motion[2] = dz / dist * speed;
                    self.position[0] += self.motion[0];
                    self.position[2] += self.motion[2];
                }
                None
            }
            AiGoal::Wander => {
                // Movement pseudo-random basique (déterministe via goal_tick).
                let n = self.goal_tick as f32 * 0.1;
                self.motion[0] = n.sin() * 0.05;
                self.motion[2] = n.cos() * 0.05;
                self.position[0] += self.motion[0];
                self.position[2] += self.motion[2];
                None
            }
            _ => None,
        }
    }
}

pub struct MobAttackRequest {
    pub target_runtime_id: u64,
    pub damage: f32,
    pub cause: DamageCause,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombie_targets_nearby_player() {
        let mut z = MobAi::new(MobKind::Zombie, 1, [0.0, 64.0, 0.0]);
        let players = vec![(42u64, [5.0, 64.0, 0.0])];
        z.think(&players);
        // Player at distance 5, zombie sight=16, attack_range=2.
        // Should be FollowTarget (too far to attack).
        assert!(matches!(
            z.goal,
            AiGoal::FollowTarget {
                target_runtime_id: 42
            }
        ));
    }

    #[test]
    fn zombie_attacks_close_player() {
        let mut z = MobAi::new(MobKind::Zombie, 1, [0.0, 64.0, 0.0]);
        let players = vec![(42u64, [1.0, 64.0, 0.0])];
        z.think(&players);
        assert!(matches!(z.goal, AiGoal::Attack { .. }));
        let attack = z.tick_motion(&players);
        assert!(attack.is_some());
    }

    #[test]
    fn passive_mob_flees_when_hurt() {
        let mut cow = MobAi::new(MobKind::Cow, 1, [0.0, 64.0, 0.0]);
        cow.health = 2.0; // 30% de max = 3 ; set à 2 pour < 30%.
        let players = vec![(42u64, [5.0, 64.0, 0.0])];
        cow.think(&players);
        assert!(matches!(cow.goal, AiGoal::Flee { .. }));
    }

    #[test]
    fn sheep_stats() {
        let s = MobKind::Sheep;
        assert_eq!(s.max_health(), 10.0);
        assert!(s.is_passive());
    }

    #[test]
    fn enderman_large_sight() {
        assert_eq!(MobKind::Enderman.sight_range(), 64.0);
    }
}
