//! `mc-rs-sim` — orchestrateur de simulation de joueurs.
//!
//! Lance une population de joueurs simulés contre un serveur mc-rs et rapporte
//! les métriques de connexion. Cible par défaut : `127.0.0.1:19132`.
//!
//! Usage :
//! ```text
//! mc-rs-sim --target 127.0.0.1:19132 --players 10 --rate 5 --duration 20
//! ```

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::task::JoinSet;
use tracing::info;
use tracing_subscriber::EnvFilter;

use mc_rs_sim::metrics::{Sampler, ServerMetrics};
use mc_rs_sim::player::Behavior;
use mc_rs_sim::{SimConfig, SimOutcome, SimPlayer};

struct Args {
    target: SocketAddr,
    players: usize,
    rate: f64,
    duration: Duration,
    mtu: u16,
    behavior: Behavior,
    /// Adresse du webui pour scraper le TPS (`/api/health`). None = désactivé.
    webui: Option<SocketAddr>,
    /// Chemin d'export CSV des résultats par joueur.
    csv: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            target: "127.0.0.1:19132".parse().unwrap(),
            players: 1,
            rate: 0.0, // 0 = tous d'un coup
            duration: Duration::from_secs(15),
            mtu: 1400,
            behavior: Behavior::Wander,
            webui: Some("127.0.0.1:8080".parse().unwrap()),
            csv: None,
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut next = || {
            it.next()
                .ok_or_else(|| format!("valeur manquante après {flag}"))
        };
        match flag.as_str() {
            "--target" | "-t" => {
                args.target = next()?
                    .parse()
                    .map_err(|e| format!("adresse invalide: {e}"))?;
            }
            "--players" | "-p" => {
                args.players = next()?.parse().map_err(|e| format!("--players: {e}"))?;
            }
            "--rate" | "-r" => {
                args.rate = next()?.parse().map_err(|e| format!("--rate: {e}"))?;
            }
            "--duration" | "-d" => {
                let secs: u64 = next()?.parse().map_err(|e| format!("--duration: {e}"))?;
                args.duration = Duration::from_secs(secs);
            }
            "--mtu" => {
                args.mtu = next()?.parse().map_err(|e| format!("--mtu: {e}"))?;
            }
            "--behavior" | "-b" => {
                let v = next()?;
                args.behavior =
                    Behavior::parse(&v).ok_or_else(|| format!("--behavior invalide: {v}"))?;
            }
            "--webui" => {
                let v = next()?;
                args.webui = if v.eq_ignore_ascii_case("off") || v.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(v.parse().map_err(|e| format!("--webui: {e}"))?)
                };
            }
            "--csv" => {
                args.csv = Some(next()?);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("argument inconnu: {other}")),
        }
    }
    Ok(args)
}

fn print_help() {
    println!(
        "mc-rs-sim — simulation de population de joueurs (serveur mc-rs)\n\n\
         OPTIONS:\n\
         \x20 -t, --target <ADDR>     Serveur cible (défaut 127.0.0.1:19132)\n\
         \x20 -p, --players <N>       Nombre de joueurs simulés (défaut 1)\n\
         \x20 -r, --rate <N/s>        Cadence de connexion, joueurs/seconde (défaut: tous d'un coup)\n\
         \x20 -d, --duration <SECS>   Durée de vie de chaque joueur (défaut 15)\n\
         \x20     --mtu <N>           MTU RakNet visée (défaut 1400)\n\
         \x20 -b, --behavior <B>      Comportement: idle | wander | active (défaut wander)\n\
         \x20     --webui <ADDR|off>  Adresse webui pour le TPS (défaut 127.0.0.1:8080)\n\
         \x20     --csv <FICHIER>     Export CSV des résultats par joueur\n\
         \x20 -h, --help              Cette aide"
    );
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Erreur: {e}\n");
            print_help();
            std::process::exit(2);
        }
    };

    info!(
        "Simulation: {} joueur(s) → {} (durée {}s, mtu {}, comportement {:?}{})",
        args.players,
        args.target,
        args.duration.as_secs(),
        args.mtu,
        args.behavior,
        if args.rate > 0.0 {
            format!(", cadence {}/s", args.rate)
        } else {
            String::new()
        }
    );

    let stagger = if args.rate > 0.0 {
        Duration::from_secs_f64(1.0 / args.rate)
    } else {
        Duration::ZERO
    };

    // Démarre l'échantillonnage des ressources serveur (TPS via webui + CPU/RAM
    // du process) pour toute la durée de la simulation.
    let sampler = args
        .webui
        .map(|addr| Sampler::start(addr, "mc-rs-server".to_string()));

    let run_start = Instant::now();
    let mut set: JoinSet<SimOutcome> = JoinSet::new();

    for i in 0..args.players {
        let cfg = SimConfig {
            server_addr: args.target,
            name: format!("Sim{i:04}"),
            mtu: args.mtu,
            behavior: args.behavior,
        };
        let duration = args.duration;
        set.spawn(async move {
            match SimPlayer::connect(cfg.clone()).await {
                Ok(mut p) => p.run(duration).await,
                Err(e) => mc_rs_sim::SimOutcome {
                    name: cfg.name,
                    handshake: None,
                    login: None,
                    spawn: None,
                    chunks: 0,
                    inputs_sent: 0,
                    chats_sent: 0,
                    last_rtt_ms: None,
                    error: Some(e.to_string()),
                },
            }
        });

        if !stagger.is_zero() && i + 1 < args.players {
            tokio::time::sleep(stagger).await;
        }
    }

    // Collecte des résultats.
    let mut outcomes = Vec::with_capacity(args.players);
    while let Some(res) = set.join_next().await {
        if let Ok(o) = res {
            outcomes.push(o);
        }
    }

    // Stoppe le sampler et récupère les séries serveur.
    let server_metrics = match sampler {
        Some(s) => s.stop().await,
        None => ServerMetrics::default(),
    };

    report(&outcomes, run_start.elapsed(), &server_metrics);

    if let Some(path) = args.csv {
        match write_csv(&path, &outcomes) {
            Ok(()) => println!("CSV écrit : {path}"),
            Err(e) => eprintln!("Erreur écriture CSV {path}: {e}"),
        }
    }
}

/// Écrit les résultats par joueur en CSV.
fn write_csv(path: &str, outcomes: &[SimOutcome]) -> std::io::Result<()> {
    use std::io::Write;
    let ms = |d: Option<Duration>| {
        d.map(|d| format!("{:.1}", d.as_secs_f64() * 1000.0))
            .unwrap_or_default()
    };
    let mut f = std::fs::File::create(path)?;
    writeln!(
        f,
        "name,handshake_ms,login_ms,spawn_ms,chunks,inputs,chats,rtt_ms,error"
    )?;
    for o in outcomes {
        writeln!(
            f,
            "{},{},{},{},{},{},{},{},{}",
            o.name,
            ms(o.handshake),
            ms(o.login),
            ms(o.spawn),
            o.chunks,
            o.inputs_sent,
            o.chats_sent,
            o.last_rtt_ms.map(|r| format!("{r:.1}")).unwrap_or_default(),
            o.error.as_deref().unwrap_or("").replace(',', ";"),
        )?;
    }
    Ok(())
}

fn report(outcomes: &[SimOutcome], wall: Duration, server: &ServerMetrics) {
    let total = outcomes.len();
    let connected: Vec<_> = outcomes.iter().filter(|o| o.handshake.is_some()).collect();
    let failed = total - connected.len();

    let mut hs_ms: Vec<f64> = connected
        .iter()
        .filter_map(|o| o.handshake.map(|d| d.as_secs_f64() * 1000.0))
        .collect();
    hs_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let pct = |v: &[f64], p: f64| -> f64 {
        if v.is_empty() {
            return 0.0;
        }
        let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
        v[idx]
    };
    let avg = |v: &[f64]| -> f64 {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };

    let logged_in = outcomes.iter().filter(|o| o.login.is_some()).count();
    let in_game = outcomes.iter().filter(|o| o.spawn.is_some()).count();
    let ms_of = |f: &dyn Fn(&mc_rs_sim::SimOutcome) -> Option<Duration>| -> Vec<f64> {
        let mut v: Vec<f64> = outcomes
            .iter()
            .filter_map(|o| f(o).map(|d| d.as_secs_f64() * 1000.0))
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    };
    let login_ms = ms_of(&|o| o.login);
    let spawn_ms = ms_of(&|o| o.spawn);
    let total_chunks: usize = outcomes.iter().map(|o| o.chunks).sum();

    let rtts: Vec<f64> = outcomes.iter().filter_map(|o| o.last_rtt_ms).collect();

    let line = |label: &str, v: &[f64]| {
        if !v.is_empty() {
            println!(
                "{label:<18}: min {:.1} | p50 {:.1} | p95 {:.1} | max {:.1} | avg {:.1}",
                v.first().copied().unwrap_or(0.0),
                pct(v, 0.50),
                pct(v, 0.95),
                v.last().copied().unwrap_or(0.0),
                avg(v),
            );
        }
    };

    println!("\n──────────── Rapport de simulation ────────────");
    println!("Joueurs           : {total}");
    println!(
        "RakNet connectés  : {} ({:.0}%)",
        connected.len(),
        100.0 * connected.len() as f64 / total.max(1) as f64
    );
    println!(
        "Login chiffré     : {} ({:.0}%)",
        logged_in,
        100.0 * logged_in as f64 / total.max(1) as f64
    );
    println!(
        "InGame            : {} ({:.0}%)",
        in_game,
        100.0 * in_game as f64 / total.max(1) as f64
    );
    println!("Chunks reçus      : {total_chunks}");
    let total_inputs: u64 = outcomes.iter().map(|o| o.inputs_sent).sum();
    let total_chats: u64 = outcomes.iter().map(|o| o.chats_sent).sum();
    println!("PlayerAuthInput   : {total_inputs} envoyés");
    if total_chats > 0 {
        println!("Chat              : {total_chats} messages");
    }
    println!("Échecs            : {failed}");
    println!("Temps mur         : {:.2}s", wall.as_secs_f64());
    line("Handshake (ms)", &hs_ms);
    line("Login (ms)", &login_ms);
    line("Spawn→InGame (ms)", &spawn_ms);
    if !rtts.is_empty() {
        println!(
            "RTT final (ms)    : p50 {:.1} | avg {:.1}",
            pct(
                &{
                    let mut r = rtts.clone();
                    r.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    r
                },
                0.50
            ),
            avg(&rtts)
        );
    }
    // ── Ressources serveur pendant la simulation ──
    if !server.is_empty() {
        println!("─── Serveur sous charge ───");
        let f32min = |v: &[f32]| v.iter().cloned().fold(f32::INFINITY, f32::min);
        let f32max = |v: &[f32]| v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let f32avg = |v: &[f32]| {
            if v.is_empty() {
                0.0
            } else {
                v.iter().sum::<f32>() / v.len() as f32
            }
        };
        if !server.tps.is_empty() {
            println!(
                "TPS               : min {:.1} | avg {:.1} | max {:.1} (cible 20)",
                f32min(&server.tps),
                f32avg(&server.tps),
                f32max(&server.tps),
            );
        }
        if let Some(max_players) = server.players.iter().max() {
            println!("Joueurs vus serveur: max {max_players}");
        }
        if !server.cpu.is_empty() {
            println!(
                "CPU serveur (%)   : avg {:.0} | max {:.0}",
                f32avg(&server.cpu),
                f32max(&server.cpu),
            );
        }
        if !server.ram_mb.is_empty() {
            println!(
                "RAM serveur (Mo)  : avg {:.0} | max {:.0}",
                f32avg(&server.ram_mb),
                f32max(&server.ram_mb),
            );
        }
    }

    // Erreurs distinctes (échec handshake OU login OU rupture en cours de vie).
    use std::collections::BTreeMap;
    let mut errs: BTreeMap<String, usize> = BTreeMap::new();
    for o in outcomes.iter().filter(|o| o.error.is_some()) {
        *errs
            .entry(o.error.clone().unwrap_or_else(|| "inconnue".into()))
            .or_default() += 1;
    }
    if !errs.is_empty() {
        println!("Erreurs           :");
        for (e, n) in errs {
            println!("  {n}× {e}");
        }
    }
    println!("───────────────────────────────────────────────");
}
