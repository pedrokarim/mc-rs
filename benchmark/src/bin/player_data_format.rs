//! Bench : JSON (notre implem actuelle) vs NBT big-endian + gzip (format PMMP).
//!
//! Deux profils de données joueur :
//! - `minimal`  — position + rotation + gamemode (ce qu'on sauve aujourd'hui)
//! - `full`     — + inventory 36 slots + skin 64x64 + geometry JSON (PMMP-like)
//!
//! Mesure : encode (serialize+compress), decode (decompress+parse), et taille
//! sérialisée finale. Chaque mesure est un median sur N runs warmup+mesure.
//!
//! Run : `cargo run --release -p benchmark --bin player_data_format`

use std::io::{Read, Write};

use benchmark::{bench as bench_fn, fmt_bytes};
use bytes::{Bytes, BytesMut};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_nbt::{read_nbt_be, write_nbt_be};
use serde::{Deserialize, Serialize};

const WARMUP: u32 = 200;
const RUNS: u32 = 2_000;

fn bench<F: FnMut()>(name: &str, f: F) {
    bench_fn(name, WARMUP, RUNS, f);
}

// ─────────────────────────── Modèles JSON ───────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct MinimalJson {
    pos: [f32; 3],
    motion: [f32; 3],
    rotation: [f32; 2],
    gamemode: i32,
    on_ground: bool,
    fall_distance: f32,
}

#[derive(Serialize, Deserialize, Clone)]
struct ItemJson {
    id: i32,
    count: i8,
    damage: i16,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkinJson {
    id: String,
    // base64 obligatoire car JSON n'a pas de bytes natifs.
    data_b64: String,
    cape_b64: String,
    geometry_name: String,
    geometry_json: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct FullJson {
    pos: [f32; 3],
    motion: [f32; 3],
    rotation: [f32; 2],
    gamemode: i32,
    on_ground: bool,
    fall_distance: f32,
    food_level: i32,
    food_exhaustion: f32,
    food_saturation: f32,
    xp_level: i32,
    xp_progress: f32,
    xp_total: i32,
    selected_slot: i32,
    level: String,
    last_known_xuid: String,
    first_played: i64,
    last_played: i64,
    inventory: Vec<ItemJson>,
    skin: SkinJson,
}

// ─────────────────────────── Builders ───────────────────────────

fn sample_minimal() -> MinimalJson {
    MinimalJson {
        pos: [123.456, 72.5, -98.7],
        motion: [0.0, -0.078, 0.0],
        rotation: [45.0, 12.5],
        gamemode: 0,
        on_ground: true,
        fall_distance: 0.0,
    }
}

fn sample_minimal_nbt() -> NbtRoot {
    let mut c = NbtCompound::new();
    c.insert(
        "Pos".into(),
        NbtTag::List(vec![
            NbtTag::Float(123.456),
            NbtTag::Float(72.5),
            NbtTag::Float(-98.7),
        ]),
    );
    c.insert(
        "Motion".into(),
        NbtTag::List(vec![
            NbtTag::Float(0.0),
            NbtTag::Float(-0.078),
            NbtTag::Float(0.0),
        ]),
    );
    c.insert(
        "Rotation".into(),
        NbtTag::List(vec![NbtTag::Float(45.0), NbtTag::Float(12.5)]),
    );
    c.insert("playerGameType".into(), NbtTag::Int(0));
    c.insert("OnGround".into(), NbtTag::Byte(1));
    c.insert("FallDistance".into(), NbtTag::Float(0.0));
    NbtRoot::new("", c)
}

fn sample_full() -> FullJson {
    // Skin 64x64x4 = 16 384 bytes (RGBA raw pixels).
    let skin_bytes: Vec<u8> = (0..16_384).map(|i| (i * 7 % 256) as u8).collect();
    let cape_bytes: Vec<u8> = (0..1_024).map(|i| (i * 11 % 256) as u8).collect();
    // Geometry = JSON ~30 KB (modèle de bones du joueur).
    let geometry_json = "{\"format_version\":\"1.12.0\",\"minecraft:geometry\":[{\"description\":{\"identifier\":\"geometry.humanoid.custom\",\"texture_width\":64,\"texture_height\":64,\"visible_bounds_width\":3,\"visible_bounds_height\":3.5}, \"bones\":[".to_string()
        + &(0..40)
            .map(|i| format!(
                "{{\"name\":\"bone_{i}\",\"pivot\":[0,0,0],\"cubes\":[{{\"origin\":[{i},0,0],\"size\":[1,1,1],\"uv\":[0,0]}}]}}"
            ))
            .collect::<Vec<_>>()
            .join(",")
        + "]}]}";

    let inventory: Vec<ItemJson> = (0..36)
        .map(|i| ItemJson {
            id: 256 + i,
            count: 64,
            damage: 0,
        })
        .collect();

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;

    FullJson {
        pos: [123.456, 72.5, -98.7],
        motion: [0.0, -0.078, 0.0],
        rotation: [45.0, 12.5],
        gamemode: 0,
        on_ground: true,
        fall_distance: 0.0,
        food_level: 20,
        food_exhaustion: 0.0,
        food_saturation: 5.0,
        xp_level: 30,
        xp_progress: 0.5,
        xp_total: 1395,
        selected_slot: 0,
        level: "world".into(),
        last_known_xuid: "2535438744446831".into(),
        first_played: 1_700_000_000_000,
        last_played: 1_712_345_678_900,
        inventory,
        skin: SkinJson {
            id: "Standard_Custom".into(),
            data_b64: b64.encode(&skin_bytes),
            cape_b64: b64.encode(&cape_bytes),
            geometry_name: "geometry.humanoid.custom".into(),
            geometry_json,
        },
    }
}

fn sample_full_nbt() -> NbtRoot {
    let skin_bytes: Vec<i8> = (0..16_384).map(|i| (i * 7 % 256) as i8).collect();
    let cape_bytes: Vec<i8> = (0..1_024).map(|i| (i * 11 % 256) as i8).collect();
    let geometry_json = "{\"format_version\":\"1.12.0\",\"minecraft:geometry\":[{\"description\":{\"identifier\":\"geometry.humanoid.custom\",\"texture_width\":64,\"texture_height\":64,\"visible_bounds_width\":3,\"visible_bounds_height\":3.5}, \"bones\":[".to_string()
        + &(0..40)
            .map(|i| format!(
                "{{\"name\":\"bone_{i}\",\"pivot\":[0,0,0],\"cubes\":[{{\"origin\":[{i},0,0],\"size\":[1,1,1],\"uv\":[0,0]}}]}}"
            ))
            .collect::<Vec<_>>()
            .join(",")
        + "]}]}";

    let mut root = NbtCompound::new();
    root.insert(
        "Pos".into(),
        NbtTag::List(vec![
            NbtTag::Float(123.456),
            NbtTag::Float(72.5),
            NbtTag::Float(-98.7),
        ]),
    );
    root.insert(
        "Motion".into(),
        NbtTag::List(vec![
            NbtTag::Float(0.0),
            NbtTag::Float(-0.078),
            NbtTag::Float(0.0),
        ]),
    );
    root.insert(
        "Rotation".into(),
        NbtTag::List(vec![NbtTag::Float(45.0), NbtTag::Float(12.5)]),
    );
    root.insert("playerGameType".into(), NbtTag::Int(0));
    root.insert("OnGround".into(), NbtTag::Byte(1));
    root.insert("FallDistance".into(), NbtTag::Float(0.0));
    root.insert("foodLevel".into(), NbtTag::Int(20));
    root.insert("foodExhaustionLevel".into(), NbtTag::Float(0.0));
    root.insert("foodSaturationLevel".into(), NbtTag::Float(5.0));
    root.insert("XpLevel".into(), NbtTag::Int(30));
    root.insert("XpP".into(), NbtTag::Float(0.5));
    root.insert("XpTotal".into(), NbtTag::Int(1395));
    root.insert("SelectedInventorySlot".into(), NbtTag::Int(0));
    root.insert("Level".into(), NbtTag::String("world".into()));
    root.insert(
        "LastKnownXUID".into(),
        NbtTag::String("2535438744446831".into()),
    );
    root.insert("firstPlayed".into(), NbtTag::Long(1_700_000_000_000));
    root.insert("lastPlayed".into(), NbtTag::Long(1_712_345_678_900));

    let inventory: Vec<NbtTag> = (0..36)
        .map(|i| {
            let mut item = NbtCompound::new();
            item.insert("id".into(), NbtTag::Int(256 + i));
            item.insert("Count".into(), NbtTag::Byte(64));
            item.insert("Damage".into(), NbtTag::Short(0));
            NbtTag::Compound(item)
        })
        .collect();
    root.insert("Inventory".into(), NbtTag::List(inventory));

    let mut skin = NbtCompound::new();
    skin.insert(
        "Name".into(),
        NbtTag::String("Standard_Custom".into()),
    );
    skin.insert("Data".into(), NbtTag::ByteArray(skin_bytes));
    skin.insert("CapeData".into(), NbtTag::ByteArray(cape_bytes));
    skin.insert(
        "GeometryName".into(),
        NbtTag::String("geometry.humanoid.custom".into()),
    );
    skin.insert("GeometryData".into(), NbtTag::String(geometry_json));
    root.insert("Skin".into(), NbtTag::Compound(skin));

    NbtRoot::new("", root)
}

// ─────────────────────────── Encodage / décodage ───────────────────────────

fn json_encode<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn json_decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> T {
    serde_json::from_slice(bytes).unwrap()
}

fn nbt_gzip_encode(root: &NbtRoot) -> Vec<u8> {
    let mut raw = BytesMut::new();
    write_nbt_be(&mut raw, root);
    let mut enc = GzEncoder::new(Vec::with_capacity(raw.len()), Compression::default());
    enc.write_all(&raw).unwrap();
    enc.finish().unwrap()
}

fn nbt_gzip_decode(bytes: &[u8]) -> NbtRoot {
    let mut dec = GzDecoder::new(bytes);
    let mut raw = Vec::with_capacity(bytes.len() * 3);
    dec.read_to_end(&mut raw).unwrap();
    let mut buf = Bytes::from(raw);
    read_nbt_be(&mut buf).unwrap()
}

fn main() {
    println!("=== player_data_format bench ===");
    println!("warmup={WARMUP}, runs={RUNS}\n");

    // ── Profil minimal ──
    println!("── Profil `minimal` (pos + rotation + gamemode + ...) ──");
    let min_j = sample_minimal();
    let min_n = sample_minimal_nbt();

    let min_j_bytes = json_encode(&min_j);
    let min_n_bytes = nbt_gzip_encode(&min_n);
    println!(
        "  taille JSON            = {}",
        fmt_bytes(min_j_bytes.len())
    );
    println!(
        "  taille NBT+gzip        = {}",
        fmt_bytes(min_n_bytes.len())
    );
    println!();

    let j_clone = min_j.clone();
    bench("JSON encode", || {
        let _ = std::hint::black_box(json_encode(std::hint::black_box(&j_clone)));
    });
    let nbt_clone = min_n.clone();
    bench("NBT+gzip encode", || {
        let _ = std::hint::black_box(nbt_gzip_encode(std::hint::black_box(&nbt_clone)));
    });
    bench("JSON decode", || {
        let _: MinimalJson = std::hint::black_box(json_decode(std::hint::black_box(&min_j_bytes)));
    });
    bench("NBT+gzip decode", || {
        let _ = std::hint::black_box(nbt_gzip_decode(std::hint::black_box(&min_n_bytes)));
    });
    println!();

    // ── Profil full ──
    println!("── Profil `full` (+ inventory 36 + skin 64x64 + geometry JSON) ──");
    let full_j = sample_full();
    let full_n = sample_full_nbt();

    let full_j_bytes = json_encode(&full_j);
    let full_n_bytes = nbt_gzip_encode(&full_n);
    println!(
        "  taille JSON            = {}",
        fmt_bytes(full_j_bytes.len())
    );
    println!(
        "  taille NBT+gzip        = {}",
        fmt_bytes(full_n_bytes.len())
    );
    println!();

    let j_clone = full_j.clone();
    bench("JSON encode", || {
        let _ = std::hint::black_box(json_encode(std::hint::black_box(&j_clone)));
    });
    let nbt_clone = full_n.clone();
    bench("NBT+gzip encode", || {
        let _ = std::hint::black_box(nbt_gzip_encode(std::hint::black_box(&nbt_clone)));
    });
    bench("JSON decode", || {
        let _: FullJson = std::hint::black_box(json_decode(std::hint::black_box(&full_j_bytes)));
    });
    bench("NBT+gzip decode", || {
        let _ = std::hint::black_box(nbt_gzip_decode(std::hint::black_box(&full_n_bytes)));
    });
    println!();

    println!("Bench terminé.");
}
