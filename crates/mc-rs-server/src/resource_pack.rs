//! Resource pack manager — chargement depuis `resource_packs/<id>/`.
//!
//! Format minimal : un dossier par pack contenant `manifest.json` (UUID +
//! version + name) + assets. Le serveur calcule le SHA-256 + sert le pack
//! en chunks via `ResourcePackChunkDataPacket` à la demande du client.
//!
//! Wiring runtime (envoi des chunks selon ResourcePackClientResponse) à
//! brancher dans `connection/login.rs` quand on supportera vraiment les
//! resource packs.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePackHeader {
    pub uuid: String,
    pub version: [u32; 3],
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePackManifest {
    pub format_version: u32,
    pub header: ResourcePackHeader,
}

#[derive(Debug, Clone)]
pub struct ResourcePack {
    pub manifest: ResourcePackManifest,
    pub data: Vec<u8>,
    pub sha256: [u8; 32],
}

/// Charge un resource pack depuis un dossier (format ZIP `.mcpack` ou dossier).
/// Retourne (Pack, raw_bytes_for_chunks).
pub fn load_pack(path: &Path) -> std::io::Result<ResourcePack> {
    let manifest_path = path.join("manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path)?;
    let manifest: ResourcePackManifest = serde_json::from_str(&manifest_text)
        .map_err(std::io::Error::other)?;

    // Pour l'instant on lit juste le manifest.json comme contenu — un vrai
    // pack zippe le dossier entier. À étendre via `zip` crate.
    let data = manifest_text.into_bytes();
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let sha = hasher.finalize();
    let mut sha256 = [0u8; 32];
    sha256.copy_from_slice(&sha);

    Ok(ResourcePack {
        manifest,
        data,
        sha256,
    })
}

/// Scan le dossier `resource_packs/` et charge tous les packs présents.
pub fn discover_packs(root: &Path) -> Vec<ResourcePack> {
    let mut packs = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return packs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match load_pack(&path) {
            Ok(p) => {
                tracing::info!(
                    "Loaded resource pack {} (uuid={}) from {:?}",
                    p.manifest.header.name,
                    p.manifest.header.uuid,
                    path
                );
                packs.push(p);
            }
            Err(e) => {
                tracing::warn!("Failed to load resource pack at {:?}: {}", path, e);
            }
        }
    }
    packs
}

/// Découpe le pack en chunks de la taille demandée par le client.
pub fn chunk_pack(data: &[u8], chunk_size: usize) -> Vec<&[u8]> {
    if chunk_size == 0 {
        return Vec::new();
    }
    data.chunks(chunk_size).collect()
}

pub fn pack_path() -> PathBuf {
    PathBuf::from("resource_packs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_pack_splits_evenly() {
        let data = vec![1u8; 1000];
        let chunks = chunk_pack(&data, 128);
        assert_eq!(chunks.len(), 8); // 7 full + 1 partial
        assert_eq!(chunks[0].len(), 128);
        assert_eq!(chunks[7].len(), 1000 - 7 * 128);
    }

    #[test]
    fn chunk_pack_zero_size_empty() {
        assert!(chunk_pack(&[1, 2, 3], 0).is_empty());
    }
}
