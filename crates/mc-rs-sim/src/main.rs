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

use mc_rs_sim::{SimConfig, SimPlayer};

struct Args {
    target: SocketAddr,
    players: usize,
    rate: f64,
    duration: Duration,
    mtu: u16,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            target: "127.0.0.1:19132".parse().unwrap(),
            players: 1,
            rate: 0.0, // 0 = tous d'un coup
            duration: Duration::from_secs(15),
            mtu: 1400,
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
        "Simulation: {} joueur(s) → {} (durée {}s, mtu {}{})",
        args.players,
        args.target,
        args.duration.as_secs(),
        args.mtu,
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

    let run_start = Instant::now();
    let mut set: JoinSet<mc_rs_sim::player::SimOutcome> = JoinSet::new();

    for i in 0..args.players {
        let cfg = SimConfig {
            server_addr: args.target,
            name: format!("Sim{i:04}"),
            mtu: args.mtu,
        };
        let duration = args.duration;
        set.spawn(async move {
            match SimPlayer::connect(cfg.clone()).await {
                Ok(mut p) => p.run(duration).await,
                Err(e) => mc_rs_sim::player::SimOutcome {
                    name: cfg.name,
                    handshake: None,
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

    report(&outcomes, run_start.elapsed());
}

fn report(outcomes: &[mc_rs_sim::player::SimOutcome], wall: Duration) {
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

    let rtts: Vec<f64> = outcomes.iter().filter_map(|o| o.last_rtt_ms).collect();

    println!("\n──────────── Rapport de simulation ────────────");
    println!("Joueurs           : {total}");
    println!(
        "Connectés         : {} ({:.0}%)",
        connected.len(),
        100.0 * connected.len() as f64 / total.max(1) as f64
    );
    println!("Échecs            : {failed}");
    println!("Temps mur         : {:.2}s", wall.as_secs_f64());
    if !hs_ms.is_empty() {
        println!(
            "Handshake (ms)    : min {:.1} | p50 {:.1} | p95 {:.1} | max {:.1} | avg {:.1}",
            hs_ms.first().copied().unwrap_or(0.0),
            pct(&hs_ms, 0.50),
            pct(&hs_ms, 0.95),
            hs_ms.last().copied().unwrap_or(0.0),
            avg(&hs_ms),
        );
    }
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
    if failed > 0 {
        // Compte les erreurs distinctes.
        use std::collections::BTreeMap;
        let mut errs: BTreeMap<String, usize> = BTreeMap::new();
        for o in outcomes.iter().filter(|o| o.handshake.is_none()) {
            *errs
                .entry(o.error.clone().unwrap_or_else(|| "inconnue".into()))
                .or_default() += 1;
        }
        println!("Erreurs           :");
        for (e, n) in errs {
            println!("  {n}× {e}");
        }
    }
    println!("───────────────────────────────────────────────");
}
