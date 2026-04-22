//! Mesure TPS (ticks per second) via fenêtre glissante 1 seconde + sampler
//! système via `sysinfo` (RSS, CPU%, threads).

use crate::snapshot::SystemStats;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

pub struct TpsTracker {
    window_start: Instant,
    tick_count_in_window: u64,
    last_tps: f32,
    total_ticks: u64,
}

impl Default for TpsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TpsTracker {
    pub fn new() -> Self {
        Self {
            window_start: Instant::now(),
            tick_count_in_window: 0,
            last_tps: 0.0,
            total_ticks: 0,
        }
    }

    /// À appeler à chaque server tick. Renvoie le TPS courant (mis à jour
    /// chaque fois que la fenêtre de 1s est remplie).
    pub fn on_tick(&mut self) -> f32 {
        self.tick_count_in_window += 1;
        self.total_ticks += 1;
        let elapsed = self.window_start.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.last_tps = self.tick_count_in_window as f32 / elapsed.as_secs_f32();
            self.tick_count_in_window = 0;
            self.window_start = Instant::now();
        }
        self.last_tps
    }

    pub fn current_tps(&self) -> f32 {
        self.last_tps
    }

    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }
}

/// Wrapper stateful sur `sysinfo::System` : garde le PID du process courant et
/// ne rafraîchit que ce dont on a besoin (cheap à appeler à 1 Hz).
pub struct SystemProbe {
    system: System,
    pid: Pid,
    host_cpu_count: u32,
    host_total_mb: f32,
}

impl SystemProbe {
    pub fn new() -> Self {
        let pid = Pid::from(std::process::id() as usize);
        let mut system = System::new_with_specifics(
            RefreshKind::new()
                .with_processes(ProcessRefreshKind::new().with_cpu().with_memory()),
        );
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        let host_snapshot = System::new_all();
        let host_cpu_count = host_snapshot.physical_core_count().unwrap_or(0) as u32;
        let host_total_mb = (host_snapshot.total_memory() as f32) / (1024.0 * 1024.0);
        Self {
            system,
            pid,
            host_cpu_count,
            host_total_mb,
        }
    }

    pub fn sample(&mut self) -> SystemStats {
        self.system.refresh_processes(
            sysinfo::ProcessesToUpdate::Some(&[self.pid]),
            true,
        );
        let proc = match self.system.process(self.pid) {
            Some(p) => p,
            None => return SystemStats::default(),
        };
        // memory() renvoie des octets.
        let mem_mb = (proc.memory() as f32) / (1024.0 * 1024.0);
        let cpu_percent = proc.cpu_usage();
        // tasks() = threads sur Linux, None ailleurs → fallback 0.
        let threads = proc.tasks().map(|t| t.len() as u32).unwrap_or(0);
        SystemStats {
            mem_mb,
            mem_total_mb: self.host_total_mb,
            cpu_percent,
            threads,
            pid: self.pid.as_u32(),
            host_cpu_count: self.host_cpu_count,
        }
    }
}

impl Default for SystemProbe {
    fn default() -> Self {
        Self::new()
    }
}

/// Compteur de bytes réseau cumulés côté RakNet. Incrément-seulement, lecture
/// snapshot par la main loop pour calcul de rate.
#[derive(Debug, Default)]
pub struct NetCounter {
    pub bytes_in: std::sync::atomic::AtomicU64,
    pub bytes_out: std::sync::atomic::AtomicU64,
}

impl NetCounter {
    pub fn add_in(&self, n: u64) {
        self.bytes_in
            .fetch_add(n, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn add_out(&self, n: u64) {
        self.bytes_out
            .fetch_add(n, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> (u64, u64) {
        (
            self.bytes_in.load(std::sync::atomic::Ordering::Relaxed),
            self.bytes_out.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn tps_measured_after_window() {
        let mut t = TpsTracker::new();
        for _ in 0..50 {
            t.on_tick();
        }
        assert_eq!(t.current_tps(), 0.0);
        sleep(Duration::from_millis(1050));
        t.on_tick();
        assert!(t.current_tps() > 40.0);
    }

    #[test]
    fn system_probe_returns_non_zero_memory() {
        let mut probe = SystemProbe::new();
        let stats = probe.sample();
        assert!(stats.mem_mb > 0.0, "mem_mb should be > 0 (got {})", stats.mem_mb);
    }
}
