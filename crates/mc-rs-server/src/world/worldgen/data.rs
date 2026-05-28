//! Accès aux données worldgen vanilla embarquées (`data/worldgen/`).
//!
//! Les identifiants sont des resource locations (`minecraft:cave_cheese`,
//! `minecraft:overworld/continents`). Le préfixe `minecraft:` est optionnel.

use include_dir::{include_dir, Dir};

use super::perlin::NoiseParameters;

static WORLDGEN: Dir = include_dir!("$CARGO_MANIFEST_DIR/data/worldgen");

#[inline]
fn strip(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

/// Paramètres d'un bruit nommé (`data/worldgen/noise/<id>.json`).
pub fn noise_params(id: &str) -> Option<NoiseParameters> {
    let path = format!("noise/{}.json", strip(id));
    let file = WORLDGEN.get_file(&path)?;
    serde_json::from_slice(file.contents()).ok()
}

/// JSON brut d'une density function (`data/worldgen/density_function/<id>.json`).
pub fn density_function_json(id: &str) -> Option<&'static str> {
    let path = format!("density_function/{}.json", strip(id));
    WORLDGEN.get_file(&path)?.contents_utf8()
}

/// JSON brut d'un noise settings (`data/worldgen/noise_settings/<id>.json`).
pub fn noise_settings_json(id: &str) -> Option<&'static str> {
    let path = format!("noise_settings/{}.json", strip(id));
    WORLDGEN.get_file(&path)?.contents_utf8()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_noise_params_parse() {
        let dir = WORLDGEN.get_dir("noise").expect("noise dir embarqué");
        let mut count = 0;
        for file in dir.files() {
            let name = file.path().file_stem().unwrap().to_str().unwrap();
            assert!(
                noise_params(name).is_some(),
                "échec de parsing du bruit '{name}'"
            );
            count += 1;
        }
        assert!(count >= 50, "trop peu de bruits embarqués: {count}");
    }

    #[test]
    fn overworld_density_functions_present() {
        for id in [
            "minecraft:overworld/continents",
            "minecraft:overworld/erosion",
            "minecraft:overworld/depth",
            "minecraft:overworld/sloped_cheese",
            "minecraft:shift_x",
            "minecraft:shift_z",
        ] {
            assert!(density_function_json(id).is_some(), "DF manquante: {id}");
        }
    }

    #[test]
    fn overworld_noise_settings_present() {
        let json = noise_settings_json("minecraft:overworld").expect("overworld settings");
        assert!(json.contains("noise_router"));
        assert!(json.contains("final_density"));
    }

    #[test]
    fn known_noise_params_values() {
        let p = noise_params("minecraft:cave_cheese").expect("cave_cheese");
        assert_eq!(p.first_octave, -8);
        assert_eq!(p.amplitudes.len(), 9);
    }
}
