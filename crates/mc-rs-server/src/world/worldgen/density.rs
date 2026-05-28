//! Interpréteur de density functions vanilla 1.18+.
//!
//! Parse l'arbre JSON (`noise_router.final_density` + `density_function/*`),
//! instancie les bruits seedés comme `RandomState` vanilla (deriver positionnel
//! + `fromHashOf`), et l'évalue en chaque point. Densité > 0 = solide.
//!
//! Le blending inter-chunks est simplifié (monde neuf) : `blend_alpha`→1,
//! `blend_offset`→0, `blend_density`→identité. Les caches/`interpolated` sont
//! transparents ici ; l'interpolation par cellules est gérée à l'échantillonnage
//! (phase A4).

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::blended_noise::BlendedNoise;
use super::data;
use super::perlin::NormalNoise;
use super::rng::{PositionalRandomFactory, XoroshiroRandom};
use super::spline::{Spline, SplinePoint, SplineValue};

#[derive(Clone, Copy)]
pub enum UnaryOp {
    Abs,
    Square,
    Cube,
    HalfNegative,
    QuarterNegative,
    Squeeze,
}

#[derive(Clone, Copy)]
pub enum Rarity {
    Type1,
    Type2,
}

impl Rarity {
    #[inline]
    fn apply(self, v: f64) -> f64 {
        match self {
            Rarity::Type1 => {
                if v < -0.5 {
                    0.75
                } else if v < 0.0 {
                    1.0
                } else if v < 0.5 {
                    1.5
                } else {
                    2.0
                }
            }
            Rarity::Type2 => {
                if v < -0.75 {
                    0.5
                } else if v < -0.5 {
                    0.75
                } else if v < 0.5 {
                    1.0
                } else if v < 0.75 {
                    2.0
                } else {
                    3.0
                }
            }
        }
    }
}

/// Arbre de density function évaluable.
pub enum Df {
    Const(f64),
    Ref(Arc<Df>),
    Noise {
        noise: Arc<NormalNoise>,
        xz_scale: f64,
        y_scale: f64,
    },
    ShiftedNoise {
        noise: Arc<NormalNoise>,
        shift_x: Arc<Df>,
        shift_y: Arc<Df>,
        shift_z: Arc<Df>,
        xz_scale: f64,
        y_scale: f64,
    },
    ShiftA(Arc<NormalNoise>),
    ShiftB(Arc<NormalNoise>),
    WeirdScaledSampler {
        input: Arc<Df>,
        noise: Arc<NormalNoise>,
        rarity: Rarity,
    },
    Blended(Arc<BlendedNoise>),
    YClampedGradient {
        from_y: f64,
        to_y: f64,
        from_value: f64,
        to_value: f64,
    },
    Add(Arc<Df>, Arc<Df>),
    Mul(Arc<Df>, Arc<Df>),
    Min(Arc<Df>, Arc<Df>),
    Max(Arc<Df>, Arc<Df>),
    Unary(UnaryOp, Arc<Df>),
    Clamp {
        input: Arc<Df>,
        min: f64,
        max: f64,
    },
    RangeChoice {
        input: Arc<Df>,
        min_inclusive: f64,
        max_exclusive: f64,
        when_in_range: Arc<Df>,
        when_out_of_range: Arc<Df>,
    },
    Spline(Arc<Spline>),
    /// Cache/interpolated : transparent (l'argument est évalué directement).
    Marker(Arc<Df>),
}

impl Df {
    pub fn compute(&self, x: i32, y: i32, z: i32) -> f64 {
        match self {
            Df::Const(c) => *c,
            Df::Ref(d) | Df::Marker(d) => d.compute(x, y, z),
            Df::Noise {
                noise,
                xz_scale,
                y_scale,
            } => noise.get_value(x as f64 * xz_scale, y as f64 * y_scale, z as f64 * xz_scale),
            Df::ShiftedNoise {
                noise,
                shift_x,
                shift_y,
                shift_z,
                xz_scale,
                y_scale,
            } => {
                let xx = x as f64 * xz_scale + shift_x.compute(x, y, z);
                let yy = y as f64 * y_scale + shift_y.compute(x, y, z);
                let zz = z as f64 * xz_scale + shift_z.compute(x, y, z);
                noise.get_value(xx, yy, zz)
            }
            Df::ShiftA(noise) => noise.get_value(x as f64 * 0.25, 0.0, z as f64 * 0.25) * 4.0,
            Df::ShiftB(noise) => noise.get_value(z as f64 * 0.25, x as f64 * 0.25, 0.0) * 4.0,
            Df::WeirdScaledSampler {
                input,
                noise,
                rarity,
            } => {
                let r = rarity.apply(input.compute(x, y, z));
                r * noise
                    .get_value(x as f64 / r, y as f64 / r, z as f64 / r)
                    .abs()
            }
            Df::Blended(b) => b.compute(x, y, z),
            Df::YClampedGradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => {
                let t = (y as f64 - from_y) / (to_y - from_y);
                clamped_lerp(*from_value, *to_value, t)
            }
            Df::Add(a, b) => a.compute(x, y, z) + b.compute(x, y, z),
            Df::Mul(a, b) => {
                let av = a.compute(x, y, z);
                // Court-circuit vanilla : si le premier facteur est nul, le
                // produit l'est (évite d'évaluer un bruit coûteux).
                if av == 0.0 {
                    0.0
                } else {
                    av * b.compute(x, y, z)
                }
            }
            Df::Min(a, b) => {
                let av = a.compute(x, y, z);
                // vanilla n'évalue b que si nécessaire (b.minValue()<av).
                av.min(b.compute(x, y, z))
            }
            Df::Max(a, b) => {
                let av = a.compute(x, y, z);
                av.max(b.compute(x, y, z))
            }
            Df::Unary(op, a) => {
                let v = a.compute(x, y, z);
                match op {
                    UnaryOp::Abs => v.abs(),
                    UnaryOp::Square => v * v,
                    UnaryOp::Cube => v * v * v,
                    UnaryOp::HalfNegative => {
                        if v > 0.0 {
                            v
                        } else {
                            v * 0.5
                        }
                    }
                    UnaryOp::QuarterNegative => {
                        if v > 0.0 {
                            v
                        } else {
                            v * 0.25
                        }
                    }
                    UnaryOp::Squeeze => {
                        let c = v.clamp(-1.0, 1.0);
                        c / 2.0 - c * c * c / 24.0
                    }
                }
            }
            Df::Clamp { input, min, max } => input.compute(x, y, z).clamp(*min, *max),
            Df::RangeChoice {
                input,
                min_inclusive,
                max_exclusive,
                when_in_range,
                when_out_of_range,
            } => {
                let v = input.compute(x, y, z);
                if v >= *min_inclusive && v < *max_exclusive {
                    when_in_range.compute(x, y, z)
                } else {
                    when_out_of_range.compute(x, y, z)
                }
            }
            Df::Spline(s) => s.apply(x, y, z),
        }
    }
}

#[inline]
fn clamped_lerp(a: f64, b: f64, t: f64) -> f64 {
    if t < 0.0 {
        a
    } else if t > 1.0 {
        b
    } else {
        a + t * (b - a)
    }
}

/// Construit les density functions en instanciant les bruits seedés.
struct Builder {
    deriver: PositionalRandomFactory,
    noise_cache: HashMap<String, Arc<NormalNoise>>,
    df_cache: HashMap<String, Arc<Df>>,
    blended: Option<Arc<BlendedNoise>>,
}

impl Builder {
    fn noise(&mut self, id: &str) -> Arc<NormalNoise> {
        if let Some(n) = self.noise_cache.get(id) {
            return n.clone();
        }
        let params = data::noise_params(id).unwrap_or_else(|| panic!("bruit manquant: {id}"));
        let mut rng = self.deriver.from_hash_of(id);
        let n = Arc::new(NormalNoise::create(&mut rng, &params));
        self.noise_cache.insert(id.to_string(), n.clone());
        n
    }

    /// Résout une référence par id (avec partage de sous-arbre).
    fn reference(&mut self, id: &str) -> Arc<Df> {
        if let Some(d) = self.df_cache.get(id) {
            return d.clone();
        }
        let json = data::density_function_json(id)
            .unwrap_or_else(|| panic!("density function manquante: {id}"));
        let v: Value = serde_json::from_str(json).expect("DF JSON valide");
        let df = Arc::new(self.parse(&v));
        self.df_cache.insert(id.to_string(), df.clone());
        df
    }

    fn arg(&mut self, map: &serde_json::Map<String, Value>, key: &str) -> Arc<Df> {
        Arc::new(self.parse(&map[key]))
    }

    fn parse(&mut self, v: &Value) -> Df {
        match v {
            Value::Number(n) => Df::Const(n.as_f64().unwrap()),
            Value::String(s) => Df::Ref(self.reference(s)),
            Value::Object(map) => {
                let t = map
                    .get("type")
                    .and_then(Value::as_str)
                    .expect("noeud DF sans type");
                self.parse_typed(t, map)
            }
            other => panic!("noeud DF invalide: {other}"),
        }
    }

    fn parse_typed(&mut self, t: &str, map: &serde_json::Map<String, Value>) -> Df {
        let f = |k: &str| map[k].as_f64().unwrap();
        match t {
            "minecraft:constant" => Df::Const(f("argument")),
            "minecraft:blend_alpha" => Df::Const(1.0),
            "minecraft:blend_offset" => Df::Const(0.0),
            "minecraft:blend_density" => Df::Ref(self.arg(map, "argument")),
            "minecraft:flat_cache"
            | "minecraft:cache_2d"
            | "minecraft:cache_once"
            | "minecraft:cache_all_in_cell"
            | "minecraft:interpolated" => Df::Marker(self.arg(map, "argument")),
            "minecraft:noise" => {
                let noise = self.noise(map["noise"].as_str().unwrap());
                Df::Noise {
                    noise,
                    xz_scale: f("xz_scale"),
                    y_scale: f("y_scale"),
                }
            }
            "minecraft:shifted_noise" => {
                let noise = self.noise(map["noise"].as_str().unwrap());
                Df::ShiftedNoise {
                    noise,
                    shift_x: self.arg(map, "shift_x"),
                    shift_y: self.arg(map, "shift_y"),
                    shift_z: self.arg(map, "shift_z"),
                    xz_scale: f("xz_scale"),
                    y_scale: f("y_scale"),
                }
            }
            "minecraft:shift_a" => Df::ShiftA(self.noise(map["argument"].as_str().unwrap())),
            "minecraft:shift_b" => Df::ShiftB(self.noise(map["argument"].as_str().unwrap())),
            "minecraft:shift" => {
                // shift = shift_a + shift_b combinés ; non utilisé en overworld
                // (présent pour complétude). On le traite comme shift_a.
                Df::ShiftA(self.noise(map["argument"].as_str().unwrap()))
            }
            "minecraft:weird_scaled_sampler" => {
                let noise = self.noise(map["noise"].as_str().unwrap());
                let rarity = match map["rarity_value_mapper"].as_str().unwrap() {
                    "type_1" => Rarity::Type1,
                    "type_2" => Rarity::Type2,
                    other => panic!("rarity inconnu: {other}"),
                };
                Df::WeirdScaledSampler {
                    input: self.arg(map, "input"),
                    noise,
                    rarity,
                }
            }
            "minecraft:old_blended_noise" => {
                if self.blended.is_none() {
                    let mut rng = self.deriver.from_hash_of("minecraft:terrain");
                    let b = Arc::new(BlendedNoise::new(
                        &mut rng,
                        f("xz_scale"),
                        f("y_scale"),
                        f("xz_factor"),
                        f("y_factor"),
                        f("smear_scale_multiplier"),
                    ));
                    self.blended = Some(b);
                }
                Df::Blended(self.blended.as_ref().unwrap().clone())
            }
            "minecraft:y_clamped_gradient" => Df::YClampedGradient {
                from_y: f("from_y"),
                to_y: f("to_y"),
                from_value: f("from_value"),
                to_value: f("to_value"),
            },
            "minecraft:add" => Df::Add(self.arg(map, "argument1"), self.arg(map, "argument2")),
            "minecraft:mul" => Df::Mul(self.arg(map, "argument1"), self.arg(map, "argument2")),
            "minecraft:min" => Df::Min(self.arg(map, "argument1"), self.arg(map, "argument2")),
            "minecraft:max" => Df::Max(self.arg(map, "argument1"), self.arg(map, "argument2")),
            "minecraft:abs" => Df::Unary(UnaryOp::Abs, self.arg(map, "argument")),
            "minecraft:square" => Df::Unary(UnaryOp::Square, self.arg(map, "argument")),
            "minecraft:cube" => Df::Unary(UnaryOp::Cube, self.arg(map, "argument")),
            "minecraft:half_negative" => {
                Df::Unary(UnaryOp::HalfNegative, self.arg(map, "argument"))
            }
            "minecraft:quarter_negative" => {
                Df::Unary(UnaryOp::QuarterNegative, self.arg(map, "argument"))
            }
            "minecraft:squeeze" => Df::Unary(UnaryOp::Squeeze, self.arg(map, "argument")),
            "minecraft:clamp" => Df::Clamp {
                input: self.arg(map, "input"),
                min: f("min"),
                max: f("max"),
            },
            "minecraft:range_choice" => Df::RangeChoice {
                input: self.arg(map, "input"),
                min_inclusive: f("min_inclusive"),
                max_exclusive: f("max_exclusive"),
                when_in_range: self.arg(map, "when_in_range"),
                when_out_of_range: self.arg(map, "when_out_of_range"),
            },
            "minecraft:spline" => match &map["spline"] {
                Value::Number(n) => Df::Const(n.as_f64().unwrap()),
                obj => Df::Spline(Arc::new(self.parse_spline(obj))),
            },
            "minecraft:end_islands" => Df::Const(0.0), // overworld n'en a pas besoin
            other => panic!("type de density function non géré: {other}"),
        }
    }

    fn parse_spline(&mut self, v: &Value) -> Spline {
        let map = v.as_object().expect("spline objet");
        let coordinate = Arc::new(self.parse(&map["coordinate"]));
        let points = map["points"]
            .as_array()
            .expect("points spline")
            .iter()
            .map(|p| {
                let pm = p.as_object().unwrap();
                SplinePoint {
                    location: pm["location"].as_f64().unwrap(),
                    derivative: pm["derivative"].as_f64().unwrap(),
                    value: self.parse_spline_value(&pm["value"]),
                }
            })
            .collect();
        Spline { coordinate, points }
    }

    fn parse_spline_value(&mut self, v: &Value) -> SplineValue {
        match v {
            Value::Number(n) => SplineValue::Const(n.as_f64().unwrap()),
            obj => SplineValue::Spline(Box::new(self.parse_spline(obj))),
        }
    }
}

/// Router de bruit overworld : terrain + les 6 fonctions climat utilisées par
/// le placement des biomes multi-noise (Phase B).
pub struct NoiseRouter {
    pub final_density: Arc<Df>,
    pub temperature: Arc<Df>,
    pub vegetation: Arc<Df>,
    pub continents: Arc<Df>,
    pub erosion: Arc<Df>,
    pub depth: Arc<Df>,
    pub ridges: Arc<Df>,
}

/// Construit le router overworld pour une seed donnée (équiv. `RandomState`).
pub fn build_overworld(seed: u64) -> NoiseRouter {
    let settings: Value = serde_json::from_str(
        data::noise_settings_json("minecraft:overworld").expect("overworld settings"),
    )
    .expect("settings JSON valide");
    let router = &settings["noise_router"];

    let deriver = XoroshiroRandom::from_seed(seed).fork_positional();
    let mut builder = Builder {
        deriver,
        noise_cache: HashMap::new(),
        df_cache: HashMap::new(),
        blended: None,
    };

    let mut stage = |key: &str| Arc::new(builder.parse(&router[key]));
    NoiseRouter {
        final_density: stage("final_density"),
        temperature: stage("temperature"),
        vegetation: stage("vegetation"),
        continents: stage("continents"),
        erosion: stage("erosion"),
        depth: stage("depth"),
        ridges: stage("ridges"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_overworld_router() {
        // Ne doit pas paniquer : tout l'arbre final_density parse et instancie.
        let _ = build_overworld(42);
    }

    #[test]
    fn density_solid_below_air_above() {
        let router = build_overworld(42);
        let fd = &router.final_density;
        // Très profond → solide (densité > 0).
        let deep = fd.compute(0, -60, 0);
        // Très haut → air (densité < 0).
        let high = fd.compute(0, 300, 0);
        assert!(
            deep > 0.0,
            "le sous-sol profond devrait être solide: {deep}"
        );
        assert!(high < 0.0, "le ciel devrait être de l'air: {high}");
    }

    #[test]
    fn surface_crossing_in_reasonable_range() {
        let router = build_overworld(42);
        let fd = &router.final_density;
        // Cherche le plus haut Y solide sur la colonne (0,0).
        let mut surface = None;
        for y in (-64..=320).rev() {
            if fd.compute(0, y, 0) > 0.0 {
                surface = Some(y);
                break;
            }
        }
        let s = surface.expect("une surface solide doit exister");
        assert!(
            (-64..=200).contains(&s),
            "surface hors plage plausible: y={s}"
        );
    }

    #[test]
    fn deterministic_same_seed() {
        let a = build_overworld(7);
        let b = build_overworld(7);
        for &(x, y, z) in &[(0, 0, 0), (13, 70, -4), (100, 120, 100)] {
            assert_eq!(
                a.final_density.compute(x, y, z),
                b.final_density.compute(x, y, z)
            );
        }
    }
}
