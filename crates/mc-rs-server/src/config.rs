use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

/// Default server.toml content with comments explaining each option.
const DEFAULT_CONFIG_TOML: &str = r#"# ══════════════════════════════════════════════════
#         MC-RS Server Configuration
# ══════════════════════════════════════════════════

[server]
# Message affiché dans la liste des serveurs
motd = "MC-RS Server"

# Sous-titre du serveur (deuxième ligne dans la liste)
sub_motd = "Powered by Rust"

# Port UDP d'écoute
port = 19132

# Nombre maximum de joueurs simultanés
max_players = 20

# Authentification Xbox Live (true = online, false = cracké)
online_mode = false

# Distance de vue maximale en chunks (2-32)
# Le client peut demander moins, mais jamais plus
view_distance = 16

# Intervalle de tick serveur en millisecondes (10 = 100 TPS)
tick_rate = 10

[world]
# Nom du dossier monde (stocké dans worlds/<name>/)
name = "world"

# Générateur de terrain : "normal" ou "flat"
generator = "normal"

# Seed du monde (0 = aléatoire). Même seed = même terrain.
seed = 0

[gameplay]
# Mode de jeu par défaut : "survival", "creative", "adventure", "spectator"
gamemode = "creative"

# Difficulté : "peaceful", "easy", "normal", "hard"
difficulty = "normal"

# Activer le PvP entre joueurs
pvp = true

# Activer le cycle jour/nuit (journées de 20 minutes)
do_daylight_cycle = true

# Activer les changements de météo (pluie, orage)
do_weather_cycle = false

# Rayon de protection du spawn en blocs (0 = désactivé)
spawn_protection = 16

[logging]
# Dossier où sont écrits les fichiers de log
directory = "logs"

# Politique de rotation : "daily" (nouveau fichier par jour), "hourly",
# "minutely" (debug), ou "never" (un seul fichier, jamais recyclé).
rotation = "daily"

# Nombre maximum d'archives à conserver (0 = illimité).
# Les plus anciennes sont supprimées automatiquement au-delà.
max_files = 14

# Niveau par défaut (format env_logger : "info", "debug,mc_rs_raknet=trace", ...).
# La variable d'environnement RUST_LOG prend toujours le dessus si elle est définie.
level = "info,mc_rs_raknet=debug"

# Écrire les logs sur stdout (console)
stdout = true

# Écrire les logs dans le fichier `<directory>/server.<DATE>.log`
file = true

# Colorer les logs stdout avec ANSI (désactiver si le terminal ne gère pas)
ansi = true

[webui]
# Panel web d'administration (http://<bind>/). Créer un admin au premier boot
# via la page /setup si la DB est vide.
#
# ATTENTION : exposer ce panel sur un réseau non-loopback sans TLS = gros risque
# de sécurité (token + mot de passe transitent en clair). Pour exposer hors
# localhost : mettre derrière un reverse proxy (nginx/caddy) avec HTTPS, OU
# activer la section [webui.tls] ci-dessous.
enabled = false

# Adresse IP + port d'écoute (par défaut loopback uniquement)
bind = "127.0.0.1:8080"

# URL de connexion base de données :
#   "sqlite://webui.db"     — SQLite local (défaut, zéro setup)
#   "postgres://user:pass@host/db"   — nécessite feature 'postgres'
#   "mongodb://host:27017/webui"     — nécessite feature 'mongodb'
database_url = "sqlite://webui.db"

# Durée de vie des sessions JWT (heures)
session_duration_hours = 24

[webui.tls]
# HTTPS pour le panel (recommandé dès que bind != 127.0.0.1)
enabled = false
cert_path = ""
key_path = ""

[rcon]
# RCON Source-format : console distante TCP (compatible mcrcon, BungeeCord, etc.).
# ATTENTION : ne pas exposer sans firewall — auth = mot de passe en clair.
enabled = false
address = "127.0.0.1"
port = 25575
password = ""

[query]
# Query Gamespy v4 (UDP) : status serveur public (motd, joueurs, version).
# Utilisé par les outils tiers (BungeeCord, GameTracker, etc.). Lecture seule.
enabled = false
address = "0.0.0.0"
port = 19132

[resource_pack]
# Si `true`, le client doit accepter le pack pour pouvoir jouer
# (recommandé quand le pack fournit des overrides UI structurels
# comme `server_form.json` — Bedrock les applique alors en priorité).
# Si `false`, le client peut télécharger le pack mais Bedrock peut
# silencieusement ignorer les overrides UI qu'il juge incertains.
must_accept = true
"#;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server")]
    pub server: ServerSection,
    #[serde(default)]
    pub world: WorldSection,
    #[serde(default)]
    pub gameplay: GameplaySection,
    #[serde(default)]
    pub logging: LoggingSection,
    /// Section `[webui]` — panel d'administration web optionnel.
    #[serde(default)]
    pub webui: mc_rs_webui::WebUiConfig,
    #[serde(default)]
    pub rcon: RconSection,
    #[serde(default)]
    pub query: QuerySection,
    #[serde(default)]
    pub resource_pack: ResourcePackSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourcePackSection {
    /// Si `true`, le client doit accepter le pack pour pouvoir jouer
    /// (Bedrock applique alors les overrides UI structurels en priorité).
    /// Si `false`, le pack reste optionnel — le client peut DL le pack mais
    /// Bedrock ignorera silencieusement les overrides UI qu'il juge incertains.
    #[serde(default = "default_pack_must_accept")]
    pub must_accept: bool,
}

fn default_pack_must_accept() -> bool {
    true
}

impl Default for ResourcePackSection {
    fn default() -> Self {
        Self {
            must_accept: default_pack_must_accept(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RconSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rcon_address")]
    pub address: String,
    #[serde(default = "default_rcon_port")]
    pub port: u16,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuerySection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_query_address")]
    pub address: String,
    #[serde(default = "default_query_port")]
    pub port: u16,
}

fn default_rcon_address() -> String {
    "127.0.0.1".to_string()
}
fn default_rcon_port() -> u16 {
    25575
}
fn default_query_address() -> String {
    "0.0.0.0".to_string()
}
fn default_query_port() -> u16 {
    19132
}

impl Default for RconSection {
    fn default() -> Self {
        Self {
            enabled: false,
            address: default_rcon_address(),
            port: default_rcon_port(),
            password: String::new(),
        }
    }
}

impl Default for QuerySection {
    fn default() -> Self {
        Self {
            enabled: false,
            address: default_query_address(),
            port: default_query_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingSection {
    #[serde(default = "default_log_dir")]
    pub directory: String,
    #[serde(default = "default_log_rotation")]
    pub rotation: String,
    #[serde(default = "default_log_max_files")]
    pub max_files: usize,
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_true")]
    pub stdout: bool,
    #[serde(default = "default_true")]
    pub file: bool,
    #[serde(default = "default_true")]
    pub ansi: bool,
}

#[derive(Debug, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_motd")]
    pub motd: String,
    #[serde(default = "default_sub_motd")]
    pub sub_motd: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default)]
    pub online_mode: bool,
    #[serde(default = "default_view_distance")]
    pub view_distance: i32,
    #[serde(default = "default_tick_rate")]
    pub tick_rate: u64,
}

#[derive(Debug, Deserialize)]
pub struct WorldSection {
    #[serde(default = "default_world_name")]
    pub name: String,
    #[serde(default = "default_generator")]
    pub generator: String,
    #[serde(default)]
    pub seed: i64,
}

#[derive(Debug, Deserialize)]
pub struct GameplaySection {
    #[serde(default = "default_gamemode")]
    pub gamemode: String,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default = "default_true")]
    pub pvp: bool,
    #[serde(default = "default_true")]
    pub do_daylight_cycle: bool,
    #[serde(default)]
    pub do_weather_cycle: bool,
    #[serde(default = "default_spawn_protection")]
    pub spawn_protection: i32,
}

/// Subset of config values needed by each Connection.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub default_gamemode: i32,
    pub difficulty: i32,
    pub world_name: String,
    pub max_view_distance: i32,
    pub generator_id: i32, // 1=infinite, 2=flat for StartGame
    pub world_seed: u64,
    pub resource_pack_must_accept: bool,
}

fn default_server() -> ServerSection {
    ServerSection {
        motd: default_motd(),
        sub_motd: default_sub_motd(),
        port: default_port(),
        max_players: default_max_players(),
        online_mode: false,
        view_distance: default_view_distance(),
        tick_rate: default_tick_rate(),
    }
}

fn default_motd() -> String {
    "MC-RS Server".to_string()
}
fn default_sub_motd() -> String {
    "Powered by Rust".to_string()
}
fn default_port() -> u16 {
    19132
}
fn default_max_players() -> u32 {
    20
}
fn default_view_distance() -> i32 {
    16
}
fn default_tick_rate() -> u64 {
    10
} // ms per tick (100 TPS)
fn default_world_name() -> String {
    "world".to_string()
}
fn default_generator() -> String {
    "normal".to_string()
}
fn default_gamemode() -> String {
    "survival".to_string()
}
fn default_difficulty() -> String {
    "normal".to_string()
}
fn default_true() -> bool {
    true
}
fn default_spawn_protection() -> i32 {
    16
}
fn default_log_dir() -> String {
    "logs".to_string()
}
fn default_log_rotation() -> String {
    "daily".to_string()
}
fn default_log_max_files() -> usize {
    14
}
fn default_log_level() -> String {
    "info,mc_rs_raknet=debug".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: default_server(),
            world: WorldSection::default(),
            gameplay: GameplaySection::default(),
            logging: LoggingSection::default(),
            webui: mc_rs_webui::WebUiConfig::default(),
            rcon: RconSection::default(),
            query: QuerySection::default(),
            resource_pack: ResourcePackSection::default(),
        }
    }
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            directory: default_log_dir(),
            rotation: default_log_rotation(),
            max_files: default_log_max_files(),
            level: default_log_level(),
            stdout: true,
            file: true,
            ansi: true,
        }
    }
}

impl Default for WorldSection {
    fn default() -> Self {
        Self {
            name: default_world_name(),
            generator: default_generator(),
            seed: 0,
        }
    }
}

impl Default for GameplaySection {
    fn default() -> Self {
        Self {
            gamemode: default_gamemode(),
            difficulty: default_difficulty(),
            pvp: true,
            do_daylight_cycle: true,
            do_weather_cycle: false,
            spawn_protection: default_spawn_protection(),
        }
    }
}

impl GameplaySection {
    pub fn gamemode_id(&self) -> i32 {
        match self.gamemode.to_lowercase().as_str() {
            "survival" | "0" => 0,
            "creative" | "1" => 1,
            "adventure" | "2" => 2,
            "spectator" | "3" => 3,
            _ => 1, // default creative
        }
    }

    pub fn gamemode_display(&self) -> &str {
        match self.gamemode.to_lowercase().as_str() {
            "survival" | "0" => "Survival",
            "creative" | "1" => "Creative",
            "adventure" | "2" => "Adventure",
            "spectator" | "3" => "Spectator",
            _ => "Creative",
        }
    }

    pub fn difficulty_id(&self) -> i32 {
        match self.difficulty.to_lowercase().as_str() {
            "peaceful" | "0" => 0,
            "easy" | "1" => 1,
            "normal" | "2" => 2,
            "hard" | "3" => 3,
            _ => 2, // default normal
        }
    }
}

/// Notes d'amorçage produites pendant `ServerConfig::load`, à émettre via
/// `tracing` **après** l'initialisation du subsystem de logs.
///
/// La config doit être chargée avant d'initialiser les logs (la section
/// `[logging]` pilote le niveau, la rotation, etc.), mais `tracing` n'est pas
/// encore prêt — on stocke donc ici les messages en attente.
#[derive(Debug, Default)]
pub struct ConfigBootstrapNotes {
    pub info: Vec<String>,
    pub warn: Vec<String>,
}

impl ConfigBootstrapNotes {
    pub fn flush(self) {
        for m in self.info {
            info!("{m}");
        }
        for m in self.warn {
            tracing::warn!("{m}");
        }
    }
}

impl ServerConfig {
    /// Charge `server.toml` sans émettre de logs (indispensable : cette fonction
    /// tourne avant l'init de `tracing`). Retourne la config et des notes à
    /// `flush()` après l'init.
    pub fn load(path: &str) -> (Self, ConfigBootstrapNotes) {
        let mut notes = ConfigBootstrapNotes::default();

        if !std::path::Path::new(path).exists() {
            match std::fs::write(path, DEFAULT_CONFIG_TOML) {
                Ok(()) => notes.info.push(format!("Generated default {path}")),
                Err(e) => notes
                    .warn
                    .push(format!("Could not write default {path}: {e}")),
            }
        }

        let config = match std::fs::read_to_string(path) {
            Ok(content) => {
                let parsed: Self = toml::from_str(&content).unwrap_or_default();
                notes.info.push(format!(
                    "Config loaded from {}\n  Server: {}:{} (max {} players)\n  World: \"{}\" ({}, seed={})\n  Gameplay: {}, {}, daylight={}, weather={}",
                    path,
                    parsed.server.motd,
                    parsed.server.port,
                    parsed.server.max_players,
                    parsed.world.name,
                    parsed.world.generator,
                    parsed.world.seed,
                    parsed.gameplay.gamemode,
                    parsed.gameplay.difficulty,
                    if parsed.gameplay.do_daylight_cycle { "on" } else { "off" },
                    if parsed.gameplay.do_weather_cycle { "on" } else { "off" },
                ));
                parsed
            }
            Err(e) => {
                notes
                    .warn
                    .push(format!("Could not read {path}: {e}, using defaults"));
                Self::default()
            }
        };

        (config, notes)
    }

    /// Resolve the effective world seed.
    ///
    /// If `world.seed` is non-zero, it is used as the source of truth for new
    /// worlds. If it is zero, a random seed is generated once and persisted in
    /// the world directory so the same world stays stable across restarts until
    /// the directory is deleted.
    pub fn resolve_world_seed(&self, world_dir: &Path) -> u64 {
        let seed_path = world_dir.join("level_seed.txt");

        if let Ok(contents) = std::fs::read_to_string(&seed_path) {
            if let Ok(parsed) = contents.trim().parse::<u64>() {
                info!("Using persisted world seed {} from {:?}", parsed, seed_path);
                return parsed;
            }
        }

        let seed = if self.world.seed != 0 {
            self.world.seed as u64
        } else {
            rand::random::<u64>()
        };

        if let Err(e) = std::fs::create_dir_all(world_dir) {
            info!("Could not create world dir {:?}: {}", world_dir, e);
        } else if let Err(e) = std::fs::write(&seed_path, format!("{seed}\n")) {
            info!("Could not persist world seed to {:?}: {}", seed_path, e);
        } else {
            info!("Persisted world seed {} to {:?}", seed, seed_path);
        }

        seed
    }

    /// Build a ConnectionConfig from the full server config.
    pub fn connection_config(&self, world_seed: u64) -> Arc<ConnectionConfig> {
        Arc::new(ConnectionConfig {
            default_gamemode: self.gameplay.gamemode_id(),
            difficulty: self.gameplay.difficulty_id(),
            world_name: self.world.name.clone(),
            max_view_distance: self.server.view_distance,
            generator_id: match self.world.generator.to_lowercase().as_str() {
                "flat" => 2,             // flat
                "normal" | "noise" => 1, // infinite
                _ => 1,
            },
            world_seed,
            resource_pack_must_accept: self.resource_pack.must_accept,
        })
    }
}
