//! Bench : NBT encode/decode sur les 3 variants (LE disk, BE Java, Network VarInt).
//!
//! On utilise le même compound pour les 3 variants et on mesure encode+decode.
//! Le compound est représentatif d'un chunk NBT (block entities + biomes).
//!
//! Run : `cargo run --release -p benchmark --bin nbt_variants`

use benchmark::{bench, fmt_bytes, throughput_mb_per_sec};
use bytes::{Bytes, BytesMut};
use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_nbt::{read_nbt_be, read_nbt_le, read_nbt_network, write_nbt_be, write_nbt_le, write_nbt_network};

const WARMUP: u32 = 100;
const RUNS: u32 = 2_000;

fn sample_compound() -> NbtRoot {
    // Représentatif d'un block entity complex : un chest rempli de 27 items.
    let mut root = NbtCompound::new();
    root.insert("id".into(), NbtTag::String("minecraft:chest".into()));
    root.insert("x".into(), NbtTag::Int(123));
    root.insert("y".into(), NbtTag::Int(72));
    root.insert("z".into(), NbtTag::Int(-456));
    root.insert(
        "CustomName".into(),
        NbtTag::String("§bLoot Chest§r".into()),
    );

    let items: Vec<NbtTag> = (0..27)
        .map(|slot| {
            let mut item = NbtCompound::new();
            item.insert("Slot".into(), NbtTag::Byte(slot));
            item.insert("id".into(), NbtTag::String(format!("minecraft:item_{slot}")));
            item.insert("Count".into(), NbtTag::Byte(64));
            item.insert("Damage".into(), NbtTag::Short(0));

            // Tag NBT extra (enchantments) — List<Compound>.
            let enchants: Vec<NbtTag> = (0..3)
                .map(|e| {
                    let mut ench = NbtCompound::new();
                    ench.insert("id".into(), NbtTag::Short((e * 7) as i16));
                    ench.insert("lvl".into(), NbtTag::Short((e + 1) as i16));
                    NbtTag::Compound(ench)
                })
                .collect();
            let mut tag = NbtCompound::new();
            tag.insert("ench".into(), NbtTag::List(enchants));
            tag.insert(
                "display".into(),
                NbtTag::Compound({
                    let mut d = NbtCompound::new();
                    d.insert(
                        "Name".into(),
                        NbtTag::String(format!("§aItem {slot}")),
                    );
                    d
                }),
            );
            item.insert("tag".into(), NbtTag::Compound(tag));
            NbtTag::Compound(item)
        })
        .collect();
    root.insert("Items".into(), NbtTag::List(items));

    NbtRoot::new("", root)
}

fn main() {
    println!("=== nbt_variants bench ===");
    println!("warmup={WARMUP}, runs={RUNS}\n");

    let root = sample_compound();

    // Tailles.
    let mut le = BytesMut::new();
    write_nbt_le(&mut le, &root);
    let le_len = le.len();

    let mut be = BytesMut::new();
    write_nbt_be(&mut be, &root);
    let be_len = be.len();

    let mut net = BytesMut::new();
    write_nbt_network(&mut net, &root);
    let net_len = net.len();

    println!("  taille LE (disk)       = {}", fmt_bytes(le_len));
    println!("  taille BE (Java)       = {}", fmt_bytes(be_len));
    println!(
        "  taille Network(VarInt) = {}  ({:+} B vs LE)",
        fmt_bytes(net_len),
        net_len as i64 - le_len as i64
    );
    println!();

    // Encode.
    println!("── Encode ──");
    let enc_le = bench("LE        encode", WARMUP, RUNS, || {
        let mut buf = BytesMut::with_capacity(le_len);
        write_nbt_le(&mut buf, std::hint::black_box(&root));
        std::hint::black_box(buf);
    });
    let enc_be = bench("BE        encode", WARMUP, RUNS, || {
        let mut buf = BytesMut::with_capacity(be_len);
        write_nbt_be(&mut buf, std::hint::black_box(&root));
        std::hint::black_box(buf);
    });
    let enc_net = bench("Network   encode", WARMUP, RUNS, || {
        let mut buf = BytesMut::with_capacity(net_len);
        write_nbt_network(&mut buf, std::hint::black_box(&root));
        std::hint::black_box(buf);
    });

    println!();
    println!("── Decode ──");
    let le_bytes = Bytes::from(le.to_vec());
    let be_bytes = Bytes::from(be.to_vec());
    let net_bytes = Bytes::from(net.to_vec());
    let dec_le = bench("LE        decode", WARMUP, RUNS, || {
        let mut b = le_bytes.clone();
        std::hint::black_box(read_nbt_le(&mut b).unwrap());
    });
    let dec_be = bench("BE        decode", WARMUP, RUNS, || {
        let mut b = be_bytes.clone();
        std::hint::black_box(read_nbt_be(&mut b).unwrap());
    });
    let dec_net = bench("Network   decode", WARMUP, RUNS, || {
        let mut b = net_bytes.clone();
        std::hint::black_box(read_nbt_network(&mut b).unwrap());
    });

    println!();
    println!("  throughput encode (MB/s) :");
    println!(
        "    LE = {:>7.1}  BE = {:>7.1}  Network = {:>7.1}",
        throughput_mb_per_sec(le_len, enc_le),
        throughput_mb_per_sec(be_len, enc_be),
        throughput_mb_per_sec(net_len, enc_net),
    );
    println!("  throughput decode (MB/s) :");
    println!(
        "    LE = {:>7.1}  BE = {:>7.1}  Network = {:>7.1}",
        throughput_mb_per_sec(le_len, dec_le),
        throughput_mb_per_sec(be_len, dec_be),
        throughput_mb_per_sec(net_len, dec_net),
    );
    println!();
    println!("Bench terminé.");
}
