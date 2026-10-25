//! Blocs vanilla « data-driven » (Bedrock 1.26.50+) : dalles et escaliers de
//! béton et de laine, `red_shrub`, `shelf_mushroom`.
//!
//! Le jeu les définit par composants, comme des blocs personnalisés : le client
//! ne les ajoute à sa palette que si le serveur lui envoie leurs définitions
//! dans la liste `Block Properties` de StartGame (schéma Mojang
//! `ServerBlockProperty.json` : nom puis définition NBT). Sans elles, la palette
//! du client compte 98 blocs de moins que la nôtre et tous les runtime IDs
//! séquentiels qui suivent le premier de ces blocs sont décalés.
//!
//! Source : dragonfly v0.11.5 `server/world/data_driven_blocks.nbt` (NBT
//! little-endian `{blocks: [{name, components}]}`), envoyé tel quel par
//! `Server.makeBlockEntries` dans le même ordre ; gophertunnel `BlockEntry`
//! (`String` + NBT réseau).

use std::sync::LazyLock;

use mc_rs_nbt::tag::{NbtRoot, NbtTag};
use mc_rs_proto::io::ProtoWriter;

const DATA_DRIVEN_BLOCKS_NBT: &[u8] = include_bytes!("../../data/data_driven_blocks.nbt");

/// Liste `Block Properties` encodée (VarUInt32 + entrées), prête pour StartGame.
static BLOCK_PROPERTIES: LazyLock<Vec<u8>> = LazyLock::new(encode_block_properties);

fn encode_block_properties() -> Vec<u8> {
    let root = mc_rs_nbt::read_nbt_le(&mut &DATA_DRIVEN_BLOCKS_NBT[..])
        .expect("data_driven_blocks.nbt valide");
    let Some(NbtTag::List(blocks)) = root.compound.get("blocks") else {
        panic!("data_driven_blocks.nbt : liste `blocks` absente");
    };
    let mut w = ProtoWriter::with_capacity(DATA_DRIVEN_BLOCKS_NBT.len() + 64);
    w.write_var_u32(blocks.len() as u32);
    for block in blocks {
        let NbtTag::Compound(block) = block else {
            panic!("data_driven_blocks.nbt : entrée de `blocks` non compound");
        };
        let Some(NbtTag::String(name)) = block.get("name") else {
            panic!("data_driven_blocks.nbt : bloc sans `name`");
        };
        let Some(NbtTag::Compound(components)) = block.get("components") else {
            panic!("data_driven_blocks.nbt : {name} sans `components`");
        };
        w.write_string(name);
        let mut nbt = Vec::new();
        mc_rs_nbt::write_nbt_network(
            &mut nbt,
            &NbtRoot {
                name: String::new(),
                compound: components.clone(),
            },
        );
        w.write_raw(&nbt);
    }
    w.into_bytes()
}

/// Payload de la liste `Block Properties` de StartGame.
pub fn block_properties() -> &'static [u8] {
    &BLOCK_PROPERTIES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::block_registry_data::BLOCK_NAME_TO_FIRST_RUNTIME_ID;
    use mc_rs_proto::io::ProtoReader;

    #[test]
    fn block_properties_list_every_data_driven_block() {
        let mut r = ProtoReader::new(block_properties());
        let count = r.read_var_u32().unwrap();
        assert_eq!(count, 98);
        for _ in 0..count {
            let name = r.read_string().unwrap();
            // Chaque bloc data-driven fait partie de la palette courante.
            assert!(
                BLOCK_NAME_TO_FIRST_RUNTIME_ID
                    .iter()
                    .any(|&(palette_name, _)| palette_name == name),
                "{name} absent de la palette"
            );
            // Définition NBT réseau, puis entrée suivante.
            let mut nbt = r.remaining_bytes();
            let before = nbt.len();
            let root = mc_rs_nbt::read_nbt_network(&mut nbt).unwrap();
            assert!(root.compound.contains_key("components"), "{name}");
            r.read_raw(before - nbt.len()).unwrap();
        }
        assert_eq!(r.remaining(), 0);
    }
}
