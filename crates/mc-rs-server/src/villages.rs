//! Villages — port conceptuel PMMP (limité). Mechanics de village :
//! center, bells, profession_blocks, gossips, reputation.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfessionBlock {
    /// Composter → Farmer
    Composter,
    /// Barrel → Fisherman
    Barrel,
    /// Loom → Shepherd
    Loom,
    /// FletchingTable → Fletcher
    FletchingTable,
    /// Lectern → Librarian
    Lectern,
    /// CartographyTable → Cartographer
    CartographyTable,
    /// BrewingStand → Cleric
    BrewingStand,
    /// BlastFurnace → Armorer
    BlastFurnace,
    /// Grindstone → WeaponSmith
    Grindstone,
    /// SmithingTable → ToolSmith
    SmithingTable,
    /// SmoKer → Butcher
    Smoker,
    /// Cauldron → Leatherworker
    Cauldron,
    /// Stonecutter → Mason
    Stonecutter,
}

impl ProfessionBlock {
    pub fn block_name(&self) -> &'static str {
        match self {
            Self::Composter => "minecraft:composter",
            Self::Barrel => "minecraft:barrel",
            Self::Loom => "minecraft:loom",
            Self::FletchingTable => "minecraft:fletching_table",
            Self::Lectern => "minecraft:lectern",
            Self::CartographyTable => "minecraft:cartography_table",
            Self::BrewingStand => "minecraft:brewing_stand",
            Self::BlastFurnace => "minecraft:blast_furnace",
            Self::Grindstone => "minecraft:grindstone",
            Self::SmithingTable => "minecraft:smithing_table",
            Self::Smoker => "minecraft:smoker",
            Self::Cauldron => "minecraft:cauldron",
            Self::Stonecutter => "minecraft:stonecutter",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Village {
    pub id: u64,
    pub center: [i32; 3],
    pub radius: i32, // blocks
    pub bell_position: Option<[i32; 3]>,
    pub villager_count: u32,
    pub iron_golem_count: u32,
    pub cat_count: u32,
}

impl Village {
    pub fn new(id: u64, center: [i32; 3]) -> Self {
        Self {
            id,
            center,
            radius: 32,
            bell_position: None,
            villager_count: 0,
            iron_golem_count: 0,
            cat_count: 0,
        }
    }

    pub fn contains(&self, pos: [i32; 3]) -> bool {
        let dx = (pos[0] - self.center[0]).abs();
        let dy = (pos[1] - self.center[1]).abs();
        let dz = (pos[2] - self.center[2]).abs();
        dx <= self.radius && dz <= self.radius && dy <= self.radius * 2
    }

    /// Un iron golem peut spawn si villager_count > 10 et iron_golem_count < villager/10.
    pub fn should_spawn_iron_golem(&self) -> bool {
        self.villager_count > 10 && self.iron_golem_count < self.villager_count / 10
    }
}

#[derive(Debug, Default)]
pub struct VillageRegistry {
    pub villages: HashMap<u64, Village>,
    next_id: u64,
}

impl VillageRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, center: [i32; 3]) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.villages.insert(id, Village::new(id, center));
        id
    }

    pub fn find_containing(&self, pos: [i32; 3]) -> Option<&Village> {
        self.villages.values().find(|v| v.contains(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn village_contains_position() {
        let v = Village::new(1, [100, 64, 100]);
        assert!(v.contains([105, 64, 105]));
        assert!(!v.contains([200, 64, 200]));
    }

    #[test]
    fn iron_golem_threshold() {
        let mut v = Village::new(1, [0, 64, 0]);
        v.villager_count = 20;
        v.iron_golem_count = 1;
        assert!(v.should_spawn_iron_golem());
        v.iron_golem_count = 2;
        assert!(!v.should_spawn_iron_golem());
    }
}
