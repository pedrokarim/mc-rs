//! Rate limiter simple token-bucket par IP pour `/login`.
//!
//! Limite : `MAX_ATTEMPTS` tentatives dans une fenêtre de `WINDOW_SECS`. Au-delà,
//! la réponse est `429 Too Many Requests` pendant le reste de la fenêtre.
//!
//! Stockage : `HashMap<IpAddr, (count, window_start)>` derrière un `Mutex`.
//! Les entrées expirent automatiquement au prochain hit post-fenêtre — pas de
//! thread de cleanup (simple, suffisant vu la taille : N IPs actifs).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const MAX_ATTEMPTS: u32 = 5;
pub const WINDOW_SECS: u64 = 300; // 5 minutes

pub struct RateLimiter {
    // Optimisation : on pourrait utiliser DashMap, mais Mutex<HashMap> est
    // largement suffisant vu qu'on ne touche qu'à /login.
    inner: Mutex<HashMap<IpAddr, Entry>>,
}

#[derive(Clone, Copy)]
struct Entry {
    count: u32,
    window_start: Instant,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Enregistre une tentative. Renvoie `None` si autorisé, ou `Some(retry_after_secs)`
    /// si limite dépassée.
    pub fn check_and_record(&self, ip: IpAddr) -> Option<u64> {
        let now = Instant::now();
        let window = Duration::from_secs(WINDOW_SECS);
        let mut map = match self.inner.lock() {
            Ok(g) => g,
            Err(poison) => poison.into_inner(), // garde fonctionnel même si poisoned
        };
        let entry = map.entry(ip).or_insert(Entry {
            count: 0,
            window_start: now,
        });
        // Fenêtre expirée : reset.
        if now.duration_since(entry.window_start) > window {
            entry.count = 0;
            entry.window_start = now;
        }
        entry.count = entry.count.saturating_add(1);
        if entry.count > MAX_ATTEMPTS {
            let remaining = window - now.duration_since(entry.window_start).min(window);
            return Some(remaining.as_secs().max(1));
        }
        None
    }

    /// Reset manuel (ex: après un login réussi → on ne punit pas le user légitime).
    pub fn reset(&self, ip: IpAddr) {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(&ip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_then_block() {
        let rl = RateLimiter::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        for _ in 0..MAX_ATTEMPTS {
            assert!(rl.check_and_record(ip).is_none());
        }
        assert!(rl.check_and_record(ip).is_some());
    }

    #[test]
    fn reset_clears() {
        let rl = RateLimiter::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        for _ in 0..=MAX_ATTEMPTS {
            rl.check_and_record(ip);
        }
        rl.reset(ip);
        assert!(rl.check_and_record(ip).is_none());
    }
}
