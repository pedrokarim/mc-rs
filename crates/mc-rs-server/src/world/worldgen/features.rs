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

/// Registre : placed + configured features chargés une fois, plus le `TreePlan`
/// précalculé par biome.
struct Registry {
    placed: HashMap<String, Value>,
    configured: HashMap<String, Value>,
    biome_tree: HashMap<String, TreePlan>,
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
    };
    let biome_names = data::list_names("biome");
    let mut plans = HashMap::new();
    for b in &biome_names {
        plans.insert(format!("minecraft:{b}"), reg.compute_tree_plan(b));
    }
    reg.biome_tree = plans;
    reg
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
                let density = density_from_placement(pf.get("placement"));
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

/// Densité (placements/chunk) à partir des modificateurs de placement :
/// `count` (ou 1 placement par défaut) × `1/rarity_filter.chance`.
fn density_from_placement(placement: Option<&Value>) -> f64 {
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
    fn all_overworld_biomes_have_a_plan() {
        // Aucun panic / parse échoué sur l'ensemble des biomes.
        for b in data::list_names("biome") {
            let _ = tree_plan(&format!("minecraft:{b}"));
        }
    }
}
