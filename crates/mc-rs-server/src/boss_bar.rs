//! Boss bar — wither/dragon/custom bars.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossBarDivision {
    None,
    Notches6,
    Notches10,
    Notches12,
    Notches20,
}

#[derive(Debug, Clone)]
pub struct BossBar {
    pub id: u64,
    pub title: String,
    pub color: BossBarColor,
    pub division: BossBarDivision,
    pub percent: f32, // 0.0-1.0
    pub darken_sky: bool,
    pub play_boss_music: bool,
    pub create_fog: bool,
    pub players: Vec<u64>,
}

impl BossBar {
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            color: BossBarColor::Purple,
            division: BossBarDivision::None,
            percent: 1.0,
            darken_sky: false,
            play_boss_music: false,
            create_fog: false,
            players: Vec::new(),
        }
    }

    pub fn add_player(&mut self, player: u64) {
        if !self.players.contains(&player) {
            self.players.push(player);
        }
    }

    pub fn remove_player(&mut self, player: u64) {
        self.players.retain(|&p| p != player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_then_remove() {
        let mut b = BossBar::new(1, "Test");
        b.add_player(42);
        b.remove_player(42);
        assert!(b.players.is_empty());
    }
}
