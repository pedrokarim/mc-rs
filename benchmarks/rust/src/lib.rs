//! Helpers partagés pour tous les benchmarks du workspace.

use std::time::{Duration, Instant};

/// Lance `f()` `warmup` fois (ignorés) puis `runs` fois en mesurant. Imprime
/// median / mean / p99 et renvoie le median (utile pour composer des ratios).
pub fn bench<F: FnMut()>(name: &str, warmup: u32, runs: u32, mut f: F) -> Duration {
    for _ in 0..warmup {
        f();
    }
    let mut samples = Vec::with_capacity(runs as usize);
    for _ in 0..runs {
        let t = Instant::now();
        f();
        samples.push(t.elapsed());
    }
    samples.sort();
    let median = samples[samples.len() / 2];
    let p99 = samples[(samples.len() * 99) / 100];
    let mean: Duration = samples.iter().sum::<Duration>() / samples.len() as u32;
    println!(
        "  {name:<40} median={:>10.3?}  mean={:>10.3?}  p99={:>10.3?}",
        median, mean, p99
    );
    median
}

/// Format un nombre d'octets en B / KB / MB.
pub fn fmt_bytes(n: usize) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.2} KB", n as f64 / 1024.0)
    } else {
        format!("{:.2} MB", n as f64 / (1024.0 * 1024.0))
    }
}

/// Throughput en MB/s à partir d'une durée et d'une taille de payload.
pub fn throughput_mb_per_sec(size_bytes: usize, d: Duration) -> f64 {
    let secs = d.as_secs_f64();
    if secs == 0.0 {
        return 0.0;
    }
    (size_bytes as f64 / (1024.0 * 1024.0)) / secs
}
