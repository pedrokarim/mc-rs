//! Noteblock — port PMMP `src/block/NoteBlock.php`.
//! Joue une note selon l'instrument sous le bloc + pitch (0-24).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Instrument {
    Harp = 0,         // over air/other
    DoubleBass = 1,   // over wood
    Snare = 2,        // over sand
    Sticks = 3,       // over glass
    BassDrum = 4,     // over stone
    Bell = 5,         // over gold
    Flute = 6,        // over clay
    Chime = 7,        // over packed_ice
    Guitar = 8,       // over wool
    Xylophone = 9,    // over bone_block
    IronXylophone = 10, // over iron_block
    CowBell = 11,     // over soul_sand
    Didgeridoo = 12,  // over pumpkin
    Bit = 13,         // over emerald_block
    Banjo = 14,       // over hay_block
    Pling = 15,       // over glowstone
}

impl Instrument {
    /// Identifie l'instrument selon le bloc sous le noteblock.
    pub fn from_block_below(block_name: &str) -> Self {
        match block_name {
            "minecraft:oak_wood" | "minecraft:birch_wood" | "minecraft:planks"
            | "minecraft:log" | "minecraft:crafting_table" | "minecraft:chest"
            | "minecraft:oak_planks" | "minecraft:birch_planks"
            | "minecraft:spruce_planks" | "minecraft:jungle_planks" => Self::DoubleBass,
            "minecraft:sand" | "minecraft:red_sand" | "minecraft:gravel" => Self::Snare,
            "minecraft:glass" | "minecraft:tinted_glass" => Self::Sticks,
            "minecraft:stone" | "minecraft:cobblestone" | "minecraft:deepslate"
            | "minecraft:andesite" | "minecraft:diorite" | "minecraft:granite"
            | "minecraft:bedrock" => Self::BassDrum,
            "minecraft:gold_block" => Self::Bell,
            "minecraft:clay" => Self::Flute,
            "minecraft:packed_ice" => Self::Chime,
            "minecraft:wool" => Self::Guitar,
            "minecraft:bone_block" => Self::Xylophone,
            "minecraft:iron_block" => Self::IronXylophone,
            "minecraft:soul_sand" => Self::CowBell,
            "minecraft:pumpkin" => Self::Didgeridoo,
            "minecraft:emerald_block" => Self::Bit,
            "minecraft:hay_block" => Self::Banjo,
            "minecraft:glowstone" => Self::Pling,
            _ => Self::Harp,
        }
    }
}

/// State d'un noteblock : pitch 0-24.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoteBlockState {
    pub pitch: u8, // 0-24
}

impl NoteBlockState {
    /// Increment pitch (rotate 0→24→0).
    pub fn increment(&mut self) {
        self.pitch = (self.pitch + 1) % 25;
    }

    /// Fréquence en Hz pour ce pitch.
    pub fn frequency(&self) -> f32 {
        // Formule PMMP `NoteBlock::getFrequency()` : F#3 → F#5.
        // frequency = 2^((pitch - 12) / 12) * 440
        2.0f32.powf((self.pitch as f32 - 12.0) / 12.0) * 440.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_is_bass_drum() {
        assert_eq!(Instrument::from_block_below("minecraft:stone"), Instrument::BassDrum);
    }

    #[test]
    fn pitch_wraps_at_24() {
        let mut n = NoteBlockState::default();
        for _ in 0..25 {
            n.increment();
        }
        assert_eq!(n.pitch, 0);
    }

    #[test]
    fn middle_pitch_is_440hz() {
        let n = NoteBlockState { pitch: 12 };
        assert!((n.frequency() - 440.0).abs() < 0.01);
    }
}
