//! Threads de chargement et de génération des chunks, hors de la boucle principale.
//!
//! Équivalent de l'`AsyncPool` de PMMP (`scheduler/AsyncPool.php`), où `World::requestChunkPopulation`
//! confie la génération à des workers (`PopulationTask`) et où le joueur ne reçoit un chunk qu'une fois
//! celui-ci prêt (`Player::requestChunks`). Ici, la file des chunks d'un joueur demande les chunks absents à
//! [`ChunkCache::request_chunk`](super::chunk_cache::ChunkCache::request_chunk), qui y poste une tâche ;
//! le worker lit la LevelDB ou génère le chunk sans tenir le verrou du cache, puis l'y insère.
//!
//! Avant, un chunk absent était généré dans la boucle principale (environ 90 ms pour le générateur
//! « noise »), ce qui figeait tout le serveur pendant qu'un joueur explorait.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use tracing::{error, info};

/// Une tâche : charger ou générer un chunk, puis l'insérer dans son cache.
pub type Job = Box<dyn FnOnce() + Send>;

/// Point d'entrée des workers, clonable (un par cache de dimension).
#[derive(Clone)]
pub struct ChunkWorkers {
    sender: Sender<Job>,
}

impl ChunkWorkers {
    /// Lance `threads` workers (au moins un). Ils s'arrêtent quand le dernier `ChunkWorkers` est détruit.
    pub fn start(threads: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let threads = threads.max(1);
        for index in 0..threads {
            let receiver = Arc::clone(&receiver);
            let spawned = std::thread::Builder::new()
                .name(format!("chunk-worker-{index}"))
                .spawn(move || run(&receiver));
            if let Err(e) = spawned {
                error!("Impossible de lancer le worker de chunks {index} : {e}");
            }
        }
        info!("{threads} workers de chargement et de génération des chunks");
        Self { sender }
    }

    /// Nombre de workers par défaut : les cœurs moins deux (la boucle principale et RakNet), entre 1 et 4,
    /// comme l'`async-workers: auto` de PMMP (cœurs moins deux), plafonné pour laisser la machine libre.
    pub fn default_threads() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(2))
            .unwrap_or(1)
            .clamp(1, 4)
    }

    /// Poste une tâche ; faux si tous les workers sont arrêtés.
    pub fn submit(&self, job: Job) -> bool {
        self.sender.send(job).is_ok()
    }
}

fn run(receiver: &Mutex<Receiver<Job>>) {
    loop {
        // Le verrou n'est tenu que le temps de recevoir : les workers exécutent leurs tâches en parallèle.
        let job = match receiver.lock() {
            Ok(guard) => guard.recv(),
            Err(_) => return,
        };
        match job {
            Ok(job) => job(),
            Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    #[test]
    fn jobs_run_on_workers() {
        let workers = ChunkWorkers::start(2);
        let done = Arc::new(AtomicUsize::new(0));
        for _ in 0..8 {
            let done = Arc::clone(&done);
            assert!(workers.submit(Box::new(move || {
                done.fetch_add(1, Ordering::SeqCst);
            })));
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while done.load(Ordering::SeqCst) < 8 && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(done.load(Ordering::SeqCst), 8);
    }
}
