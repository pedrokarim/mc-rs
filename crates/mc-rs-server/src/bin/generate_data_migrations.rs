//! Génère les tables de migration des données sauvegardées lors d'un changement
//! de palette de blocs / de table d'items (montée de version Bedrock).
//!
//! - `src/world/palette_migration_data.rs` : pour chaque palette héritée,
//!   runtime ID ancien → runtime ID de la palette courante, apparié sur
//!   `(name, states)`. Chaque migration vise directement la palette courante :
//!   il faut donc repasser toutes les palettes héritées à chaque montée.
//! - `src/item_migration_data.rs` : pour chaque version de sauvegarde joueur, la
//!   table `(runtime_id, nom)` de son ancienne table d'items, pour retrouver le
//!   nom d'un item sauvegardé par ID réseau.
//!
//! Usage :
//! ```text
//! cargo run -p mc-rs-server --bin generate_data_migrations -- \
//!     --palette <libellé>=<palette.nbt>...                  \
//!     [--block-schema <schéma.json>]...                     \
//!     --items <data_version>,<libellé>,<premier_schéma>=<table>...
//! ```
//! - `--palette` : palette héritée (compounds NBT réseau concaténés).
//! - `--block-schema` : schéma d'upgrade d'états de blocs au format
//!   `pmmp/BedrockBlockUpgradeSchema` (aussi `df-mc/worldupgrader`). Seuls les
//!   `addedProperties` sont pris en charge : ils complètent, avec leur valeur
//!   par défaut, les états hérités auxquels manque la propriété, avant
//!   l'appariement. Toute autre opération fait échouer la génération.
//! - `--items` : table d'items héritée, `required_item_list.json` (BedrockData)
//!   ou `vanilla_items.nbt` (dragonfly). `premier_schéma` est le numéro du
//!   premier schéma d'upgrade d'items (`item_id_upgrade_schema.json`) que cette
//!   table ne reflète pas encore.
//!
//! La palette courante est lue dans `data/canonical_block_states.nbt`.

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use mc_rs_nbt::tag::{NbtCompound, NbtTag};

fn manifest_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Même empreinte que `generate_block_registry` (FNV-1a 64 du fichier).
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Représentation canonique (type explicite) d'une valeur d'état.
fn canonical_value(tag: &NbtTag) -> String {
    match tag {
        NbtTag::Byte(v) => format!("b{v}"),
        NbtTag::Short(v) => format!("s{v}"),
        NbtTag::Int(v) => format!("i{v}"),
        NbtTag::Long(v) => format!("l{v}"),
        NbtTag::String(v) => format!("t{v:?}"),
        other => format!("?{other:?}"),
    }
}

/// États d'un bloc, clés triées, valeurs canoniques.
type States = BTreeMap<String, String>;

fn read_states(states: Option<&NbtCompound>) -> States {
    states
        .map(|states| {
            states
                .iter()
                .map(|(k, v)| (k.clone(), canonical_value(v)))
                .collect()
        })
        .unwrap_or_default()
}

fn states_key(states: &States) -> String {
    states
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// `(nom, états)` dans l'ordre des runtime IDs.
type PaletteStates = Vec<(String, States)>;

/// Lit une palette (suite de compounds NBT réseau `{name, states, version}`)
/// et retourne ses états avec l'empreinte du fichier.
fn read_palette(path: &Path) -> Result<(PaletteStates, u64), Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    let fingerprint = fnv1a64(&data);
    let mut cursor = &data[..];
    let mut states = Vec::new();
    while !cursor.is_empty() {
        let root = mc_rs_nbt::read_nbt_network(&mut cursor)?;
        let name = match root.compound.get("name") {
            Some(NbtTag::String(name)) => name.clone(),
            _ => return Err(format!("palette entry without name in {}", path.display()).into()),
        };
        states.push((
            name,
            read_states(root.compound.get("states").and_then(NbtTag::as_compound)),
        ));
    }
    Ok((states, fingerprint))
}

/// `addedProperties` d'un schéma d'upgrade : bloc → propriété → valeur
/// canonique par défaut.
type AddedProperties = HashMap<String, States>;

/// Clés acceptées dans un schéma d'upgrade de blocs.
const SCHEMA_VERSION_KEYS: [&str; 4] = [
    "maxVersionMajor",
    "maxVersionMinor",
    "maxVersionPatch",
    "maxVersionRevision",
];

/// Valeur `{"byte": 0}` / `{"int": 1}` / `{"string": "none"}` d'un schéma
/// (`BlockStateUpgradeSchemaModelTag`) → valeur canonique.
fn schema_value(value: &serde_json::Value) -> Result<String, String> {
    let obj = value
        .as_object()
        .filter(|obj| obj.len() == 1)
        .ok_or_else(|| format!("valeur de schéma invalide : {value}"))?;
    let (kind, v) = obj.iter().next().unwrap();
    match (kind.as_str(), v) {
        ("byte", serde_json::Value::Number(n)) => Ok(format!("b{}", n)),
        ("int", serde_json::Value::Number(n)) => Ok(format!("i{}", n)),
        ("string", serde_json::Value::String(s)) => Ok(format!("t{s:?}")),
        _ => Err(format!(
            "type de valeur de schéma non pris en charge : {value}"
        )),
    }
}

fn read_block_schema(path: &Path) -> Result<AddedProperties, Box<dyn std::error::Error>> {
    let raw: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(path)?)?;
    let mut added = AddedProperties::new();
    for (key, value) in &raw {
        if SCHEMA_VERSION_KEYS.contains(&key.as_str()) {
            continue;
        }
        if key != "addedProperties" {
            return Err(format!(
                "{} : opération `{key}` non prise en charge par ce générateur",
                path.display()
            )
            .into());
        }
        let blocks = value
            .as_object()
            .ok_or("addedProperties doit être un objet")?;
        for (block, props) in blocks {
            let props = props.as_object().ok_or("propriétés ajoutées invalides")?;
            let entry = added.entry(block.clone()).or_default();
            for (prop, v) in props {
                entry.insert(prop.clone(), schema_value(v)?);
            }
        }
    }
    Ok(added)
}

/// Une migration de palette générée.
struct PaletteTable {
    label: String,
    source: PathBuf,
    from_fingerprint: u64,
    table: Vec<u32>,
    states: usize,
    completed: usize,
    fallback_same_name: Vec<String>,
    fallback_air: Vec<String>,
}

fn build_palette_table(
    label: &str,
    path: &Path,
    current: &PaletteStates,
    schemas: &[AddedProperties],
) -> Result<PaletteTable, Box<dyn std::error::Error>> {
    let (legacy, from_fingerprint) = read_palette(path)?;

    let mut exact: HashMap<(&str, String), u32> = HashMap::new();
    let mut first_by_name: HashMap<&str, u32> = HashMap::new();
    for (id, (name, states)) in current.iter().enumerate() {
        exact.insert((name.as_str(), states_key(states)), id as u32);
        first_by_name.entry(name.as_str()).or_insert(id as u32);
    }
    let air = *first_by_name
        .get("minecraft:air")
        .ok_or("current palette has no minecraft:air")?;

    let mut table = Vec::with_capacity(legacy.len());
    let mut completed = 0;
    let mut fallback_same_name = Vec::new();
    let mut fallback_air = Vec::new();
    for (name, states) in &legacy {
        // Propriétés ajoutées depuis : valeur par défaut du schéma, seulement
        // si l'état hérité ne la porte pas encore.
        let mut upgraded = states.clone();
        for schema in schemas {
            if let Some(props) = schema.get(name) {
                for (prop, value) in props {
                    upgraded
                        .entry(prop.clone())
                        .or_insert_with(|| value.clone());
                }
            }
        }
        if upgraded != *states {
            completed += 1;
        }
        if let Some(&id) = exact.get(&(name.as_str(), states_key(&upgraded))) {
            table.push(id);
        } else if let Some(&id) = first_by_name.get(name.as_str()) {
            fallback_same_name.push(format!("{name}[{}]", states_key(states)));
            table.push(id);
        } else {
            fallback_air.push(name.clone());
            table.push(air);
        }
    }
    Ok(PaletteTable {
        label: label.to_string(),
        source: path.to_path_buf(),
        from_fingerprint,
        table,
        states: legacy.len(),
        completed,
        fallback_same_name,
        fallback_air,
    })
}

/// Lit une table d'items héritée : `required_item_list.json` (BedrockData,
/// `{nom: {runtime_id, …}}`) ou `vanilla_items.nbt` (dragonfly, compound NBT
/// réseau `{nom: {runtime_id: Int, …}}`).
fn read_item_table(path: &Path) -> Result<Vec<(i64, String)>, Box<dyn std::error::Error>> {
    let mut items: Vec<(i64, String)> = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nbt"))
    {
        let data = fs::read(path)?;
        let root = mc_rs_nbt::read_nbt_network(&mut &data[..])?;
        root.compound
            .iter()
            .map(|(name, entry)| {
                let id = entry
                    .as_compound()
                    .and_then(|c| c.get("runtime_id"))
                    .and_then(|tag| match tag {
                        NbtTag::Int(v) => Some(i64::from(*v)),
                        NbtTag::Short(v) => Some(i64::from(*v)),
                        _ => None,
                    })
                    .ok_or_else(|| format!("item {name} sans runtime_id"))?;
                Ok((id, name.clone()))
            })
            .collect::<Result<_, String>>()?
    } else {
        let raw: HashMap<String, serde_json::Value> =
            serde_json::from_str(&fs::read_to_string(path)?)?;
        raw.into_iter()
            .map(|(name, entry)| {
                let id = entry
                    .get("runtime_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| format!("item {name} sans runtime_id"))?;
                Ok((id, name))
            })
            .collect::<Result<_, String>>()?
    };
    items.sort();
    Ok(items)
}

/// Table d'items héritée, telle que passée par `--items`.
struct ItemTableArg {
    data_version: u32,
    label: String,
    first_upgrade_schema: u32,
    path: PathBuf,
}

fn parse_items_arg(arg: &str) -> Result<ItemTableArg, String> {
    let (spec, path) = arg
        .split_once('=')
        .ok_or_else(|| format!("--items {arg} : `=<table>` manquant"))?;
    let parts: Vec<&str> = spec.split(',').collect();
    let [version, label, schema] = parts[..] else {
        return Err(format!(
            "--items {arg} : attendu <data_version>,<libellé>,<premier_schéma>=<table>"
        ));
    };
    Ok(ItemTableArg {
        data_version: version
            .parse()
            .map_err(|_| format!("data_version invalide : {version}"))?,
        label: label.to_string(),
        first_upgrade_schema: schema
            .parse()
            .map_err(|_| format!("numéro de schéma invalide : {schema}"))?,
        path: PathBuf::from(path),
    })
}

fn usage() -> ! {
    eprintln!(
        "usage: cargo run -p mc-rs-server --bin generate_data_migrations -- \
         --palette <libellé>=<palette.nbt>... [--block-schema <schéma.json>]... \
         --items <data_version>,<libellé>,<premier_schéma>=<table>..."
    );
    std::process::exit(2);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut palettes: Vec<(String, PathBuf)> = Vec::new();
    let mut schema_paths: Vec<PathBuf> = Vec::new();
    let mut item_tables: Vec<ItemTableArg> = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let Some(value) = args.next() else { usage() };
        match flag.as_str() {
            "--palette" => {
                let (label, path) = value.split_once('=').unwrap_or_else(|| usage());
                palettes.push((label.to_string(), PathBuf::from(path)));
            }
            "--block-schema" => schema_paths.push(PathBuf::from(value)),
            "--items" => item_tables.push(parse_items_arg(&value)?),
            _ => usage(),
        }
    }
    if palettes.is_empty() || item_tables.is_empty() {
        usage();
    }

    // ── Palette ────────────────────────────────────────────────────────────
    let current_path = manifest_path("data/canonical_block_states.nbt");
    let (current, current_fp) = read_palette(&current_path)?;
    let schemas = schema_paths
        .iter()
        .map(|path| read_block_schema(path))
        .collect::<Result<Vec<_>, _>>()?;
    let tables = palettes
        .iter()
        .map(|(label, path)| build_palette_table(label, path, &current, &schemas))
        .collect::<Result<Vec<_>, _>>()?;

    let mut out = String::new();
    writeln!(
        out,
        "// @generated by `cargo run -p mc-rs-server --bin generate_data_migrations`"
    )?;
    for path in &schema_paths {
        writeln!(out, "// Schéma d'états : {}", file_name(path))?;
    }
    for t in &tables {
        writeln!(out, "// Palette {} : {}", t.label, file_name(&t.source))?;
        writeln!(
            out,
            "//   {} états hérités : {} appariés exactement (dont {} complétés par schéma), {} repliés sur le premier état du même nom, {} sans équivalent (→ air).",
            t.states,
            t.states - t.fallback_same_name.len() - t.fallback_air.len(),
            t.completed,
            t.fallback_same_name.len(),
            t.fallback_air.len()
        )?;
        for s in &t.fallback_same_name {
            writeln!(out, "//   même nom : {s}")?;
        }
        for s in &t.fallback_air {
            writeln!(out, "//   → air : {s}")?;
        }
    }
    writeln!(out)?;
    writeln!(out, "use super::palette_migration::PaletteMigration;")?;
    writeln!(out)?;
    writeln!(
        out,
        "/// Migrations connues vers la palette courante (empreinte {current_fp:#018x})."
    )?;
    writeln!(
        out,
        "pub const PALETTE_MIGRATIONS: &[PaletteMigration] = &["
    )?;
    for t in &tables {
        writeln!(out, "    PaletteMigration {{")?;
        writeln!(out, "        label: {:?},", t.label)?;
        writeln!(
            out,
            "        from_fingerprint: {:#018x},",
            t.from_fingerprint
        )?;
        writeln!(out, "        to_fingerprint: {current_fp:#018x},")?;
        writeln!(out, "        table: &[")?;
        for chunk in t.table.chunks(16) {
            let line: Vec<String> = chunk.iter().map(u32::to_string).collect();
            writeln!(out, "            {},", line.join(", "))?;
        }
        writeln!(out, "        ],")?;
        writeln!(out, "    }},")?;
    }
    writeln!(out, "];")?;
    let palette_out = manifest_path("src/world/palette_migration_data.rs");
    fs::write(&palette_out, out)?;
    for t in &tables {
        println!(
            "palette {} : {} → {} états, {} complété(s) par schéma, {} repli(s) même nom, {} → air",
            t.label,
            t.states,
            current.len(),
            t.completed,
            t.fallback_same_name.len(),
            t.fallback_air.len()
        );
    }
    println!("→ {}", palette_out.display());

    // ── Items ──────────────────────────────────────────────────────────────
    item_tables.sort_by_key(|t| t.data_version);
    let mut out = String::new();
    writeln!(
        out,
        "// @generated by `cargo run -p mc-rs-server --bin generate_data_migrations`"
    )?;
    for t in &item_tables {
        writeln!(
            out,
            "// Table v{} ({}) : {}",
            t.data_version,
            t.label,
            file_name(&t.path)
        )?;
    }
    writeln!(out)?;
    writeln!(out, "use crate::item_registry::LegacyItemTable;")?;
    writeln!(out)?;
    writeln!(
        out,
        "/// Tables d'items héritées, une par version de sauvegarde joueur, triées par runtime ID."
    )?;
    writeln!(out, "pub const LEGACY_ITEM_TABLES: &[LegacyItemTable] = &[")?;
    for t in &item_tables {
        let items = read_item_table(&t.path)?;
        writeln!(out, "    LegacyItemTable {{")?;
        writeln!(out, "        data_version: {},", t.data_version)?;
        writeln!(
            out,
            "        first_upgrade_schema: {},",
            t.first_upgrade_schema
        )?;
        writeln!(out, "        ids: &[")?;
        for (id, name) in &items {
            writeln!(out, "            ({id}, {name:?}),")?;
        }
        writeln!(out, "        ],")?;
        writeln!(out, "    }},")?;
        println!(
            "items v{} ({}) : {} entrées",
            t.data_version,
            t.label,
            items.len()
        );
    }
    writeln!(out, "];")?;
    let items_out = manifest_path("src/item_migration_data.rs");
    fs::write(&items_out, out)?;
    println!("→ {}", items_out.display());

    Ok(())
}

/// Nom de fichier seul : le chemin complet dépend de la machine.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
