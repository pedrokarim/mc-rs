//! Bench : AES-256-CTR fakeGCM (mc-rs-crypto::EncryptionContext).
//!
//! Chaque paquet sortant post-handshake est encrypté + SHA-256 checksum.
//! On mesure encrypt et decrypt throughput sur trois tailles :
//! - 64 B      (petit paquet : MovePlayer, Text, etc.)
//! - 1.5 KB    (paquet moyen : inventory delta, entity updates)
//! - 128 KB    (gros paquet : chunk data)
//!
//! Run : `cargo run --release -p benchmark --bin aes_ctr`

use benchmark::{bench, fmt_bytes, throughput_mb_per_sec};
use mc_rs_crypto::encrypt::EncryptionContext;

const WARMUP: u32 = 200;
const RUNS: u32 = 5_000;

fn make_payload(size: usize, seed: u8) -> Vec<u8> {
    (0..size).map(|i| (i as u8).wrapping_add(seed)).collect()
}

fn run(size: usize, label: &str) {
    println!("── Payload = {label} ({}) ──", fmt_bytes(size));
    let key = {
        let mut k = [0u8; 32];
        for (i, b) in k.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37);
        }
        k
    };
    let payload = make_payload(size, 7);

    // Encrypt : on doit renouveler le contexte à chaque run (counter++), sinon
    // le bench mesure un flux CTR continu. Le vrai usage = 1 encrypt/paquet
    // mais le même contexte vit plusieurs paquets — on utilise donc un flux
    // continu pour plus de réalisme.
    let mut ctx = EncryptionContext::new(key);
    let enc_d = bench("AES-256-CTR encrypt", WARMUP, RUNS, || {
        let _ = std::hint::black_box(ctx.encrypt(std::hint::black_box(&payload)));
    });

    // Decrypt : on doit pré-encrypter N runs et les consommer avec un contexte
    // frais côté decrypt. Simpler : encrypt sur ctx_a, decrypt sur ctx_b, sync
    // counter manuellement via N encrypts warmup.
    let n_samples = (WARMUP + RUNS) as usize;
    let mut ctx_enc = EncryptionContext::new(key);
    let encrypted: Vec<Vec<u8>> = (0..n_samples).map(|_| ctx_enc.encrypt(&payload)).collect();
    let mut ctx_dec = EncryptionContext::new(key);
    let mut idx = 0usize;
    let dec_d = bench("AES-256-CTR decrypt", WARMUP, RUNS, || {
        let ct = &encrypted[idx];
        idx += 1;
        let _ = std::hint::black_box(ctx_dec.decrypt(std::hint::black_box(ct)).unwrap());
    });

    println!(
        "  throughput encrypt = {:>7.1} MB/s",
        throughput_mb_per_sec(size, enc_d)
    );
    println!(
        "  throughput decrypt = {:>7.1} MB/s",
        throughput_mb_per_sec(size, dec_d)
    );
    println!();
}

fn main() {
    println!("=== aes_ctr bench ===");
    println!("warmup={WARMUP}, runs={RUNS}\n");

    run(64, "small");
    run(1_536, "medium");
    run(131_072, "large");

    println!("Bench terminé.");
}
