//! Échantillonnage des ressources serveur pendant une simulation.
//!
//! Deux sources, à 1 Hz :
//! - **TPS + joueurs en ligne** : scrape `GET /api/health` du webui (endpoint
//!   non authentifié). C'est LA métrique de charge — la dégradation du TPS sous
//!   population croissante est le signal qu'on cherche.
//! - **CPU% + RAM du process serveur** : via `sysinfo`, en repérant le process
//!   par son nom.
//!
//! Le sampler tourne en tâche de fond et s'arrête sur signal ; il rend les
//! séries collectées pour agrégation dans le rapport.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::debug;

/// Séries temporelles collectées pendant la simulation.
#[derive(Debug, Default, Clone)]
pub struct ServerMetrics {
    /// TPS échantillonné (via /api/health).
    pub tps: Vec<f32>,
    /// Joueurs en ligne vus par le serveur.
    pub players: Vec<usize>,
    /// CPU% du process serveur (100% = un cœur plein ; peut dépasser 100%).
    pub cpu: Vec<f32>,
    /// RAM (RSS) du process serveur, en Mo.
    pub ram_mb: Vec<f32>,
}

impl ServerMetrics {
    pub fn is_empty(&self) -> bool {
        self.tps.is_empty() && self.cpu.is_empty()
    }
}

/// Un échantillon /api/health.
struct HealthSample {
    tps: f32,
    players: usize,
}

/// Scrape `GET /api/health` en HTTP/1.1 minimal (TCP std, sans client lourd).
async fn fetch_health(addr: SocketAddr) -> Option<HealthSample> {
    let connect = timeout(Duration::from_secs(2), TcpStream::connect(addr))
        .await
        .ok()?
        .ok()?;
    let mut stream = connect;
    let req = format!(
        "GET /api/health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).await.ok()?;

    let mut buf = Vec::with_capacity(512);
    timeout(Duration::from_secs(2), stream.read_to_end(&mut buf))
        .await
        .ok()?
        .ok()?;

    let text = String::from_utf8_lossy(&buf);
    // Corps après l'en-tête HTTP.
    let body = text.split("\r\n\r\n").nth(1)?.trim();
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    Some(HealthSample {
        tps: v.get("tps").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
        players: v
            .get("players_online")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
    })
}

/// Sampler en tâche de fond.
pub struct Sampler {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<ServerMetrics>,
}

impl Sampler {
    /// Démarre l'échantillonnage. `webui` = adresse du tableau de bord
    /// (`/api/health`) ; `process_name` = motif du nom de process serveur à
    /// suivre pour CPU/RAM (ex. "mc-rs-server").
    pub fn start(webui: SocketAddr, process_name: String) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_task = stop.clone();

        let handle = tokio::spawn(async move {
            use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

            let mut metrics = ServerMetrics::default();

            // Setup sysinfo : on ne rafraîchit que CPU + mémoire des process.
            let mut sys = System::new_with_specifics(
                RefreshKind::new()
                    .with_processes(ProcessRefreshKind::new().with_cpu().with_memory()),
            );
            sys.refresh_processes(ProcessesToUpdate::All, true);
            let pid = sys
                .processes()
                .iter()
                .find(|(_, p)| {
                    p.name()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains(&process_name.to_ascii_lowercase())
                })
                .map(|(pid, _)| *pid);

            if pid.is_none() {
                debug!("process serveur '{process_name}' introuvable — CPU/RAM non suivis");
            }

            while !stop_task.load(Ordering::Relaxed) {
                if let Some(h) = fetch_health(webui).await {
                    metrics.tps.push(h.tps);
                    metrics.players.push(h.players);
                }
                if let Some(pid) = pid {
                    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
                    if let Some(p) = sys.process(pid) {
                        metrics.cpu.push(p.cpu_usage());
                        metrics.ram_mb.push(p.memory() as f32 / (1024.0 * 1024.0));
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            metrics
        });

        Self { stop, handle }
    }

    /// Arrête le sampler et récupère les séries collectées.
    pub async fn stop(self) -> ServerMetrics {
        self.stop.store(true, Ordering::Relaxed);
        self.handle.await.unwrap_or_default()
    }
}
