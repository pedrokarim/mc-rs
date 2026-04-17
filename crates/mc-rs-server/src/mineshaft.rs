//! Mineshaft — port conceptuel. Structures souterraines aléatoires avec rails,
//! cobweb, chests, spawner, torches.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MineshaftKind {
    Normal, // wooden supports
    Mesa,   // dark oak variant
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MineshaftPiece {
    Room,
    Corridor,
    Stairs,
    Intersection,
    EndHole, // natural cave hole
}

/// Longueur moyenne d'une mineshaft (blocs Manhattan).
pub fn average_length(kind: MineshaftKind) -> u32 {
    match kind {
        MineshaftKind::Normal => 256,
        MineshaftKind::Mesa => 384,
    }
}

/// Chance de générer un chest par piece.
pub fn chest_chance(_piece: MineshaftPiece) -> f32 {
    0.5 / 10.0 // 5% per piece ≈ 5-10 chests par mineshaft
}

/// Chance de spawner (cave spider) sur un piece.
pub fn spawner_chance(piece: MineshaftPiece) -> f32 {
    match piece {
        MineshaftPiece::Room | MineshaftPiece::Intersection => 0.15,
        _ => 0.0,
    }
}

/// Liste loot pool mineshaft chest (noms items).
pub fn mineshaft_chest_loot() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:rail", 20),
        ("minecraft:powered_rail", 6),
        ("minecraft:detector_rail", 5),
        ("minecraft:torch", 15),
        ("minecraft:bread", 15),
        ("minecraft:coal", 15),
        ("minecraft:wheat", 15),
        ("minecraft:iron_ingot", 10),
        ("minecraft:gold_ingot", 5),
        ("minecraft:diamond", 3),
        ("minecraft:gold_pickaxe", 1),
        ("minecraft:enchanted_golden_apple", 1),
        ("minecraft:lapis_lazuli", 5),
        ("minecraft:redstone", 5),
        ("minecraft:melon_seeds", 5),
        ("minecraft:pumpkin_seeds", 5),
        ("minecraft:beetroot_seeds", 5),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mineshaft_chest_has_items() {
        assert!(!mineshaft_chest_loot().is_empty());
    }

    #[test]
    fn mesa_longer_than_normal() {
        assert!(average_length(MineshaftKind::Mesa) > average_length(MineshaftKind::Normal));
    }
}
