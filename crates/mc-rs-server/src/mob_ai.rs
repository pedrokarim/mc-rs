//! Registre des types de mobs vanilla (`MobKind`) et leurs statistiques
//! (santé, dégâts, portées, hostilité). Utilisé par les œufs de spawn, le
//! breeding, les règles de spawn, etc.
//!
//! NB : l'**IA** des mobs (sensors, behaviors, navigation) vit désormais dans
//! [`crate::ai`] (framework générique inspiré d'Allay) et opère sur les entités
//! vivantes [`crate::mob_entities::MobEntity`]. L'ancienne logique IA parallèle
//! (struct `MobAi`) a été retirée pour éviter deux systèmes concurrents.

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

#[cfg(test)]
mod tests {
    use super::*;

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
