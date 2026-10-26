//! Bloc écrit avec ses états, syntaxe des commandes de Bedrock : `minecraft:wheat ["growth"=7]`.
//!
//! Le découpage des commandes retire les guillemets et sépare ce qui suit une espace : `wheat[growth=7]`
//! est donc accepté aussi, et les commandes recollent un argument qui commence par `[` au nom qui le
//! précède (`join_block_states`). Les états non précisés prennent la valeur de l'état par défaut du bloc
//! (`BLOCKS.get`). Les états booléens s'écrivent `true` / `false` (comme chez BDS) ou `1` / `0`.
//!
//! Palette lue dans `data/canonical_block_states.nbt` (indice = runtime ID).

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use mc_rs_nbt::NbtTag;

use crate::world::block_registry::BLOCKS;

/// Valeur d'un état de la palette.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StateValue {
    Byte(i8),
    Int(i32),
    String(String),
}

struct Palette {
    states: Vec<BTreeMap<String, StateValue>>,
    by_name: HashMap<String, Vec<u32>>,
}

static PALETTE: LazyLock<Palette> = LazyLock::new(|| {
    let bytes = include_bytes!("../../data/canonical_block_states.nbt");
    let mut buf = &bytes[..];
    let mut palette = Palette {
        states: Vec::new(),
        by_name: HashMap::new(),
    };
    while !buf.is_empty() {
        let root =
            mc_rs_nbt::read_nbt_network(&mut buf).expect("canonical_block_states.nbt valide");
        let runtime_id = palette.states.len() as u32;
        let name = match root.compound.get("name") {
            Some(NbtTag::String(name)) => name.clone(),
            _ => panic!("état {runtime_id} sans nom"),
        };
        let mut states = BTreeMap::new();
        if let Some(NbtTag::Compound(raw)) = root.compound.get("states") {
            for (key, value) in raw {
                let value = match value {
                    NbtTag::Byte(v) => StateValue::Byte(*v),
                    NbtTag::Int(v) => StateValue::Int(*v),
                    NbtTag::String(v) => StateValue::String(v.clone()),
                    other => panic!(
                        "{name} : état {key} de type {} non géré",
                        other.tag_type_id()
                    ),
                };
                states.insert(key.clone(), value);
            }
        }
        palette.states.push(states);
        palette.by_name.entry(name).or_default().push(runtime_id);
    }
    palette
});

/// Compare une valeur écrite dans une commande à celle d'un état.
fn matches(written: &str, value: &StateValue) -> bool {
    match value {
        StateValue::Byte(v) => match written {
            "true" => *v == 1,
            "false" => *v == 0,
            other => other.parse::<i8>().is_ok_and(|n| n == *v),
        },
        StateValue::Int(v) => written.parse::<i32>().is_ok_and(|n| n == *v),
        StateValue::String(v) => written == v,
    }
}

/// Runtime ID d'un bloc écrit `nom` ou `nom[clé=valeur,…]` (préfixe `minecraft:` facultatif). `None` si le
/// nom, une clé ou une valeur est inconnu.
pub fn resolve(spec: &str) -> Option<u32> {
    let spec = spec.trim();
    let (name, states) = match spec.split_once('[') {
        Some((name, rest)) => (name.trim(), Some(rest.trim().strip_suffix(']')?)),
        None => (spec, None),
    };
    let name = if name.contains(':') {
        name.to_string()
    } else {
        format!("minecraft:{name}")
    };
    let candidates = PALETTE.by_name.get(&name)?;
    let default = BLOCKS.get(&name);
    let Some(states) = states.filter(|s| !s.trim().is_empty()) else {
        return Some(default);
    };

    let defaults = &PALETTE.states[default as usize];
    let mut written = BTreeMap::new();
    for pair in states.split(',') {
        let (key, value) = pair.split_once('=')?;
        let key = key.trim().trim_matches('"').to_string();
        let value = value.trim().trim_matches('"').to_string();
        if !defaults.contains_key(&key) {
            return None;
        }
        written.insert(key, value);
    }
    candidates.iter().copied().find(|&id| {
        PALETTE.states[id as usize]
            .iter()
            .all(|(key, value)| match written.get(key) {
                Some(text) => matches(text, value),
                None => defaults.get(key) == Some(value),
            })
    })
}

/// Recolle au nom de bloc d'indice `index` un argument suivant qui commence par `[` (états écrits après
/// une espace, découpés à part par le parseur). Renvoie le bloc et le nombre d'arguments consommés.
pub fn join_block_states(args: &[String], index: usize) -> Option<(String, usize)> {
    let name = args.get(index)?;
    match args.get(index + 1) {
        Some(next) if next.starts_with('[') && !name.contains('[') => {
            Some((format!("{name}{next}"), 2))
        }
        _ => Some((name.clone(), 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_resolve_to_the_default_state() {
        assert_eq!(resolve("stone"), Some(BLOCKS.stone));
        assert_eq!(resolve("minecraft:stone"), Some(BLOCKS.stone));
        assert_eq!(resolve("minecraft:no_such_block"), None);
    }

    #[test]
    fn states_select_the_matching_runtime_id() {
        // Bougie rouge : 8 états qui se suivent, candles 0 à 3, allumée ou non.
        let first = BLOCKS.get("minecraft:red_candle");
        let three = resolve(r#"minecraft:red_candle["candles"=3]"#).unwrap();
        assert!((first..first + 8).contains(&three));
        assert_ne!(three, first);
        // Guillemets retirés par le parseur, espace avant les états, booléens en 0/1.
        assert_eq!(resolve("red_candle [candles=3,lit=false]"), Some(three));
        assert_eq!(resolve("red_candle[candles=3,lit=0]"), Some(three));
        assert_ne!(resolve("red_candle[candles=3,lit=true]"), Some(three));
        assert_eq!(resolve("red_candle[]"), Some(first));
    }

    #[test]
    fn unknown_states_are_rejected() {
        assert_eq!(resolve("red_candle[candles=9]"), None);
        assert_eq!(resolve("red_candle[colour=red]"), None);
        assert_eq!(resolve("red_candle[candles=3"), None);
    }

    #[test]
    fn states_written_after_a_space_are_joined() {
        let args: Vec<String> = ["0", "minecraft:wheat", "[growth=7]", "replace"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            join_block_states(&args, 1),
            Some(("minecraft:wheat[growth=7]".to_string(), 2))
        );
        assert_eq!(
            join_block_states(&args, 3),
            Some(("replace".to_string(), 1))
        );
    }
}
