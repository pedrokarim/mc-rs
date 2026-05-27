//! Entity persistence flags.

#[derive(Debug, Clone, Copy, Default)]
pub struct PersistenceFlags {
    pub persistent: bool,   // Never despawns
    pub from_spawner: bool, // From mob spawner (despawns quickly)
    pub from_bucket: bool,  // Axolotl/fish from bucket (persistent)
    pub has_custom_name: bool,
    pub is_pet: bool, // Tamed
    pub is_leashed: bool,
}

impl PersistenceFlags {
    /// Should mob persist through despawn?
    pub fn should_persist(&self) -> bool {
        self.persistent
            || self.from_bucket
            || self.has_custom_name
            || self.is_pet
            || self.is_leashed
    }

    /// Should mob despawn more aggressively (from spawner)?
    pub fn aggressive_despawn(&self) -> bool {
        self.from_spawner && !self.should_persist()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_persists() {
        let mut f = PersistenceFlags::default();
        f.has_custom_name = true;
        assert!(f.should_persist());
    }
}
