//! Jukebox — port PMMP `src/block/tile/Jukebox.php`.

use mc_rs_proto::packets::player::ItemStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicDiscKind {
    C418_13,
    C418_Cat,
    C418_Blocks,
    C418_Chirp,
    C418_Far,
    C418_Mall,
    C418_Mellohi,
    C418_Stal,
    C418_Strad,
    C418_Ward,
    C418_11,
    C418_Wait,
    C418_Pigstep,
    LenaRaine_Otherside,
    LenaRaine_5,
    S0undMagic_Relic,
}

impl MusicDiscKind {
    pub fn item_name(&self) -> &'static str {
        match self {
            Self::C418_13 => "minecraft:music_disc_13",
            Self::C418_Cat => "minecraft:music_disc_cat",
            Self::C418_Blocks => "minecraft:music_disc_blocks",
            Self::C418_Chirp => "minecraft:music_disc_chirp",
            Self::C418_Far => "minecraft:music_disc_far",
            Self::C418_Mall => "minecraft:music_disc_mall",
            Self::C418_Mellohi => "minecraft:music_disc_mellohi",
            Self::C418_Stal => "minecraft:music_disc_stal",
            Self::C418_Strad => "minecraft:music_disc_strad",
            Self::C418_Ward => "minecraft:music_disc_ward",
            Self::C418_11 => "minecraft:music_disc_11",
            Self::C418_Wait => "minecraft:music_disc_wait",
            Self::C418_Pigstep => "minecraft:music_disc_pigstep",
            Self::LenaRaine_Otherside => "minecraft:music_disc_otherside",
            Self::LenaRaine_5 => "minecraft:music_disc_5",
            Self::S0undMagic_Relic => "minecraft:music_disc_relic",
        }
    }

    pub fn duration_ticks(&self) -> u32 {
        match self {
            Self::C418_13 => 3585,
            Self::C418_Cat => 3700,
            Self::C418_Blocks => 5700,
            Self::C418_Chirp => 3700,
            Self::C418_Far => 3700,
            Self::C418_Mall => 3700,
            Self::C418_Mellohi => 1900,
            Self::C418_Stal => 3000,
            Self::C418_Strad => 3700,
            Self::C418_Ward => 5050,
            Self::C418_11 => 1420,
            Self::C418_Wait => 4650,
            Self::C418_Pigstep => 2600,
            Self::LenaRaine_Otherside => 4300,
            Self::LenaRaine_5 => 3800,
            Self::S0undMagic_Relic => 4600,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct JukeboxState {
    pub record: Option<MusicDiscKind>,
    pub position: [i32; 3],
    pub ticks_playing: u32,
}

impl JukeboxState {
    pub fn insert(&mut self, disc: MusicDiscKind) -> bool {
        if self.record.is_some() {
            return false;
        }
        self.record = Some(disc);
        self.ticks_playing = 0;
        true
    }

    pub fn take(&mut self) -> Option<MusicDiscKind> {
        let r = self.record.take();
        self.ticks_playing = 0;
        r
    }

    pub fn tick(&mut self) {
        if let Some(r) = self.record {
            self.ticks_playing = self.ticks_playing.wrapping_add(1);
            if self.ticks_playing >= r.duration_ticks() {
                self.ticks_playing = 0;
                self.record = None;
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        self.record.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_once_only() {
        let mut j = JukeboxState::default();
        assert!(j.insert(MusicDiscKind::C418_Cat));
        assert!(!j.insert(MusicDiscKind::C418_13));
    }

    #[test]
    fn take_clears_record() {
        let mut j = JukeboxState::default();
        j.insert(MusicDiscKind::C418_Cat);
        let taken = j.take();
        assert!(taken.is_some());
        assert!(j.record.is_none());
    }
}
