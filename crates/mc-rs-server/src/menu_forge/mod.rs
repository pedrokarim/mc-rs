//! Moteur d’exécution des menus **Menu Forge** sur Bedrock (rendu « formulaire
//! dessiné »).
//!
//! Le studio Menu Forge exporte, dans un dossier réglé par `[menu_forge]` de
//! `server.toml`, un pack de ressources (`pack/`, dispositions JSON UI) et un
//! descripteur d’exécution (`runtime.json`). Ce module charge le descripteur,
//! compose le formulaire d’un menu (titre à jetons, une entrée par case du
//! coffre, puis une par texte dynamique) et exécute les actions d’un clic.
//!
//! Contrat avec l’exporteur : `docs/bedrock.md` du dépôt Menu Forge.
//! Intégration : `connection/menu_forge.rs` (formulaires et effets),
//! `commands/mf.rs` (`/mf`), `/menu` ; documentation :
//! `dev-docs/32-MENU-FORGE.md`.

mod catalog;
mod condition;
#[cfg(test)]
mod export_tests;
mod session;
#[cfg(test)]
mod tests;

pub use catalog::{Catalog, CommandRunner, PackInfo};
pub use session::{Effect, NoLists, Session, Viewer};

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use tracing::{info, warn};

use crate::resource_pack::{load_pack, ResourcePack};

/// Drapeau de titre des menus Menu Forge : identique au `flag` du descripteur
/// et à `$flag_menu_forge` de `resource_packs/mcrs_ui/ui/server_form.json`.
pub const MENU_FORGE_FLAG: &str = "\u{00A7}m\u{00A7}v";
/// Marqueur d’une entrée inerte (icône affichée, pas de bouton).
pub const INERT_MARKER: &str = "{x}";
/// Début du marqueur d’une entrée de texte dynamique (`{t<k>}`).
pub const TEXT_MARKER: &str = "{t";
/// Descripteur d’exécution, dans le dossier d’export.
pub const RUNTIME_FILE: &str = "runtime.json";
/// Pack de ressources, dans le dossier d’export.
pub const PACK_DIR: &str = "pack";

/// Réglages `[menu_forge]` de `server.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuForgeSettings {
    /// Charger le descripteur et servir le pack exportés.
    pub enabled: bool,
    /// Dossier d’export du studio (`runtime.json` et `pack/`).
    pub directory: PathBuf,
    /// `/menu` passe par les menus Menu Forge quand ils existent (sinon, anciens formulaires).
    pub menu_command: bool,
}

impl MenuForgeSettings {
    /// Moteur désactivé : aucun menu, `/menu` garde ses anciens formulaires.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            directory: PathBuf::new(),
            menu_command: false,
        }
    }
}

/// Moteur partagé par tout le serveur : réglages et catalogue courant
/// (remplacé par `/mf reload`).
pub struct MenuForgeRuntime {
    settings: MenuForgeSettings,
    catalog: RwLock<Arc<Catalog>>,
}

impl MenuForgeRuntime {
    pub fn new(settings: MenuForgeSettings, catalog: Catalog) -> Self {
        Self {
            settings,
            catalog: RwLock::new(Arc::new(catalog)),
        }
    }

    pub fn settings(&self) -> &MenuForgeSettings {
        &self.settings
    }

    /// Catalogue courant (une session garde celui de son ouverture).
    pub fn catalog(&self) -> Arc<Catalog> {
        match self.catalog.read() {
            Ok(catalog) => Arc::clone(&catalog),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    /// Vrai si `/menu` doit ouvrir le menu Menu Forge `id` plutôt que l’ancien formulaire.
    pub fn serves_menu_command(&self, id: &str) -> bool {
        self.settings.enabled && self.settings.menu_command && self.catalog().menu(id).is_some()
    }

    /// Identifiants des menus, triés (complétion de `/mf open`).
    pub fn menu_ids(&self) -> Vec<String> {
        self.catalog().menu_ids()
    }

    /// Relit `runtime.json` (la logique des menus) ; le pack, envoyé à la
    /// connexion, n’est relu qu’au redémarrage du serveur.
    pub fn reload(&self) -> Result<String, String> {
        if !self.settings.enabled {
            return Err("Menu Forge est désactivé ([menu_forge] enabled = false).".to_string());
        }
        let catalog = Catalog::load(&self.settings.directory)?;
        let summary = catalog.summary();
        for problem in catalog.warnings() {
            warn!("[menu-forge] {problem}");
        }
        match self.catalog.write() {
            Ok(mut current) => *current = Arc::new(catalog),
            Err(poisoned) => *poisoned.into_inner() = Arc::new(catalog),
        }
        Ok(summary)
    }
}

static RUNTIME: OnceLock<MenuForgeRuntime> = OnceLock::new();

/// Installe le moteur au démarrage du serveur (un seul appel est pris en compte).
pub fn install(runtime: MenuForgeRuntime) {
    if RUNTIME.set(runtime).is_err() {
        warn!("[menu-forge] moteur déjà installé, second appel ignoré");
    }
}

/// Moteur installé ; à défaut, un moteur désactivé et vide.
pub fn shared() -> &'static MenuForgeRuntime {
    RUNTIME.get_or_init(|| MenuForgeRuntime::new(MenuForgeSettings::disabled(), Catalog::empty()))
}

/// Au démarrage : charge le descripteur et le pack exportés. Une erreur est
/// journalisée et laisse un catalogue vide (`/menu` garde alors ses anciens
/// formulaires) ; elle n’empêche jamais le serveur de démarrer.
pub fn load_at_startup(settings: MenuForgeSettings) -> (MenuForgeRuntime, Option<ResourcePack>) {
    if !settings.enabled {
        info!("[menu-forge] désactivé ([menu_forge] enabled = false)");
        return (MenuForgeRuntime::new(settings, Catalog::empty()), None);
    }
    let catalog = match Catalog::load(&settings.directory) {
        Ok(catalog) => {
            info!("[menu-forge] {}", catalog.summary());
            for problem in catalog.warnings() {
                warn!("[menu-forge] {problem}");
            }
            catalog
        }
        Err(error) => {
            warn!("[menu-forge] {error} : aucun menu chargé, /menu garde ses anciens formulaires");
            Catalog::empty()
        }
    };
    let pack = load_exported_pack(&settings.directory, catalog.pack());
    (MenuForgeRuntime::new(settings, catalog), pack)
}

fn load_exported_pack(directory: &Path, expected: Option<&PackInfo>) -> Option<ResourcePack> {
    let pack_dir = directory.join(PACK_DIR);
    if !pack_dir.join("manifest.json").is_file() {
        warn!("[menu-forge] pas de pack exporté dans {pack_dir:?} : les menus s’afficheraient en liste vanilla");
        return None;
    }
    match load_pack(&pack_dir) {
        Ok(pack) => {
            info!(
                "[menu-forge] pack « {} » {} chargé depuis {pack_dir:?}",
                pack.manifest.header.name,
                pack.version_string()
            );
            if let Some(expected) = expected {
                if expected.uuid != pack.uuid() || expected.version != pack.manifest.header.version
                {
                    warn!(
                        "[menu-forge] le pack ({} {}) ne correspond pas au descripteur ({} {}) : réexporter depuis le studio",
                        pack.uuid(),
                        pack.version_string(),
                        expected.uuid,
                        expected.version.map(|part| part.to_string()).join(".")
                    );
                }
            }
            Some(pack)
        }
        Err(error) => {
            warn!("[menu-forge] pack illisible dans {pack_dir:?} : {error}");
            None
        }
    }
}
