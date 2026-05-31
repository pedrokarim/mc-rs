//! Composition **data-driven** des biomes — lit les VRAIES données vanilla
//! vendorées (`data/worldgen/{biome,placed_feature,configured_feature}`) au lieu
//! de coder en dur quel arbre / quelle densité par biome.
//!
//! Chaîne vanilla : `biome.features[step]` → `placed_feature` (modificateurs de
//! placement = densité) → `configured_feature` (`random_selector` = sélection
//! d'espèces, ou `tree` direct). On en dérive un [`TreePlan`] par biome,
//! équivalent data-driven de l'ancien `decoration::tree_plan` hand-codé.
//!
//! Increment B (végétation : arbres d'abord). Les types de features non encore
//! supportés (fallen_tree, azalea_tree…) résolvent en `None` → emplacement
//! consommé mais rien posé (fidèle au tirage, sans rendu approximatif).

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::Value;

use super::super::block_registry::BLOCKS;
use super::super::random::Random;
use super::data;
use super::decoration::Species;

#[inline]
fn strip(s: &str) -> &str {
    s.strip_prefix("minecraft:").unwrap_or(s)
}

/// Plan d'arbres d'un biome : densité (arbres/chunk) + sélecteur (défaut +
/// alternatives à chances exactes), exactement comme le `random_selector` vanilla.
pub(super) struct TreePlan {
    pub(super) density: f64,
    default: Option<Species>,
    entries: Vec<(f64, Option<Species>)>,
}

impl TreePlan {
    const EMPTY: TreePlan = TreePlan {
        density: 0.0,
        default: None,
        entries: Vec::new(),
    };

    /// Tire une espèce selon la sémantique `random_selector` : chaque entrée
    /// testée dans l'ordre (`rng < chance`), sinon le défaut. `None` = pas
    /// d'arbre posé (feature non supportée), mais le tirage est consommé.
    pub(super) fn pick(&self, rng: &mut Random) -> Option<Species> {
        for (chance, sp) in &self.entries {
            if rng.next_float() < *chance {
                return *sp;
            }
        }
        self.default
    }
}

/// Mappe un nom de feature/arbre (configured ou placed, sans namespace) vers une
/// espèce. Ordre = du plus spécifique au plus général.
fn name_to_species(name: &str) -> Option<Species> {
    use Species::*;
    let n = strip(name);
    if n.contains("fallen") || n.contains("azalea") {
        return None; // non supportés (tronc au sol / grottes luxuriantes)
    }
    Some(if n.contains("fancy_oak") {
        FancyOak
    } else if n.contains("dark_oak") || n.contains("pale_oak") {
        DarkOak
    } else if n.contains("super_birch") || n.contains("tall_birch") {
        SuperBirch
    } else if n.contains("mega_spruce") || n.contains("mega_pine") {
        MegaSpruce
    } else if n.contains("mega_jungle") {
        MegaJungle
    } else if n.contains("jungle_bush") || n == "bush" {
        JungleBush
    } else if n.contains("mangrove") {
        Mangrove
    } else if n.contains("acacia") {
        Acacia
    } else if n.contains("cherry") {
        Cherry
    } else if n.contains("birch") {
        Birch
    } else if n.contains("pine") {
        Pine
    } else if n.contains("spruce") {
        Spruce
    } else if n.contains("jungle") {
        JungleTree
    } else if n.contains("oak") {
        Oak
    } else {
        return None;
    })
}

/// Mappe une config `tree` inline par son trunk placer + son bloc de bûche.
fn tree_config_to_species(config: &Value) -> Option<Species> {
    use Species::*;
    let tp = config
        .get("trunk_placer")
        .and_then(|t| t.get("type"))
        .and_then(Value::as_str)
        .map(strip)
        .unwrap_or("");
    let log = config
        .get("trunk_provider")
        .and_then(|p| p.get("state"))
        .and_then(|s| s.get("Name"))
        .and_then(Value::as_str)
        .map(strip)
        .unwrap_or("");
    let foliage = config
        .get("foliage_placer")
        .and_then(|f| f.get("type"))
        .and_then(Value::as_str)
        .map(strip)
        .unwrap_or("");
    Some(match tp {
        "fancy_trunk_placer" => FancyOak,
        "dark_oak_trunk_placer" => DarkOak,
        "mega_jungle_trunk_placer" => MegaJungle,
        "giant_trunk_placer" => {
            if log.contains("spruce") {
                MegaSpruce
            } else {
                MegaJungle
            }
        }
        "cherry_trunk_placer" => Cherry,
        "upwards_branching_trunk_placer" => Mangrove,
        "forking_trunk_placer" => Acacia,
        "straight_trunk_placer" => {
            if log.contains("birch") {
                // Bouleau haut = height_rand_b > 0.
                let rb = config
                    .get("trunk_placer")
                    .and_then(|t| t.get("height_rand_b"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                if rb > 0 {
                    SuperBirch
                } else {
                    Birch
                }
            } else if log.contains("spruce") {
                if foliage.contains("pine") {
                    Pine
                } else {
                    Spruce
                }
            } else if log.contains("jungle") {
                if foliage.contains("bush") {
                    JungleBush
                } else {
                    JungleTree
                }
            } else if foliage.contains("azalea") || log.is_empty() {
                return None;
            } else {
                Oak
            }
        }
        _ => return None,
    })
}

/// Une feature de végétation au sol (herbe/fougère) d'un biome : placement (pour
/// le nombre de patches) + `tries` du `random_patch` (blocs par patch).
struct VegFeat {
    placement: Value,
    tries: f64,
}

/// Registre : placed + configured features chargés une fois, plus le `TreePlan`
/// et la liste des features d'herbe précalculés par biome.
struct Registry {
    placed: HashMap<String, Value>,
    configured: HashMap<String, Value>,
    biome_tree: HashMap<String, TreePlan>,
    biome_grass: HashMap<String, Vec<VegFeat>>,
    biome_flower: HashMap<String, Vec<VegFeat>>,
    /// Palette de fleurs (IDs runtime) par biome.
    biome_flower_blocks: HashMap<String, Vec<u32>>,
}

static REG: LazyLock<Registry> = LazyLock::new(build_registry);

fn build_registry() -> Registry {
    let mut placed = HashMap::new();
    for name in data::list_names("placed_feature") {
        if let Some(v) = data::json_value("placed_feature", &name) {
            placed.insert(name, v);
        }
    }
    let mut configured = HashMap::new();
    for name in data::list_names("configured_feature") {
        if let Some(v) = data::json_value("configured_feature", &name) {
            configured.insert(name, v);
        }
    }
    let mut reg = Registry {
        placed,
        configured,
        biome_tree: HashMap::new(),
        biome_grass: HashMap::new(),
        biome_flower: HashMap::new(),
        biome_flower_blocks: HashMap::new(),
    };
    let biome_names = data::list_names("biome");
    let mut plans = HashMap::new();
    let mut grass = HashMap::new();
    let mut flower = HashMap::new();
    let mut flower_blocks = HashMap::new();
    for b in &biome_names {
        let key = format!("minecraft:{b}");
        plans.insert(key.clone(), reg.compute_tree_plan(b));
        grass.insert(key.clone(), reg.compute_grass(b));
        let (fd, fb) = reg.compute_flowers(b);
        flower.insert(key.clone(), fd);
        flower_blocks.insert(key, fb);
    }
    reg.biome_tree = plans;
    reg.biome_grass = grass;
    reg.biome_flower = flower;
    reg.biome_flower_blocks = flower_blocks;
    reg
}

/// Vrai si une feature place de l'herbe/fougère au sol (pas d'algues).
fn is_grass_feature(name: &str) -> bool {
    let n = strip(name);
    (n.contains("grass") || n.contains("fern")) && !n.contains("seagrass")
}

/// Vrai si une feature place des fleurs.
fn is_flower_feature(name: &str) -> bool {
    let n = strip(name);
    n.contains("flower") || n.contains("wildflower")
}

/// Collecte récursivement tous les `Name` de blocs sous une valeur (provider).
fn collect_names(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(o) => {
            for (k, val) in o {
                if k == "Name" {
                    if let Some(s) = val.as_str() {
                        out.push(strip(s).to_string());
                    }
                } else {
                    collect_names(val, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|e| collect_names(e, out)),
        _ => {}
    }
}

impl Registry {
    /// Type d'une configured feature référencée (string nommée ou objet inline).
    fn configured_type<'a>(&'a self, feat: &'a Value) -> Option<&'a str> {
        match feat {
            Value::String(s) => self
                .configured
                .get(strip(s))
                .and_then(|c| c.get("type"))
                .and_then(Value::as_str)
                .map(strip),
            Value::Object(_) => feat.get("type").and_then(Value::as_str).map(strip),
            _ => None,
        }
    }

    /// Résout une référence de **placed feature** (nom ou inline) vers une espèce.
    /// Une placed feature pointe toujours vers une configured via son champ
    /// `feature` — d'où la séparation placed/configured (les noms peuvent
    /// coïncider entre les deux registres, ce qui bouclerait sinon).
    fn resolve_placed(&self, fref: &Value) -> Option<Species> {
        match fref {
            Value::String(s) => {
                let pf = self.placed.get(strip(s))?;
                self.resolve_configured(pf.get("feature")?)
            }
            Value::Object(o) => self.resolve_configured(o.get("feature")?),
            _ => None,
        }
    }

    /// Résout une référence de **configured feature** (nom ou inline) vers une
    /// espèce.
    fn resolve_configured(&self, fref: &Value) -> Option<Species> {
        match fref {
            Value::String(s) => name_to_species(strip(s)),
            Value::Object(o) => match o.get("type").and_then(Value::as_str).map(strip) {
                Some("tree") => o.get("config").and_then(tree_config_to_species),
                Some("random_selector") => o
                    .get("config")
                    .and_then(|c| c.get("default"))
                    .and_then(|d| self.resolve_placed(d)),
                _ => None,
            },
            _ => None,
        }
    }

    /// `tries` du `random_patch` d'une feature configured référencée (défaut 32).
    fn configured_tries(&self, feat: &Value) -> f64 {
        let cfg = match feat {
            Value::String(s) => self.configured.get(strip(s)).and_then(|c| c.get("config")),
            Value::Object(_) => feat.get("config"),
            _ => None,
        };
        cfg.and_then(|c| c.get("tries"))
            .and_then(Value::as_f64)
            .unwrap_or(32.0)
    }

    /// Parcourt une configured feature et collecte les blocs de ses providers
    /// `to_place` (suit les sous-features nommées). Sert à extraire la palette de
    /// fleurs (les prédicats `would_survive` ne sont pas sous `to_place`, donc
    /// pas collectés).
    fn collect_flower_blocks(&self, cf: &Value, out: &mut Vec<String>, depth: i32) {
        if depth <= 0 {
            return;
        }
        match cf {
            // Référence nommée vers une configured feature → on la charge.
            Value::String(s) => {
                if let Some(c) = self.configured.get(strip(s)) {
                    self.collect_flower_blocks(c, out, depth - 1);
                }
            }
            Value::Object(o) => {
                if let Some(tp) = o.get("to_place") {
                    collect_names(tp, out);
                }
                for (k, v) in o {
                    if k != "to_place" {
                        self.collect_flower_blocks(v, out, depth - 1);
                    }
                }
            }
            Value::Array(a) => a
                .iter()
                .for_each(|e| self.collect_flower_blocks(e, out, depth - 1)),
            _ => {}
        }
    }

    /// Features de fleurs d'un biome : densité (VegFeat) + palette résolue en IDs.
    fn compute_flowers(&self, biome: &str) -> (Vec<VegFeat>, Vec<u32>) {
        let mut feats = Vec::new();
        let mut names: Vec<String> = Vec::new();
        let Some(bio) = data::json_value("biome", biome) else {
            return (feats, Vec::new());
        };
        let Some(steps) = bio.get("features").and_then(Value::as_array) else {
            return (feats, Vec::new());
        };
        for step in steps {
            let Some(ids) = step.as_array() else { continue };
            for id in ids {
                let Some(id) = id.as_str() else { continue };
                if !is_flower_feature(id) {
                    continue;
                }
                let Some(pf) = self.placed.get(strip(id)) else {
                    continue;
                };
                let tries = pf
                    .get("feature")
                    .map(|f| self.configured_tries(f))
                    .unwrap_or(64.0);
                feats.push(VegFeat {
                    placement: pf.get("placement").cloned().unwrap_or(Value::Null),
                    tries,
                });
                if let Some(f) = pf.get("feature") {
                    self.collect_flower_blocks(f, &mut names, 8);
                }
            }
        }
        // Noms → IDs runtime (dédup en gardant les poids = répétitions).
        let blocks: Vec<u32> = names
            .iter()
            .map(|n| BLOCKS.get(&format!("minecraft:{n}")))
            .filter(|&id| id != BLOCKS.air)
            .collect();
        (feats, blocks)
    }

    /// Features d'herbe/fougère d'un biome (toutes étapes).
    fn compute_grass(&self, biome: &str) -> Vec<VegFeat> {
        let mut out = Vec::new();
        let Some(bio) = data::json_value("biome", biome) else {
            return out;
        };
        let Some(steps) = bio.get("features").and_then(Value::as_array) else {
            return out;
        };
        for step in steps {
            let Some(ids) = step.as_array() else { continue };
            for id in ids {
                let Some(id) = id.as_str() else { continue };
                if !is_grass_feature(id) {
                    continue;
                }
                let Some(pf) = self.placed.get(strip(id)) else {
                    continue;
                };
                let tries = pf
                    .get("feature")
                    .map(|f| self.configured_tries(f))
                    .unwrap_or(32.0);
                out.push(VegFeat {
                    placement: pf.get("placement").cloned().unwrap_or(Value::Null),
                    tries,
                });
            }
        }
        out
    }

    fn compute_tree_plan(&self, biome: &str) -> TreePlan {
        let Some(bio) = data::json_value("biome", biome) else {
            return TreePlan::EMPTY;
        };
        let Some(steps) = bio.get("features").and_then(Value::as_array) else {
            return TreePlan::EMPTY;
        };
        for step in steps {
            let Some(ids) = step.as_array() else { continue };
            for id in ids {
                let Some(id) = id.as_str() else { continue };
                let Some(pf) = self.placed.get(strip(id)) else {
                    continue;
                };
                let Some(feat) = pf.get("feature") else {
                    continue;
                };
                match self.configured_type(feat) {
                    Some("random_selector") | Some("tree") => {}
                    _ => continue,
                }
                let density = placement_count(pf.get("placement"), 0.0);
                // random_selector : default + entrées.
                let cfg = match feat {
                    Value::String(s) => self.configured.get(strip(s)).map(|c| c.get("config")),
                    Value::Object(_) => Some(feat.get("config")),
                    _ => None,
                }
                .flatten();
                if self.configured_type(feat) == Some("tree") {
                    return TreePlan {
                        density,
                        default: cfg.and_then(tree_config_to_species),
                        entries: Vec::new(),
                    };
                }
                let Some(cfg) = cfg else { continue };
                // default + entrées du random_selector référencent des PLACED features.
                let default = cfg.get("default").and_then(|d| self.resolve_placed(d));
                let entries = cfg
                    .get("features")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|e| {
                                let chance = e.get("chance").and_then(Value::as_f64)?;
                                Some((chance, self.resolve_placed(e.get("feature")?)))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return TreePlan {
                    density,
                    default,
                    entries,
                };
            }
        }
        TreePlan::EMPTY
    }
}

/// Moyenne d'un `IntProvider` de count (`weighted_list` / `uniform` / `constant`
/// / entier brut).
fn count_mean(v: &Value) -> f64 {
    if let Some(n) = v.as_f64() {
        return n;
    }
    let Some(o) = v.as_object() else { return 1.0 };
    match o.get("type").and_then(Value::as_str).map(strip) {
        Some("weighted_list") => {
            let dist = o.get("distribution").and_then(Value::as_array);
            if let Some(dist) = dist {
                let mut num = 0.0;
                let mut den = 0.0;
                for e in dist {
                    let d = e.get("data").and_then(Value::as_f64).unwrap_or(0.0);
                    let w = e.get("weight").and_then(Value::as_f64).unwrap_or(0.0);
                    num += d * w;
                    den += w;
                }
                if den > 0.0 {
                    return num / den;
                }
            }
            1.0
        }
        Some("uniform") | Some("biased_to_bottom_int") | Some("clamped") => {
            let min = o
                .get("min_inclusive")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let max = o
                .get("max_inclusive")
                .and_then(Value::as_f64)
                .unwrap_or(min);
            (min + max) / 2.0
        }
        Some("constant") => o.get("value").and_then(Value::as_f64).unwrap_or(1.0),
        _ => 1.0,
    }
}

/// Nombre de placements/chunk à partir des modificateurs de placement :
/// `count` (ou `noise_threshold_count` piloté par `noise`, ou 1 par défaut) ×
/// `1/rarity_filter.chance`.
fn placement_count(placement: Option<&Value>, noise: f64) -> f64 {
    let Some(mods) = placement.and_then(Value::as_array) else {
        return 1.0;
    };
    let mut base: Option<f64> = None;
    let mut rarity = 1.0;
    for m in mods {
        match m.get("type").and_then(Value::as_str).map(strip) {
            Some("count") => {
                if let Some(c) = m.get("count") {
                    base = Some(count_mean(c));
                }
            }
            Some("noise_threshold_count") => {
                let level = m.get("noise_level").and_then(Value::as_f64).unwrap_or(0.0);
                let key = if noise > level {
                    "above_noise"
                } else {
                    "below_noise"
                };
                base = Some(m.get(key).and_then(Value::as_f64).unwrap_or(0.0));
            }
            Some("rarity_filter") => {
                if let Some(ch) = m.get("chance").and_then(Value::as_f64) {
                    if ch > 0.0 {
                        rarity = 1.0 / ch;
                    }
                }
            }
            _ => {}
        }
    }
    base.unwrap_or(1.0) * rarity
}

static EMPTY_PLAN: TreePlan = TreePlan {
    density: 0.0,
    default: None,
    entries: Vec::new(),
};

/// Plan d'arbres data-driven d'un biome (`minecraft:plains`, …). Précalculé.
pub(super) fn tree_plan(biome: &str) -> &'static TreePlan {
    REG.biome_tree.get(biome).unwrap_or(&EMPTY_PLAN)
}

/// Nombre d'herbes/fougères à tenter de poser dans un chunk de ce biome, dérivé
/// des vraies features vanilla (`Σ patches × tries`, `noise` pilotant les
/// `noise_threshold_count`). Ex. neige ≈ 32 (1 patch clairsemé) ≪ jungle ≈ 800.
pub(super) fn grass_attempts(biome: &str, noise: f64) -> i32 {
    veg_attempts(REG.biome_grass.get(biome), noise)
}

/// Nombre de fleurs à tenter de poser dans un chunk de ce biome (mêmes données
/// vanilla que l'herbe : `Σ patches × tries`).
pub(super) fn flower_attempts(biome: &str, noise: f64) -> i32 {
    veg_attempts(REG.biome_flower.get(biome), noise)
}

/// Palette de fleurs (IDs runtime) d'un biome — poids = répétitions (ex. cerisaie
/// = pink_petals ×16, flower_forest = 11 espèces). Vide = pas de fleurs.
pub(super) fn flower_blocks(biome: &str) -> &'static [u32] {
    REG.biome_flower_blocks
        .get(biome)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

fn veg_attempts(feats: Option<&Vec<VegFeat>>, noise: f64) -> i32 {
    let Some(feats) = feats else { return 0 };
    feats
        .iter()
        .map(|f| placement_count(Some(&f.placement), noise) * f.tries)
        .sum::<f64>()
        .round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dens(b: &str) -> f64 {
        tree_plan(b).density
    }

    #[test]
    fn densities_match_vanilla() {
        // Moyennes pondérées exactes des count distributions vanilla.
        assert!(
            (dens("minecraft:plains") - 0.05).abs() < 1e-6,
            "{}",
            dens("minecraft:plains")
        );
        assert!((dens("minecraft:snowy_plains") - 0.1).abs() < 1e-6);
        assert!((dens("minecraft:jungle") - 50.1).abs() < 1e-6);
        assert!((dens("minecraft:dark_forest") - 16.0).abs() < 1e-6);
        assert!((dens("minecraft:savanna") - 1.1).abs() < 1e-6);
        // meadow = rarity_filter 1/100.
        assert!(
            (dens("minecraft:meadow") - 0.01).abs() < 1e-6,
            "{}",
            dens("minecraft:meadow")
        );
        // désert : aucun arbre.
        assert_eq!(dens("minecraft:desert"), 0.0);
    }

    #[test]
    fn selectors_resolve_expected_species() {
        // savanna : défaut chêne + 80 % acacia.
        let p = tree_plan("minecraft:savanna");
        assert!(matches!(p.default, Some(Species::Oak)));
        assert!(p
            .entries
            .iter()
            .any(|(c, s)| (*c - 0.8).abs() < 1e-6 && matches!(s, Some(Species::Acacia))));
        // taiga : défaut sapin + 1/3 pin.
        let t = tree_plan("minecraft:taiga");
        assert!(matches!(t.default, Some(Species::Spruce)));
        assert!(t
            .entries
            .iter()
            .any(|(_, s)| matches!(s, Some(Species::Pine))));
        // cerisaie : cerisier.
        assert!(matches!(
            tree_plan("minecraft:cherry_grove").default,
            Some(Species::Cherry)
        ));
        // forêt sombre : le chêne noir est l'entrée dominante (~2/3), pas le défaut.
        assert!(tree_plan("minecraft:dark_forest")
            .entries
            .iter()
            .any(|(c, s)| *c > 0.6 && matches!(s, Some(Species::DarkOak))));
    }

    #[test]
    fn grass_density_matches_vanilla_scale() {
        // neige = 1 patch clairsemé (patch_grass_badlands, tries 32) ≈ 32.
        assert_eq!(grass_attempts("minecraft:snowy_plains", 0.0), 32);
        // forêt = count 2 × 32 = 64.
        assert_eq!(grass_attempts("minecraft:forest", 0.0), 64);
        // jungle = count 25 × 32 = 800 (densément herbeuse).
        assert_eq!(grass_attempts("minecraft:jungle", 0.0), 800);
        // neige ≪ jungle (le fix : plus de sur-végétation neigeuse).
        assert!(
            grass_attempts("minecraft:snowy_plains", 0.0)
                < grass_attempts("minecraft:jungle", 0.0) / 10
        );
    }

    #[test]
    fn flower_palettes_and_density() {
        // flower_forest : palette riche (11 espèces), dense.
        let ff = flower_blocks("minecraft:flower_forest");
        assert!(
            ff.len() >= 8,
            "flower_forest palette trop pauvre: {}",
            ff.len()
        );
        assert!(flower_attempts("minecraft:flower_forest", 0.0) > 0);
        // cerisaie : pink_petals (palette non vide).
        assert!(!flower_blocks("minecraft:cherry_grove").is_empty());
        // marais : blue_orchid.
        let swamp = flower_blocks("minecraft:swamp");
        assert_eq!(swamp.len(), 1, "marais = 1 fleur (blue_orchid)");
        // forêt : poppy + dandelion (flower_default).
        assert_eq!(flower_blocks("minecraft:forest").len(), 2);
        // désert : aucune fleur réelle (flower_default sur sable → palette résolue
        // mais densité faible) ; au moins ça ne panique pas.
        let _ = flower_blocks("minecraft:desert");
    }

    #[test]
    fn all_overworld_biomes_have_a_plan() {
        // Aucun panic / parse échoué sur l'ensemble des biomes.
        for b in data::list_names("biome") {
            let _ = tree_plan(&format!("minecraft:{b}"));
        }
    }
}
