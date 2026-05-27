//! Resource pack loader — packs un dossier `resource_packs/<id>/` en ZIP
//! mémoire, calcule le SHA-256 et expose les chunks pour
//! `ResourcePackChunkData`.
//!
//! Le client Bedrock attend un ZIP (.mcpack est un ZIP renommé) contenant
//! au minimum `manifest.json` à la racine. La taille annoncée dans
//! `ResourcePacksInfo` + `ResourcePackDataInfo` doit correspondre exactement
//! à `data.len()`.

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::FileOptions;
use zip::CompressionMethod;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePackHeader {
    pub uuid: String,
    pub version: [u32; 3],
    pub name: String,
    #[serde(default)]
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
    /// ZIP raw bytes — c'est ce que le client télécharge.
    pub data: Vec<u8>,
    pub sha256: [u8; 32],
}

impl ResourcePack {
    pub fn uuid(&self) -> &str {
        &self.manifest.header.uuid
    }

    pub fn version_string(&self) -> String {
        let v = self.manifest.header.version;
        format!("{}.{}.{}", v[0], v[1], v[2])
    }

    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn sha256_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for b in self.sha256 {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }

    /// Renvoie le slice du `chunk_index` selon `chunk_size`.
    pub fn chunk(&self, chunk_index: u32, chunk_size: usize) -> &[u8] {
        let start = (chunk_index as usize) * chunk_size;
        if start >= self.data.len() {
            return &[];
        }
        let end = (start + chunk_size).min(self.data.len());
        &self.data[start..end]
    }
}

/// Charge un resource pack depuis un dossier — zippe son contenu en mémoire.
/// Si `path` pointe vers un fichier `.zip`/`.mcpack`, lit le ZIP raw.
pub fn load_pack(path: &Path) -> std::io::Result<ResourcePack> {
    let (manifest_text, data) = if path.is_dir() {
        let manifest_path = path.join("manifest.json");
        let manifest_text = std::fs::read_to_string(&manifest_path)?;
        let data = zip_directory(path)?;
        (manifest_text, data)
    } else {
        // Fichier .zip / .mcpack — lire le manifest depuis le ZIP.
        let data = std::fs::read(path)?;
        let manifest_text = read_manifest_from_zip(&data)?;
        (manifest_text, data)
    };

    let manifest: ResourcePackManifest =
        serde_json::from_str(&manifest_text).map_err(std::io::Error::other)?;

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

/// Zippe récursivement un dossier en mémoire (deflate). Les chemins dans
/// le ZIP sont relatifs à `root`.
fn zip_directory(root: &Path) -> std::io::Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let options: FileOptions = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);
        add_dir_to_zip(&mut writer, root, root, options)?;
        writer.finish().map_err(std::io::Error::other)?;
    }
    Ok(buf.into_inner())
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    writer: &mut zip::ZipWriter<W>,
    base: &Path,
    current: &Path,
    options: FileOptions,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).map_err(std::io::Error::other)?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            add_dir_to_zip(writer, base, &path, options)?;
        } else {
            writer
                .start_file(&rel_str, options)
                .map_err(std::io::Error::other)?;
            let bytes = std::fs::read(&path)?;
            writer.write_all(&bytes)?;
        }
    }
    Ok(())
}

fn read_manifest_from_zip(data: &[u8]) -> std::io::Result<String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).map_err(std::io::Error::other)?;
    let mut file = archive
        .by_name("manifest.json")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "manifest.json missing"))?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

/// Scan le dossier `resource_packs/` et charge tous les packs présents
/// (dossiers ET fichiers .zip/.mcpack).
pub fn discover_packs(root: &Path) -> Vec<ResourcePack> {
    let mut packs = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return packs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Accepte dossier OU fichier zippé .mcpack/.zip
        let is_pack_file = path.extension().is_some_and(|ext| {
            let ext = ext.to_ascii_lowercase();
            ext == "mcpack" || ext == "zip"
        });
        if !path.is_dir() && !is_pack_file {
            continue;
        }
        match load_pack(&path) {
            Ok(p) => {
                tracing::info!(
                    "Loaded resource pack '{}' (uuid={}, size={} bytes) from {:?}",
                    p.manifest.header.name,
                    p.manifest.header.uuid,
                    p.data.len(),
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

pub fn pack_path() -> PathBuf {
    PathBuf::from("resource_packs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(suffix: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("mc-rs-test-pack-{}-{}", suffix, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn loads_and_zips_a_directory_pack() {
        let dir = temp_dir("load");
        let manifest = r#"{"format_version":2,"header":{"uuid":"00000000-0000-0000-0000-000000000001","version":[1,0,0],"name":"Test"}}"#;
        std::fs::write(dir.join("manifest.json"), manifest).unwrap();
        std::fs::create_dir_all(dir.join("textures")).unwrap();
        std::fs::write(dir.join("textures/dummy.txt"), "hello").unwrap();

        let pack = load_pack(&dir).unwrap();
        assert_eq!(pack.uuid(), "00000000-0000-0000-0000-000000000001");
        assert_eq!(pack.version_string(), "1.0.0");
        assert!(pack.size() > 0);
        assert_eq!(pack.sha256_hex().len(), 64);

        // Le ZIP doit contenir manifest.json + textures/dummy.txt
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&pack.data)).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "manifest.json"));
        assert!(names.iter().any(|n| n == "textures/dummy.txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chunk_returns_exact_slice_and_empty_after_end() {
        let dir = temp_dir("chunk");
        std::fs::write(
            dir.join("manifest.json"),
            r#"{"format_version":2,"header":{"uuid":"x","version":[1,0,0],"name":"x"}}"#,
        )
        .unwrap();
        let pack = load_pack(&dir).unwrap();

        let c0 = pack.chunk(0, 16);
        assert!(!c0.is_empty());
        // Far past the end → empty.
        assert!(pack.chunk(9999, 16).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_skips_non_pack_files() {
        let root = temp_dir("disc");
        // Non-pack file
        let mut f = std::fs::File::create(root.join("README.txt")).unwrap();
        writeln!(f, "ignore me").unwrap();
        // Valid pack directory
        let pack_dir = root.join("pack-a");
        std::fs::create_dir_all(&pack_dir).unwrap();
        std::fs::write(
            pack_dir.join("manifest.json"),
            r#"{"format_version":2,"header":{"uuid":"a","version":[1,0,0],"name":"A"}}"#,
        )
        .unwrap();

        let packs = discover_packs(&root);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].uuid(), "a");

        let _ = std::fs::remove_dir_all(&root);
    }
}
