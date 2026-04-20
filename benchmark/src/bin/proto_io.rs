//! Bench : ProtoReader/ProtoWriter hot paths.
//!
//! Chaque paquet wire passe par les VarInt + write_string/read_string, appelés
//! des millions de fois/seconde sous charge. On mesure :
//! - VarUInt32 / VarInt32 encode + decode (valeurs petites vs grandes)
//! - write_string + read_string (chaînes 16 B, 256 B, 4 KB)
//! - write_raw / read_raw (copie buffer)
//!
//! Run : `cargo run --release -p benchmark --bin proto_io`

use benchmark::bench;
use mc_rs_proto::io::{ProtoReader, ProtoWriter};

const WARMUP: u32 = 500;
const RUNS: u32 = 10_000;

fn main() {
    println!("=== proto_io bench ===");
    println!("warmup={WARMUP}, runs={RUNS}\n");

    // ── VarInt encode ──
    println!("── VarUInt32 / VarInt32 encode (1000 valeurs par run) ──");
    let small_values: Vec<u32> = (0..1000).map(|i| i as u32).collect(); // 1-3 bytes
    let large_values: Vec<u32> = (0..1000).map(|i| (i as u32).wrapping_mul(131_071)).collect(); // up to 5 bytes

    bench("VarUInt32 encode (small,1-2B)", WARMUP, RUNS, || {
        let mut w = ProtoWriter::with_capacity(2048);
        for v in std::hint::black_box(&small_values) {
            w.write_var_u32(*v);
        }
        std::hint::black_box(w);
    });
    bench("VarUInt32 encode (large,4-5B)", WARMUP, RUNS, || {
        let mut w = ProtoWriter::with_capacity(5120);
        for v in std::hint::black_box(&large_values) {
            w.write_var_u32(*v);
        }
        std::hint::black_box(w);
    });
    bench("VarInt32  encode (mixed zigzag)", WARMUP, RUNS, || {
        let mut w = ProtoWriter::with_capacity(5120);
        for v in std::hint::black_box(&large_values) {
            w.write_var_i32(*v as i32);
        }
        std::hint::black_box(w);
    });

    // ── VarInt decode ──
    println!();
    println!("── VarUInt32 / VarInt32 decode (1000 valeurs par run) ──");
    let buf_small = {
        let mut w = ProtoWriter::with_capacity(2048);
        for v in &small_values {
            w.write_var_u32(*v);
        }
        w.into_bytes()
    };
    let buf_large = {
        let mut w = ProtoWriter::with_capacity(5120);
        for v in &large_values {
            w.write_var_u32(*v);
        }
        w.into_bytes()
    };

    bench("VarUInt32 decode (small)", WARMUP, RUNS, || {
        let mut r = ProtoReader::new(std::hint::black_box(&buf_small));
        for _ in 0..1000 {
            std::hint::black_box(r.read_var_u32().unwrap());
        }
    });
    bench("VarUInt32 decode (large)", WARMUP, RUNS, || {
        let mut r = ProtoReader::new(std::hint::black_box(&buf_large));
        for _ in 0..1000 {
            std::hint::black_box(r.read_var_u32().unwrap());
        }
    });

    // ── Strings ──
    println!();
    println!("── write_string / read_string (100 strings par run) ──");
    let s_16 = "a".repeat(16);
    let s_256 = "b".repeat(256);
    let s_4k = "c".repeat(4096);

    for (label, s) in [("16 B", &s_16), ("256 B", &s_256), ("4 KB", &s_4k)] {
        let total = s.len() * 100;
        bench(&format!("write_string x100 [{label}]"), WARMUP, RUNS, || {
            let mut w = ProtoWriter::with_capacity(total + 200);
            for _ in 0..100 {
                w.write_string(std::hint::black_box(s));
            }
            std::hint::black_box(w);
        });
        let buf = {
            let mut w = ProtoWriter::with_capacity(total + 200);
            for _ in 0..100 {
                w.write_string(s);
            }
            w.into_bytes()
        };
        bench(&format!("read_string  x100 [{label}]"), WARMUP, RUNS, || {
            let mut r = ProtoReader::new(std::hint::black_box(&buf));
            for _ in 0..100 {
                std::hint::black_box(r.read_string().unwrap());
            }
        });
    }

    println!();
    println!("Bench terminé.");
}
