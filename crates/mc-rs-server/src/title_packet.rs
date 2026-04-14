//! Title / subtitle / action bar.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleKind {
    Clear,
    Reset,
    Title,
    Subtitle,
    ActionBar,
    Times,
    TitleJson,
    SubtitleJson,
    ActionBarJson,
}

#[derive(Debug, Clone)]
pub struct TitleSettings {
    pub fade_in_ticks: u32,
    pub stay_ticks: u32,
    pub fade_out_ticks: u32,
}

impl Default for TitleSettings {
    fn default() -> Self {
        Self {
            fade_in_ticks: 10,
            stay_ticks: 40,
            fade_out_ticks: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TitleDisplay {
    pub title: String,
    pub subtitle: String,
    pub action_bar: String,
    pub settings: TitleSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_times() {
        let s = TitleSettings::default();
        assert_eq!(s.fade_in_ticks, 10);
    }
}
