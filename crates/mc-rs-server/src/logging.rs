//! Centralized logging initialization.
//!
//! Inspired by PocketMine-MP `MainLogger` + `MainLoggerThread` :
//! - Sortie simultanée stdout + fichier, chacune pouvant être coupée
//! - ANSI coloré configurable côté stdout
//! - Rotation configurable (daily / hourly / minutely / never) + limite d'archives
//! - Writer fichier non-bloquant (thread dédié, `tracing-appender`)
//! - Format de ligne : `HH:MM:SS.mmm  LEVEL  target: message`
//! - Niveau par défaut via config ; `RUST_LOG` env var prend toujours le dessus

use std::fmt as stdfmt;
use std::path::Path;

use tokio::sync::broadcast;
use tracing::field::Visit;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer, Registry};

use crate::config::LoggingSection;

/// Horloge locale formatée `HH:MM:SS.mmm` (comme PMMP `MainLogger`).
struct LocalClock;

impl FormatTime for LocalClock {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%H:%M:%S%.3f"))
    }
}

/// Garde à garder vivante pendant toute la durée du process : le `Drop` du
/// `WorkerGuard` interne flush le writer non-bloquant et ferme le fichier.
#[must_use = "The LogGuard must stay alive for the entire process lifetime; drop flushes pending logs."]
pub struct LogGuard {
    _file_worker: Option<WorkerGuard>,
}

fn parse_rotation(value: &str) -> Rotation {
    match value.to_ascii_lowercase().as_str() {
        "minutely" => Rotation::MINUTELY,
        "hourly" => Rotation::HOURLY,
        "daily" => Rotation::DAILY,
        "never" => Rotation::NEVER,
        _ => Rotation::DAILY,
    }
}

/// Initialise le sous-système de logs à partir de la section `[logging]` de
/// `server.toml`.
///
/// - `RUST_LOG` (env var) prend le dessus sur `cfg.level` si définie.
/// - `cfg.stdout = false` coupe totalement la sortie console.
/// - `cfg.file   = false` désactive l'écriture sur disque.
/// - `cfg.rotation = "never"` produit un seul fichier `server.log` sans date.
///
/// À appeler **une seule fois** en tout début de `main()`, *après* avoir chargé
/// la config (voir `ServerConfig::load`), *avant* toute autre initialisation.
pub fn init(
    cfg: &LoggingSection,
    webui_log_tx: Option<broadcast::Sender<mc_rs_webui::LogLine>>,
) -> LogGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cfg.level));

    let stdout_layer = cfg.stdout.then(|| {
        fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(cfg.ansi)
            .with_timer(LocalClock)
            .with_target(true)
            .with_level(true)
            .boxed()
    });

    let (file_layer, file_worker) = if cfg.file {
        let dir = Path::new(&cfg.directory);
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!(
                "warning: failed to create log directory {}: {e}",
                dir.display()
            );
        }

        let rotation = parse_rotation(&cfg.rotation);
        let mut builder = RollingFileAppender::builder()
            .rotation(rotation)
            .filename_prefix("server")
            .filename_suffix("log");
        if cfg.max_files > 0 {
            builder = builder.max_log_files(cfg.max_files);
        }
        let appender = builder
            .build(dir)
            .expect("failed to initialize rolling file log appender");

        let (writer, guard) = tracing_appender::non_blocking(appender);
        let layer = fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .with_timer(LocalClock)
            .with_target(true)
            .with_level(true)
            .boxed();
        (Some(layer), Some(guard))
    } else {
        (None, None)
    };

    let webui_layer = webui_log_tx.map(|tx| WebUiBroadcastLayer { tx }.boxed());

    Registry::default()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(webui_layer)
        .init();

    LogGuard {
        _file_worker: file_worker,
    }
}

/// Layer tracing qui sérialise chaque `Event` en `mc_rs_webui::LogLine` et
/// push dans un `broadcast::Sender`. Non-bloquant : si le channel est full
/// (aucun receiver ou lag), la ligne est silencieusement droppée — priorité à
/// ne pas freiner la main loop pour un panel admin.
struct WebUiBroadcastLayer {
    tx: broadcast::Sender<mc_rs_webui::LogLine>,
}

struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn stdfmt::Debug) {
        if field.name() == "message" {
            use std::fmt::Write;
            let _ = write!(&mut self.message, "{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        }
    }
}

impl<S: tracing::Subscriber> Layer<S> for WebUiBroadcastLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);
        let meta = event.metadata();
        let line = mc_rs_webui::LogLine {
            ts: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            level: meta.level().to_string(),
            target: meta.target().to_string(),
            message: visitor.message,
        };
        // send renvoie Err uniquement si aucun receiver — on ignore.
        let _ = self.tx.send(line);
    }
}

/// Installe un hook de panic qui log via `tracing::error!` + bannière stdout.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());

        tracing::error!(target: "panic", "thread panicked at {location}: {payload}");

        let banner = format!("\n==== PANIC ====\nat {location}\n{payload}\n================\n");
        let _ = std::io::Write::write_all(&mut std::io::stdout(), banner.as_bytes());
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let _ = std::io::Write::write_all(&mut std::io::stderr(), banner.as_bytes());
    }));
}
