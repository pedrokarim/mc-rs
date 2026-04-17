//! Furnace tick — port PMMP `src/block/tile/Furnace.php::onUpdate()`.

use crate::crafting::{CraftingManager, FurnaceRecipe};
use mc_rs_proto::packets::player::ItemStack;

/// Fuel items avec leur durée de burn (ticks).
pub fn fuel_burn_time(item_name: &str) -> u32 {
    match item_name {
        "minecraft:lava_bucket" => 20000,
        "minecraft:coal_block" => 16000,
        "minecraft:blaze_rod" => 2400,
        "minecraft:coal" | "minecraft:charcoal" => 1600,
        "minecraft:boat" | "minecraft:oak_boat" | "minecraft:birch_boat" => 1200,
        "minecraft:oak_log"
        | "minecraft:birch_log"
        | "minecraft:spruce_log"
        | "minecraft:jungle_log"
        | "minecraft:acacia_log"
        | "minecraft:dark_oak_log" => 300,
        "minecraft:oak_planks"
        | "minecraft:birch_planks"
        | "minecraft:spruce_planks"
        | "minecraft:jungle_planks"
        | "minecraft:acacia_planks"
        | "minecraft:dark_oak_planks" => 300,
        "minecraft:stick" => 100,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnaceKind {
    Normal,
    BlastFurnace,
    Smoker,
}

impl FurnaceKind {
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::BlastFurnace | Self::Smoker => 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FurnaceState {
    pub kind: FurnaceKind,
    pub input: ItemStack,
    pub fuel: ItemStack,
    pub output: ItemStack,
    /// Ticks restants avant épuisement du fuel actuel.
    pub burn_time_remaining: u32,
    /// Ticks depuis le début de la cuisson actuelle.
    pub cook_time: u32,
    /// Temps total nécessaire pour cette recette.
    pub cook_time_total: u32,
}

impl FurnaceState {
    pub fn new(kind: FurnaceKind) -> Self {
        Self {
            kind,
            input: ItemStack::AIR,
            fuel: ItemStack::AIR,
            output: ItemStack::AIR,
            burn_time_remaining: 0,
            cook_time: 0,
            cook_time_total: 0,
        }
    }

    pub fn is_burning(&self) -> bool {
        self.burn_time_remaining > 0
    }

    /// Tick (20 TPS).
    pub fn tick(&mut self, manager: &CraftingManager) -> FurnaceTickResult {
        let mut result = FurnaceTickResult::default();
        // Match recipe selon kind.
        let recipe: Option<&FurnaceRecipe> = match self.kind {
            FurnaceKind::Normal => manager.match_furnace(&self.input),
            FurnaceKind::BlastFurnace => manager.match_blast(&self.input),
            FurnaceKind::Smoker => manager.match_smoker(&self.input),
        };

        // Si pas de recipe valide, reset cook_time.
        if recipe.is_none() {
            if self.cook_time > 0 {
                self.cook_time = 0;
                self.cook_time_total = 0;
            }
            // burn fuel anyway
            if self.burn_time_remaining > 0 {
                self.burn_time_remaining -= 1;
            }
            return result;
        }

        let recipe = recipe.unwrap();
        if self.cook_time_total == 0 {
            self.cook_time_total =
                (recipe.cook_time_ticks as f32 / self.kind.speed_multiplier()) as u32;
        }

        // Consomme fuel si nécessaire.
        if self.burn_time_remaining == 0 && !self.fuel.is_air() {
            let fuel_name = crate::item_registry::item_name_by_id(self.fuel.id)
                .map(String::from)
                .unwrap_or_default();
            let burn = fuel_burn_time(&fuel_name);
            if burn > 0 {
                self.burn_time_remaining = burn;
                self.fuel.count -= 1;
                if self.fuel.count == 0 {
                    self.fuel = ItemStack::AIR;
                }
            }
        }

        if self.burn_time_remaining > 0 {
            self.burn_time_remaining -= 1;
            self.cook_time += 1;

            if self.cook_time >= self.cook_time_total {
                // Finish cooking.
                self.cook_time = 0;
                self.cook_time_total = 0;
                self.input.count -= 1;
                if self.input.count == 0 {
                    self.input = ItemStack::AIR;
                }
                if self.output.is_air() {
                    self.output = recipe.output.clone();
                } else if self.output.id == recipe.output.id {
                    self.output.count += recipe.output.count;
                }
                result.completed = true;
                result.xp_to_give = recipe.xp;
            }
        }
        result
    }
}

#[derive(Debug, Clone, Default)]
pub struct FurnaceTickResult {
    pub completed: bool,
    pub xp_to_give: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coal_burns_1600_ticks() {
        assert_eq!(fuel_burn_time("minecraft:coal"), 1600);
    }

    #[test]
    fn blast_furnace_2x_speed() {
        assert_eq!(FurnaceKind::BlastFurnace.speed_multiplier(), 2.0);
    }

    #[test]
    fn lava_bucket_is_strongest_fuel() {
        assert!(fuel_burn_time("minecraft:lava_bucket") > fuel_burn_time("minecraft:coal"));
    }
}
