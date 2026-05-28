//! Phase B — moteur de climat multi-noise 6D.
//!
//! Porté de `Climate.ts` (deepslate), lui-même fidèle au `Climate` vanilla.
//! Espace de paramètres à 7 dimensions : 6 climat (temperature, humidity,
//! continentalness, erosion, depth, weirdness) + `offset`. Chaque biome est
//! défini par un `ParamPoint` (intervalles) ; le placement retient le biome de
//! `fittness` minimale au point cible.
//!
//! La recherche est linéaire : elle donne exactement le même résultat que le
//! `RTree` vanilla (qui n'est qu'une structure d'accélération, pas une
//! approximation).
//!
//! Le `Sampler` mappe les fonctions du router comme vanilla : humidity ←
//! `vegetation`, continentalness ← `continents`, weirdness ← `ridges`. Les
//! valeurs sont échantillonnées aux coordonnées de bloc `(qx<<2, qy<<2, qz<<2)`
//! (le climat est résolu par quart de bloc / cellule 4×4×4).

use std::sync::Arc;

use serde_json::Value;

use super::data;
use super::density::{Df, NoiseRouter};

#[inline]
fn sq(x: f64) -> f64 {
    x * x
}

/// Intervalle de paramètre climatique `[min, max]`.
#[derive(Clone, Copy, Debug)]
pub struct Param {
    pub min: f64,
    pub max: f64,
}

impl Param {
    pub fn point(v: f64) -> Self {
        Param { min: v, max: v }
    }

    pub fn range(min: f64, max: f64) -> Self {
        Param { min, max }
    }

    /// Distance vanilla d'un intervalle à une valeur ponctuelle.
    #[inline]
    fn distance_to(&self, v: f64) -> f64 {
        let diff_max = v - self.max;
        let diff_min = self.min - v;
        if diff_max > 0.0 {
            diff_max
        } else {
            diff_min.max(0.0)
        }
    }
}

/// Point de définition d'un biome dans l'espace climatique 7D.
#[derive(Clone, Copy, Debug)]
pub struct ParamPoint {
    pub temperature: Param,
    pub humidity: Param,
    pub continentalness: Param,
    pub erosion: Param,
    pub depth: Param,
    pub weirdness: Param,
    pub offset: f64,
}

impl ParamPoint {
    /// Somme des carrés des distances par dimension (métrique vanilla).
    fn fittness(&self, t: &TargetPoint) -> f64 {
        sq(self.temperature.distance_to(t.temperature))
            + sq(self.humidity.distance_to(t.humidity))
            + sq(self.continentalness.distance_to(t.continentalness))
            + sq(self.erosion.distance_to(t.erosion))
            + sq(self.depth.distance_to(t.depth))
            + sq(self.weirdness.distance_to(t.weirdness))
            + sq(self.offset - t.offset)
    }
}

/// Point cible échantillonné (valeurs ponctuelles).
#[derive(Clone, Copy, Debug)]
pub struct TargetPoint {
    pub temperature: f64,
    pub humidity: f64,
    pub continentalness: f64,
    pub erosion: f64,
    pub depth: f64,
    pub weirdness: f64,
    pub offset: f64,
}

/// Liste de paramètres biome → identifiant, avec recherche du plus proche.
pub struct BiomeParameters<T: Copy> {
    entries: Vec<(ParamPoint, T)>,
}

impl<T: Copy> BiomeParameters<T> {
    pub fn new(entries: Vec<(ParamPoint, T)>) -> Self {
        assert!(
            !entries.is_empty(),
            "au moins un biome requis dans la param list"
        );
        BiomeParameters { entries }
    }

    /// Biome de `fittness` minimale au point cible.
    pub fn find(&self, t: &TargetPoint) -> T {
        let mut best = self.entries[0].1;
        let mut best_f = f64::MAX;
        for (p, id) in &self.entries {
            let f = p.fittness(t);
            if f < best_f {
                best_f = f;
                best = *id;
            }
        }
        best
    }
}

/// Param list overworld chargé : points de paramètres → index de biome, plus
/// la table des noms de biomes Java (`minecraft:plains`, …).
pub struct OverworldBiomes {
    pub params: BiomeParameters<u16>,
    pub names: Vec<String>,
}

fn read_param(v: &Value) -> Param {
    match v {
        Value::Number(n) => Param::point(n.as_f64().unwrap()),
        Value::Array(a) => Param::range(a[0].as_f64().unwrap(), a[1].as_f64().unwrap()),
        other => panic!("paramètre climat invalide: {other}"),
    }
}

/// Charge le param list multi-noise overworld résolu et vendoré
/// (`data/worldgen/biome_parameters/overworld.json`).
pub fn load_overworld() -> OverworldBiomes {
    let json = data::biome_parameters_json("overworld").expect("param list overworld vendoré");
    let arr: Value = serde_json::from_str(json).expect("param list JSON valide");
    let arr = arr.as_array().expect("param list = tableau");

    let mut names: Vec<String> = Vec::new();
    let mut entries: Vec<(ParamPoint, u16)> = Vec::with_capacity(arr.len());
    for e in arr {
        let name = e["biome"].as_str().expect("biome name");
        let idx = match names.iter().position(|n| n == name) {
            Some(i) => i as u16,
            None => {
                names.push(name.to_string());
                (names.len() - 1) as u16
            }
        };
        let p = &e["parameters"];
        entries.push((
            ParamPoint {
                temperature: read_param(&p["temperature"]),
                humidity: read_param(&p["humidity"]),
                continentalness: read_param(&p["continentalness"]),
                erosion: read_param(&p["erosion"]),
                depth: read_param(&p["depth"]),
                weirdness: read_param(&p["weirdness"]),
                offset: p["offset"].as_f64().unwrap_or(0.0),
            },
            idx,
        ));
    }

    OverworldBiomes {
        params: BiomeParameters::new(entries),
        names,
    }
}

/// Échantillonneur climatique : les 6 fonctions de densité du router.
pub struct ClimateSampler {
    temperature: Arc<Df>,
    humidity: Arc<Df>,
    continentalness: Arc<Df>,
    erosion: Arc<Df>,
    depth: Arc<Df>,
    weirdness: Arc<Df>,
}

impl ClimateSampler {
    pub fn from_router(r: &NoiseRouter) -> Self {
        ClimateSampler {
            temperature: r.temperature.clone(),
            humidity: r.vegetation.clone(),
            continentalness: r.continents.clone(),
            erosion: r.erosion.clone(),
            depth: r.depth.clone(),
            weirdness: r.ridges.clone(),
        }
    }

    /// Échantillonne le climat à une position en quarts de bloc (cellule 4×4×4).
    pub fn sample(&self, qx: i32, qy: i32, qz: i32) -> TargetPoint {
        let (bx, by, bz) = (qx << 2, qy << 2, qz << 2);
        TargetPoint {
            temperature: self.temperature.compute(bx, by, bz),
            humidity: self.humidity.compute(bx, by, bz),
            continentalness: self.continentalness.compute(bx, by, bz),
            erosion: self.erosion.compute(bx, by, bz),
            depth: self.depth.compute(bx, by, bz),
            weirdness: self.weirdness.compute(bx, by, bz),
            offset: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::worldgen::density;

    #[test]
    fn find_picks_nearest() {
        let cold = ParamPoint {
            temperature: Param::point(-0.9),
            humidity: Param::point(0.0),
            continentalness: Param::point(0.0),
            erosion: Param::point(0.0),
            depth: Param::point(0.0),
            weirdness: Param::point(0.0),
            offset: 0.0,
        };
        let warm = ParamPoint {
            temperature: Param::point(0.9),
            ..cold
        };
        let params = BiomeParameters::new(vec![(cold, 1u32), (warm, 2u32)]);

        let hot_target = TargetPoint {
            temperature: 0.8,
            humidity: 0.0,
            continentalness: 0.0,
            erosion: 0.0,
            depth: 0.0,
            weirdness: 0.0,
            offset: 0.0,
        };
        assert_eq!(params.find(&hot_target), 2);

        let cold_target = TargetPoint {
            temperature: -0.7,
            ..hot_target
        };
        assert_eq!(params.find(&cold_target), 1);
    }

    #[test]
    fn loads_overworld_param_list() {
        let ow = load_overworld();
        // 54 biomes distincts, ~7593 points de paramètres.
        assert_eq!(ow.names.len(), 54, "nombre de biomes inattendu");
        // find renvoie un index valide pour une cible donnée.
        let target = TargetPoint {
            temperature: 0.0,
            humidity: 0.0,
            continentalness: 0.3,
            erosion: 0.0,
            depth: 0.0,
            weirdness: 0.0,
            offset: 0.0,
        };
        let idx = ow.params.find(&target);
        assert!((idx as usize) < ow.names.len());
        assert!(ow.names[idx as usize].starts_with("minecraft:"));
    }

    #[test]
    fn sampler_returns_finite_values() {
        let router = density::build_overworld(42);
        let sampler = ClimateSampler::from_router(&router);
        // Quelques cellules réparties.
        for &(qx, qy, qz) in &[(0, 16, 0), (10, 20, -5), (-30, 8, 30)] {
            let t = sampler.sample(qx, qy, qz);
            for v in [
                t.temperature,
                t.humidity,
                t.continentalness,
                t.erosion,
                t.depth,
                t.weirdness,
            ] {
                assert!(v.is_finite(), "valeur climat non finie: {v}");
                assert!((-10.0..=10.0).contains(&v), "valeur climat aberrante: {v}");
            }
        }
    }
}
