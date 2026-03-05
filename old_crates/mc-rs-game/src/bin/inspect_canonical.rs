use std::collections::BTreeMap;
use std::fs;

use mc_rs_nbt::{read_nbt_network, NbtTag};

fn tag_kind(tag: &NbtTag) -> &'static str {
    match tag {
        NbtTag::Byte(_) => "Byte",
        NbtTag::Short(_) => "Short",
        NbtTag::Int(_) => "Int",
        NbtTag::Long(_) => "Long",
        NbtTag::Float(_) => "Float",
        NbtTag::Double(_) => "Double",
        NbtTag::ByteArray(_) => "ByteArray",
        NbtTag::String(_) => "String",
        NbtTag::List(_) => "List",
        NbtTag::Compound(_) => "Compound",
        NbtTag::IntArray(_) => "IntArray",
        NbtTag::LongArray(_) => "LongArray",
    }
}

fn main() {
    let path = ".reference/canonical_block_states.nbt";
    let data = fs::read(path).expect("failed to read canonical_block_states.nbt");
    let mut target_names: BTreeMap<String, bool> = BTreeMap::new();
    target_names.insert("minecraft:air".to_string(), false);
    target_names.insert("minecraft:bedrock".to_string(), false);
    target_names.insert("minecraft:dirt".to_string(), false);
    target_names.insert("minecraft:grass_block".to_string(), false);

    let mut buf = &data[..];
    let mut scanned = 0usize;
    while !buf.is_empty() {
        let root = match read_nbt_network(&mut buf) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("parse error after {scanned} entries: {e}");
                break;
            }
        };

        let Some(name) = root
            .compound
            .get("name")
            .and_then(|t| t.as_string())
            .map(str::to_owned)
        else {
            continue;
        };
        scanned += 1;

        if target_names.contains_key(&name) {
            target_names.insert(name.clone(), true);
            println!("\nFound {name} at entry #{scanned}:");
            if let Some(states) = root.compound.get("states").and_then(|t| t.as_compound()) {
                println!("  states:");
                if states.is_empty() {
                    println!("    (empty)");
                } else {
                    let mut keys: Vec<_> = states.keys().cloned().collect();
                    keys.sort();
                    for sk in keys {
                        let sv = states.get(&sk).unwrap();
                        match sv {
                            NbtTag::Byte(v) => println!("    {sk}: Byte({v})"),
                            NbtTag::Int(v) => println!("    {sk}: Int({v})"),
                            NbtTag::String(v) => println!("    {sk}: String({v})"),
                            _ => println!("    {sk}: {}", tag_kind(sv)),
                        }
                    }
                }
            }
            if let Some(version) = root.compound.get("version").and_then(|t| t.as_int()) {
                println!("  version: {version}");
            }
        }
    }

    println!("\nScanned compound entries: {scanned}");
    println!("Found targets:");
    for (name, found) in target_names {
        println!("  - {name}: {found}");
    }
}
