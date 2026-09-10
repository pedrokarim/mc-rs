//! Bench : zlib vs snappy vs raw pour les game packet batches.
//!
//! Le serveur compresse à 20+ Hz par joueur (PlayerAuthInput feedback + chunks
//! + entities + inventory...). Trois profils :
//! - `small`  — 3 paquets, ~300 B total (chat + move + inv delta)
//! - `medium` — 15 paquets, ~8 KB (typical multi-player frame)
//! - `large`  — 1 paquet, ~150 KB (chunk data)
//!
//! Métriques : encode_time, decode_time, taille compressée, throughput MB/s.
//!
//! Run : `cargo run --release -p benchmark --bin batch_compression`

use benchmark::{bench, fmt_bytes, throughput_mb_per_sec};
use mc_rs_proto::batch::{decode_batch, encode_batch, CompressionAlgorithm};

const WARMUP: u32 = 50;
const RUNS: u32 = 500;

fn fill_deterministic(len: usize, seed: u64) -> Vec<u8> {
    // Payload pseudo-aléatoire mais compressible (patterns qui se répètent).
    let mut s = seed;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // Restreindre à un alphabet de ~32 valeurs pour que zlib puisse
        // effectivement compresser (sinon c'est du bruit pur).
        out.push(((s >> 56) as u8) & 0x1F);
    }
    out
}

fn profile(name: &str, packets: &[Vec<u8>]) {
    let raw_size: usize = packets.iter().map(|p| p.len()).sum();
    let n_packets = packets.len();
    println!(
        "── Profil `{name}` — {n_packets} paquet(s), payload raw = {} ──",
        fmt_bytes(raw_size)
    );

    // Tailles résultantes.
    let zlib_6 = encode_batch(packets, CompressionAlgorithm::Zlib, 6);
    let zlib_1 = encode_batch(packets, CompressionAlgorithm::Zlib, 1);
    let zlib_9 = encode_batch(packets, CompressionAlgorithm::Zlib, 9);
    let snappy = encode_batch(packets, CompressionAlgorithm::Snappy, 0);
    let none = encode_batch(packets, CompressionAlgorithm::None, 0);
    println!(
        "  taille zlib(L1)        = {}  ratio={:.2}",
        fmt_bytes(zlib_1.len()),
        zlib_1.len() as f64 / raw_size as f64
    );
    println!(
        "  taille zlib(L6)        = {}  ratio={:.2}",
        fmt_bytes(zlib_6.len()),
        zlib_6.len() as f64 / raw_size as f64
    );
    println!(
        "  taille zlib(L9)        = {}  ratio={:.2}",
        fmt_bytes(zlib_9.len()),
        zlib_9.len() as f64 / raw_size as f64
    );
    println!(
        "  taille snappy          = {}  ratio={:.2}",
        fmt_bytes(snappy.len()),
        snappy.len() as f64 / raw_size as f64
    );
    println!(
        "  taille none            = {}  (raw + 1 byte algo header)",
        fmt_bytes(none.len()),
    );
    println!();

    // Encode throughput.
    let enc_zlib6 = bench("zlib(L6) encode", WARMUP, RUNS, || {
        let _ = std::hint::black_box(encode_batch(
            std::hint::black_box(packets),
            CompressionAlgorithm::Zlib,
            6,
        ));
    });
    let enc_zlib1 = bench("zlib(L1) encode", WARMUP, RUNS, || {
        let _ = std::hint::black_box(encode_batch(
            std::hint::black_box(packets),
            CompressionAlgorithm::Zlib,
            1,
        ));
    });
    let enc_snappy = bench("snappy   encode", WARMUP, RUNS, || {
        let _ = std::hint::black_box(encode_batch(
            std::hint::black_box(packets),
            CompressionAlgorithm::Snappy,
            0,
        ));
    });
    let enc_none = bench("none     encode", WARMUP, RUNS, || {
        let _ = std::hint::black_box(encode_batch(
            std::hint::black_box(packets),
            CompressionAlgorithm::None,
            0,
        ));
    });

    // Decode throughput : on lit l'algo EFFECTIF depuis encoded[0] pour suivre
    // un éventuel downgrade automatique (COMPRESSION_THRESHOLD).
    let algo_zlib6 = CompressionAlgorithm::from_u8(zlib_6[0]).unwrap();
    let algo_snappy = CompressionAlgorithm::from_u8(snappy[0]).unwrap();
    let algo_none = CompressionAlgorithm::from_u8(none[0]).unwrap();
    let dec_zlib6 = bench("zlib(L6) decode", WARMUP, RUNS, || {
        let _ = std::hint::black_box(decode_batch(&zlib_6[1..], algo_zlib6).unwrap());
    });
    let dec_snappy = bench("snappy   decode", WARMUP, RUNS, || {
        let _ = std::hint::black_box(decode_batch(&snappy[1..], algo_snappy).unwrap());
    });
    let dec_none = bench("none     decode", WARMUP, RUNS, || {
        let _ = std::hint::black_box(decode_batch(&none[1..], algo_none).unwrap());
    });

    println!();
    println!("  throughput encode (MB/s, raw payload) :");
    println!(
        "    zlib(L6) = {:>8.1}  |  zlib(L1) = {:>8.1}  |  snappy = {:>8.1}  |  none = {:>8.1}",
        throughput_mb_per_sec(raw_size, enc_zlib6),
        throughput_mb_per_sec(raw_size, enc_zlib1),
        throughput_mb_per_sec(raw_size, enc_snappy),
        throughput_mb_per_sec(raw_size, enc_none),
    );
    println!("  throughput decode (MB/s, raw payload) :");
    println!(
        "    zlib(L6) = {:>8.1}  |                     snappy = {:>8.1}  |  none = {:>8.1}",
        throughput_mb_per_sec(raw_size, dec_zlib6),
        throughput_mb_per_sec(raw_size, dec_snappy),
        throughput_mb_per_sec(raw_size, dec_none),
    );
    println!();
}

fn main() {
    println!("=== batch_compression bench ===");
    println!("warmup={WARMUP}, runs={RUNS}\n");

    // `small` : frame typique multi-joueur
    let small: Vec<Vec<u8>> = vec![
        fill_deterministic(80, 1),  // chat / text
        fill_deterministic(120, 2), // MovePlayer
        fill_deterministic(90, 3),  // inventory slot delta
    ];
    profile("small", &small);

    // `medium` : frame plus chargée (mob updates, chunk tickets, entity spawns)
    let medium: Vec<Vec<u8>> = (0..15)
        .map(|i| fill_deterministic(500 + (i * 30), (i as u64) + 100))
        .collect();
    profile("medium", &medium);

    // `large` : un seul paquet chunk (level_chunk fait typically 50-200 KB en raw).
    let large: Vec<Vec<u8>> = vec![fill_deterministic(150_000, 42)];
    profile("large", &large);

    println!("Bench terminé.");
}
