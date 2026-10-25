//! Garnitures d'armure (paquet TrimData) : motifs et matériaux de BDS,
//! importés de sa capture (`data/trim_data.json`, produit par
//! `scripts/import_bds_trim_data.py` ; méthode :
//! `dev-docs/34-BDS-RETRO-INGENIERIE.md`). Le client s'en sert pour la table
//! de forge et l'affichage des armures garnies.

use std::sync::LazyLock;

use mc_rs_proto::packets::world::{TrimData, TrimMaterial, TrimPattern};
use serde::Deserialize;

const TRIM_DATA_JSON: &str = include_str!("../data/trim_data.json");

#[derive(Deserialize)]
struct RawTrimData {
    patterns: Vec<RawPattern>,
    materials: Vec<RawMaterial>,
}

#[derive(Deserialize)]
struct RawPattern {
    item: String,
    pattern: String,
}

#[derive(Deserialize)]
struct RawMaterial {
    material: String,
    colour: String,
    item: String,
}

/// Payload du paquet TrimData, encodé une fois.
static PAYLOAD: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let raw: RawTrimData = serde_json::from_str(TRIM_DATA_JSON).expect("trim_data.json valide");
    TrimData {
        patterns: raw
            .patterns
            .into_iter()
            .map(|p| TrimPattern {
                item: p.item,
                pattern: p.pattern,
            })
            .collect(),
        materials: raw
            .materials
            .into_iter()
            .map(|m| TrimMaterial {
                material: m.material,
                colour: m.colour,
                item: m.item,
            })
            .collect(),
    }
    .encode()
});

pub fn payload() -> &'static [u8] {
    &PAYLOAD
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_rs_proto::io::ProtoReader;

    #[test]
    fn payload_lists_bds_patterns_and_materials() {
        let mut r = ProtoReader::new(payload());
        assert_eq!(r.read_var_u32().unwrap(), 18);
        assert_eq!(
            r.read_string().unwrap(),
            "minecraft:ward_armor_trim_smithing_template"
        );
        assert_eq!(r.read_string().unwrap(), "ward");
        for _ in 1..18 {
            r.read_string().unwrap();
            r.read_string().unwrap();
        }
        assert_eq!(r.read_var_u32().unwrap(), 11);
        assert_eq!(r.read_string().unwrap(), "quartz");
        assert_eq!(r.read_string().unwrap(), "§h");
        assert_eq!(r.read_string().unwrap(), "minecraft:quartz");
        for _ in 1..11 {
            for _ in 0..3 {
                r.read_string().unwrap();
            }
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn every_trim_item_exists() {
        let raw: RawTrimData = serde_json::from_str(TRIM_DATA_JSON).unwrap();
        for item in raw
            .patterns
            .iter()
            .map(|p| &p.item)
            .chain(raw.materials.iter().map(|m| &m.item))
        {
            assert!(
                crate::item_registry::network_id(item).is_some(),
                "{item} absent du registre d'items"
            );
        }
    }
}
