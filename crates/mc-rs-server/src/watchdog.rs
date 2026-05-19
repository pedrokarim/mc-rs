//! Watchdog de gel du main-loop.
//!
//! Problème observé : le serveur se fige (process vivant, plus aucun log,
//! aucun panic donc aucun `CRASH-*.log`). Sans debugger natif dispo, on ne
//! peut pas savoir OÙ le thread est bloqué. Ce module comble ce trou
//! d'observabilité, comme le panic-hook synchrone l'a fait pour les panics.
//!
//! Principe (v2) : **chaque appel [`checkpoint`] incrémente le heartbeat**.
//! Les checkpoints sont semés dans TOUTES les phases du `tokio::select!`
//! (arm `recv` ET arm `tick`). Tant que le main-loop progresse — peu importe
//! la phase — le heartbeat avance. S'il gèle, c'est qu'on est bloqué dans la
//! phase dont l'id est dans [`CHECKPOINT`] → un thread OS dédié écrit
//! `logs/FREEZE-<ts>.log` avec cette phase exacte. Il logue aussi un état
//! périodique (preuve de vie + dernière phase) pour visibilité continue.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Compteur de progression : incrémenté à CHAQUE checkpoint franchi (donc à
/// chaque sous-étape du select!, arm recv comme arm tick).
static HEARTBEAT: AtomicU64 = AtomicU64::new(0);
/// Index de la dernière phase franchie (dans [`CHECKPOINTS`]).
static CHECKPOINT: AtomicUsize = AtomicUsize::new(0);

/// Secondes sans progression avant de déclarer un gel.
const STALL_SECS: u64 = 6;
/// Période du log de preuve-de-vie (visibilité continue même sans gel).
const HEARTBEAT_LOG_SECS: u64 = 20;

/// Libellés des phases. L'index est l'argument passé à [`checkpoint`].
/// Garder synchronisé avec les appels dans `main.rs`.
pub const CHECKPOINTS: &[&str] = &[
    "loop-top",                  // 0
    "recv_and_process",          // 1
    "accept_peers",              // 2
    "process_peer_events@recv",  // 3
    "webui_snapshot/tick-start", // 4
    "raknet.tick_sessions",      // 5
    "world_tick",                // 6
    "game_tick",                 // 7
    "session_tick",              // 8
    "mob_tick",                  // 9
    "item_tick",                 // 10
    "autosave",                  // 11
    "process_peer_events@tick",  // 12
    "shutdown_save",             // 13
];

/// Conservé pour compat : équivaut à un checkpoint "tick-start".
#[inline]
pub fn beat() {
    HEARTBEAT.fetch_add(1, Ordering::Relaxed);
}

/// Enregistre la phase courante ET fait progresser le heartbeat. Appelé au
/// début de chaque sous-étape du main-loop.
#[inline]
pub fn checkpoint(id: usize) {
    CHECKPOINT.store(id, Ordering::Relaxed);
    HEARTBEAT.fetch_add(1, Ordering::Relaxed);
}

fn checkpoint_name(id: usize) -> &'static str {
    CHECKPOINTS.get(id).copied().unwrap_or("<unknown>")
}

/// Démarre le thread watchdog. À appeler une fois au boot, après l'init des
/// logs. `log_dir` = `config.logging.directory`.
pub fn start(log_dir: impl Into<PathBuf>) {
    let log_dir = log_dir.into();
    let spawned = std::thread::Builder::new()
        .name("watchdog".to_string())
        .spawn(move || {
            let poll = Duration::from_secs(1);
            let mut last_seen = HEARTBEAT.load(Ordering::Relaxed);
            let mut stalled_for: u64 = 0;
            let mut since_alive_log: u64 = 0;
            let mut already_reported = false;

            loop {
                std::thread::sleep(poll);
                let now = HEARTBEAT.load(Ordering::Relaxed);
                let cp = CHECKPOINT.load(Ordering::Relaxed);

                // Preuve de vie périodique : on voit le heartbeat avancer et
                // la dernière phase, même sans gel.
                since_alive_log += poll.as_secs();
                if since_alive_log >= HEARTBEAT_LOG_SECS {
                    since_alive_log = 0;
                    tracing::info!(
                        target: "watchdog",
                        "alive: heartbeat={now} last_phase=[{cp}] {}",
                        checkpoint_name(cp)
                    );
                }

                if now != last_seen {
                    last_seen = now;
                    stalled_for = 0;
                    already_reported = false;
                    continue;
                }

                stalled_for += poll.as_secs();
                if stalled_for < STALL_SECS || already_reported {
                    continue;
                }
                already_reported = true;

                let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let ts_file = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S");
                let dump = format!(
                    "==== FREEZE DETECTED ====\n\
                     time:            {ts}\n\
                     stalled_for:     ~{stalled_for}s (aucun checkpoint franchi)\n\
                     heartbeat:        {now} (gelé)\n\
                     phase bloquée:   [{cp}] {name}\n\
                     => le main-loop est bloqué DANS cette phase. Chercher un\n\
                        lock std::sync tenu pendant un appel bloquant, un\n\
                        re-lock réentrant, ou un .await qui ne résout jamais\n\
                        dans la section '{name}' (main.rs / handler associé).\n\
                     =========================\n",
                    name = checkpoint_name(cp),
                );

                let _ = std::fs::create_dir_all(&log_dir);
                let path = log_dir.join(format!("FREEZE-{ts_file}.log"));
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                {
                    let _ = f.write_all(dump.as_bytes());
                    let _ = f.flush();
                    let _ = f.sync_all();
                }
                let _ = std::io::stderr().write_all(dump.as_bytes());
                let _ = std::io::stderr().flush();
                tracing::error!(
                    target: "watchdog",
                    "FREEZE: main-loop bloqué ~{stalled_for}s en phase [{cp}] {}",
                    checkpoint_name(cp)
                );
            }
        });

    match spawned {
        Ok(_) => tracing::info!(
            target: "watchdog",
            "watchdog démarré (stall>{STALL_SECS}s → logs/FREEZE-*.log)"
        ),
        Err(e) => tracing::error!(target: "watchdog", "watchdog NON démarré: {e}"),
    }
}
