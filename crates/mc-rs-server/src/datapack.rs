//! Datapack / behavior pack abstraction.

#[derive(Debug, Clone)]
pub struct DataPack {
    pub name: String,
    pub description: String,
    pub version: [u8; 3],
    pub uuid: String,
    pub is_behavior: bool,
    pub min_engine_version: [u8; 3],
    pub header: PackHeader,
}

#[derive(Debug, Clone)]
pub struct PackHeader {
    pub name: String,
    pub uuid: String,
    pub version: [u8; 3],
    pub min_engine_version: [u8; 3],
}

/// Module types in datapacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
    ResourcePack, // resource_pack (skin/model)
    BehaviorPack, // rules / entity / command
    SkinPack,
    WorldTemplate,
    Data,
    InterfacePack,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_equality() {
        assert_eq!(ModuleKind::BehaviorPack, ModuleKind::BehaviorPack);
    }
}
