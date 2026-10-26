//! Règles de cassage des blocs : outil compatible, temps de cassage, drops, XP, usure de l'outil.
//!
//! Port de PMMP : `BlockBreakInfo` (`isToolCompatible`, `getBreakTime`), `Block::getDrops` /
//! `getXpDropForTool`, `Tool::getMiningEfficiency`, `Item::onDestroyBlock`, `Durable::applyDamage`.
//! Données par état de la palette dans `data/block_break.json`, produites par
//! `scripts/pmmp/export_block_break_data.php` : drops calculés par PMMP lui-même, dureté et outil requis
//! de BDS (dump d'Allay).
//!
//! Les drops viennent d'abord de `data/block_drops_bds.json` : drops observés sur BDS 1.26.52.3 pour
//! chaque état, avec chaque outil (`scripts/dev/build_bds_drops.py`), tirés selon les fréquences
//! observées pour les blocs aléatoires. PMMP ne sert plus qu'aux états absents de ce relevé. Méthode et
//! écarts : `dev-docs/36-CASSAGE-DE-BLOCS.md`.

use std::collections::HashMap;
use std::sync::LazyLock;

use mc_rs_proto::packets::player::ItemStack;
use rand::Rng;
use serde::Deserialize;

use crate::enchantments::EnchantmentKind;
use crate::item_extra_data::ItemExtraData;

/// Bits de type d'outil (PMMP `BlockToolType`).
pub mod tool_type {
    pub const NONE: u8 = 0;
    pub const SWORD: u8 = 1 << 0;
    pub const SHOVEL: u8 = 1 << 1;
    pub const PICKAXE: u8 = 1 << 2;
    pub const AXE: u8 = 1 << 3;
    pub const SHEARS: u8 = 1 << 4;
    pub const HOE: u8 = 1 << 5;
}

const DATA_JSON: &str = include_str!("../data/block_break.json");

/// PMMP `BlockBreakInfo::COMPATIBLE_TOOL_MULTIPLIER` / `INCOMPATIBLE_TOOL_MULTIPLIER`.
const COMPATIBLE_TOOL_MULTIPLIER: f32 = 1.5;
const INCOMPATIBLE_TOOL_MULTIPLIER: f32 = 5.0;

/// Caractéristiques d'un outil (PMMP `Item::getBlockToolType`, `getBlockToolHarvestLevel`,
/// `getMiningEfficiency` sans enchantement, `Durable::getMaxDurability`).
#[derive(Debug, Clone, Deserialize)]
pub struct ToolInfo {
    pub tool_type: u8,
    pub harvest_level: u8,
    pub efficiency_correct: f32,
    pub efficiency_incorrect: f32,
    pub max_durability: Option<u32>,
}

/// Un item rendu par un bloc cassé.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DropItem {
    pub item: String,
    #[serde(default)]
    pub meta: u32,
    #[serde(default = "one")]
    pub count: u16,
    /// Runtime ID de l'état de bloc pour un item bloc.
    pub block: Option<u32>,
}

fn one() -> u16 {
    1
}

/// Drops d'un scénario : liste fixe calculée par PMMP, ou règle à porter à part.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DropList {
    Fixed(Vec<DropItem>),
    /// `"random"` : drops aléatoires (Fortune, chances) ; `"world"` : drops lus dans la tuile.
    Rule(String),
}

/// Drops par outil utilisé (PMMP `Block::getDrops` avec chaque famille d'outil).
#[derive(Debug, Clone, Deserialize)]
pub struct Drops {
    pub hand: DropList,
    pub pickaxe: DropList,
    pub axe: DropList,
    pub shovel: DropList,
    pub hoe: DropList,
    pub sword: DropList,
    pub shears: DropList,
    pub silk_touch: DropList,
    pub incompatible: DropList,
    pub incompatible_silk_touch: DropList,
}

/// XP d'un bloc (PMMP `getXpDropAmount` : constante, ou `mt_rand(min, max)`).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum XpDrop {
    Fixed(u32),
    Range { min: u32, max: u32 },
}

/// Données de cassage d'un état de bloc.
#[derive(Debug, Clone, Deserialize)]
pub struct BreakProfile {
    /// `"pmmp"` (bloc connu de PMMP) ou `"bds"` (données de BDS seules, sans drops).
    pub source: String,
    /// Classe PMMP du bloc.
    pub class: Option<String>,
    pub hardness: f32,
    /// Résistance aux explosions, à l'échelle de PMMP (5 × celle de BDS).
    pub blast_resistance: f32,
    pub tool_type: u8,
    /// BDS `requiresCorrectToolForDrops` : choisit le multiplicateur de vitesse.
    pub requires_correct_tool: bool,
    pub harvest_level: u8,
    pub drops: Option<Drops>,
    #[serde(default)]
    pub affected_by_silk_touch: bool,
    pub xp: Option<XpDrop>,
}

#[derive(Deserialize)]
struct RawData {
    tools: HashMap<String, ToolInfo>,
    profiles: Vec<BreakProfile>,
    states: Vec<u32>,
}

static DATA: LazyLock<RawData> = LazyLock::new(|| {
    let data: RawData = serde_json::from_str(DATA_JSON).expect("block_break.json valide");
    assert_eq!(
        data.states.len(),
        crate::world::block_registry_data::PALETTE_STATE_COUNT as usize,
        "block_break.json ne correspond pas à la palette : relancer scripts/pmmp/export_block_break_data.php"
    );
    data
});

const BDS_DROPS_JSON: &str = include_str!("../data/block_drops_bds.json");

/// Un résultat observé sur BDS : items rendus par un tirage, et nombre de tirages qui l'ont donné.
#[derive(Debug, Clone, Deserialize)]
pub struct BdsOutcome {
    pub weight: u32,
    pub items: Vec<DropItem>,
}

/// Drops observés pour un état, par clé d'outil (`main`, `iron_pickaxe`, `netherite_pickaxe+fortune3`…).
pub type BdsTable = HashMap<String, Vec<BdsOutcome>>;

#[derive(Deserialize)]
struct BdsDrops {
    tables: Vec<BdsTable>,
    /// Indice de table par runtime ID, -1 sans relevé.
    states: Vec<i32>,
}

static BDS_DROPS: LazyLock<BdsDrops> = LazyLock::new(|| {
    let data: BdsDrops = serde_json::from_str(BDS_DROPS_JSON).expect("block_drops_bds.json valide");
    assert_eq!(
        data.states.len(),
        crate::world::block_registry_data::PALETTE_STATE_COUNT as usize,
        "block_drops_bds.json ne correspond pas à la palette : refaire le relevé (scripts/dev/build_bds_drops.py)"
    );
    data
});

/// Drops observés sur BDS pour un état, s'il a été relevé.
pub fn bds_table(runtime_id: u32) -> Option<&'static BdsTable> {
    let index = *BDS_DROPS.states.get(runtime_id as usize)?;
    BDS_DROPS.tables.get(usize::try_from(index).ok()?)
}

/// Données de cassage d'un état (runtime ID de la palette).
pub fn profile(runtime_id: u32) -> Option<&'static BreakProfile> {
    let index = *DATA.states.get(runtime_id as usize)?;
    DATA.profiles.get(index as usize)
}

/// Item tenu, vu par les règles de cassage.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeldTool {
    pub tool_type: u8,
    pub harvest_level: u8,
    efficiency_correct: f32,
    efficiency_incorrect: f32,
    pub efficiency: u8,
    pub silk_touch: bool,
    pub fortune: u8,
    pub unbreaking: u8,
    /// Masse : relevée à part sur BDS (tag `minecraft:is_mace_item_destructible`), absente des outils de PMMP.
    pub mace: bool,
}

impl HeldTool {
    /// Main nue (PMMP `VanillaItems::AIR()`).
    pub fn hand() -> Self {
        Self {
            efficiency_correct: 1.0,
            efficiency_incorrect: 1.0,
            ..Self::default()
        }
    }

    pub fn from_stack(stack: &ItemStack) -> Self {
        let mut held = Self::hand();
        if stack.is_air() {
            return held;
        }
        let name = crate::item_registry::item_name_by_id(stack.id);
        held.mace = name == Some("minecraft:mace");
        if let Some(info) = name.and_then(|name| DATA.tools.get(name)) {
            held.tool_type = info.tool_type;
            held.harvest_level = info.harvest_level;
            held.efficiency_correct = info.efficiency_correct;
            held.efficiency_incorrect = info.efficiency_incorrect;
        }
        if let Some(extra) = ItemExtraData::parse(&stack.extra_data) {
            held.efficiency = extra.enchantment_level(EnchantmentKind::Efficiency);
            held.silk_touch = extra.enchantment_level(EnchantmentKind::SilkTouch) > 0;
            held.fortune = extra.enchantment_level(EnchantmentKind::Fortune);
            held.unbreaking = extra.enchantment_level(EnchantmentKind::Unbreaking);
        }
        held
    }

    /// PMMP `Item::getMiningEfficiency($isCorrectTool)` : efficacité de base de l'outil plus
    /// Efficacité (niveau² + 1) s'il est adapté ; l'épée multiplie le tout par 1,5 (`Sword`), ce que
    /// porte `efficiency_incorrect`.
    fn mining_efficiency(&self, is_correct_tool: bool) -> f32 {
        if !is_correct_tool {
            return self.efficiency_incorrect;
        }
        let bonus = if self.efficiency > 0 {
            f32::from(self.efficiency).powi(2) + 1.0
        } else {
            0.0
        };
        self.efficiency_correct + bonus * self.efficiency_incorrect
    }
}

/// Résultat de `drops` : liste calculée, ou règle pas encore portée dans mc-rs.
#[derive(Debug, Clone, PartialEq)]
pub enum DropOutcome<'a> {
    Items(&'a [DropItem]),
    Unported,
}

impl BreakProfile {
    /// PMMP `BlockBreakInfo::isBreakable`.
    pub fn is_breakable(&self) -> bool {
        self.hardness >= 0.0
    }

    /// PMMP `BlockBreakInfo::breaksInstantly`.
    pub fn breaks_instantly(&self) -> bool {
        self.hardness == 0.0
    }

    /// PMMP `BlockBreakInfo::isToolCompatible` : le bloc rend ses drops normaux avec cet outil.
    pub fn is_tool_compatible(&self, tool: &HeldTool) -> bool {
        if self.hardness < 0.0 {
            return false;
        }
        self.tool_type == tool_type::NONE
            || self.harvest_level == 0
            || (self.tool_type & tool.tool_type != 0 && tool.harvest_level >= self.harvest_level)
    }

    /// Temps de cassage en secondes (PMMP `BlockBreakInfo::getBreakTime`). Le multiplicateur suit BDS
    /// (Allay `BlockBaseComponentImpl::calculateBreakTime`) : rapide si l'outil est compatible ou si le
    /// bloc n'exige pas d'outil. Il ne diffère de PMMP que sur les améthystes, que PMMP modélise par
    /// « outil requis » pour leurs drops.
    pub fn break_time(&self, tool: &HeldTool) -> f32 {
        let multiplier = if self.is_tool_compatible(tool) || !self.requires_correct_tool {
            COMPATIBLE_TOOL_MULTIPLIER
        } else {
            INCOMPATIBLE_TOOL_MULTIPLIER
        };
        let is_correct_tool = self.tool_type & tool.tool_type != 0;
        self.hardness * multiplier / tool.mining_efficiency(is_correct_tool)
    }

    /// Drops du bloc cassé avec `tool` (PMMP `Block::getDrops`).
    pub fn drops(&self, tool: &HeldTool) -> DropOutcome<'_> {
        let Some(drops) = &self.drops else {
            return DropOutcome::Unported;
        };
        let list = if self.is_tool_compatible(tool) {
            if self.affected_by_silk_touch && tool.silk_touch {
                &drops.silk_touch
            } else {
                match tool.tool_type {
                    tool_type::PICKAXE => &drops.pickaxe,
                    tool_type::AXE => &drops.axe,
                    tool_type::SHOVEL => &drops.shovel,
                    tool_type::HOE => &drops.hoe,
                    tool_type::SWORD => &drops.sword,
                    tool_type::SHEARS => &drops.shears,
                    _ => &drops.hand,
                }
            }
        } else if tool.silk_touch {
            &drops.incompatible_silk_touch
        } else {
            &drops.incompatible
        };
        match list {
            DropList::Fixed(items) => DropOutcome::Items(items),
            DropList::Rule(_) => DropOutcome::Unported,
        }
    }

    /// XP rendue (PMMP `Block::getXpDropForTool`) : aucune avec Toucher de soie ou un outil incompatible.
    pub fn xp_drop(&self, tool: &HeldTool, rng: &mut impl Rng) -> u32 {
        if tool.silk_touch || !self.is_tool_compatible(tool) {
            return 0;
        }
        match self.xp {
            None => 0,
            Some(XpDrop::Fixed(xp)) => xp,
            Some(XpDrop::Range { min, max }) => rng.gen_range(min..=max),
        }
    }
}

/// Usure infligée à l'outil par le cassage (PMMP `Item::onDestroyBlock`) : 1 pour pioche, hache,
/// pelle et houe, 2 pour l'épée, sauf bloc instantané ; 1 pour les cisailles dans tous les cas.
pub fn destroy_damage(profile: &BreakProfile, tool: &HeldTool) -> u32 {
    match tool.tool_type {
        tool_type::SHEARS => 1,
        _ if profile.breaks_instantly() => 0,
        tool_type::SWORD => 2,
        tool_type::PICKAXE | tool_type::AXE | tool_type::SHOVEL | tool_type::HOE => 1,
        _ => 0,
    }
}

/// Points d'usure annulés par Solidité (PMMP `Durable::getUnbreakingDamageReduction`) : chaque point
/// est annulé si un tirage dépasse 1 / (niveau + 1).
pub fn unbreaking_reduction(amount: u32, unbreaking: u8, rng: &mut impl Rng) -> u32 {
    if unbreaking == 0 {
        return 0;
    }
    let chance = 1.0 / (f32::from(unbreaking) + 1.0);
    (0..amount).filter(|_| rng.gen::<f32>() > chance).count() as u32
}

/// Conditions du joueur qui modulent la vitesse de cassage.
#[derive(Debug, Clone, Copy, Default)]
pub struct MinerState {
    pub on_ground: bool,
    pub flying: bool,
    /// Yeux sous l'eau sans Affinité aquatique sur le casque.
    pub underwater_without_aqua_affinity: bool,
    /// Niveaux d'effet (amplificateur + 1), 0 sans l'effet.
    pub haste: u8,
    pub mining_fatigue: u8,
}

/// Progression par tick de jeu, 1 = bloc cassé (PMMP
/// `SurvivalBlockBreakHandler::calculateBreakProgressPerTick`).
pub fn break_progress_per_tick(profile: &BreakProfile, tool: &HeldTool, miner: &MinerState) -> f32 {
    if !profile.is_breakable() {
        return 0.0;
    }
    let mut break_time_ticks = profile.break_time(tool) * 20.0;
    if !miner.on_ground && !miner.flying {
        break_time_ticks *= 5.0;
    }
    if miner.underwater_without_aqua_affinity {
        break_time_ticks *= 5.0;
    }
    if break_time_ticks <= 0.0 {
        return 1.0;
    }
    let mut progress = 1.0 / break_time_ticks;
    if miner.haste > 0 {
        let level = f32::from(miner.haste);
        progress *= (1.0 + 0.2 * level) * 1.2f32.powf(level);
    }
    if miner.mining_fatigue > 0 {
        progress *= 0.21f32.powf(f32::from(miner.mining_fatigue));
    }
    progress
}

/// Outil relevé sur BDS qui représente `tool` (clés de `block_drops_bds.json`).
///
/// Le relevé couvre la main nue, les pioches de chaque rang, une hache, une pelle, une houe et une épée en
/// bois (aucun bloc vanilla n'exige un rang pour ces outils), la masse, les cisailles, et Toucher de soie
/// et Fortune I à III sur une pioche en netherite. L'enchantement ne compte que si l'outil tenu est
/// compatible avec le bloc : il agit alors comme sur la pioche relevée (BDS applique Toucher de soie et
/// Fortune quel que soit l'outil, gravier et pelle compris) ; sinon, le bloc rend ce que rend l'outil sans
/// enchantement.
pub fn bds_tool_key(profile: &BreakProfile, tool: &HeldTool) -> String {
    let compatible = profile.is_tool_compatible(tool);
    if compatible && tool.silk_touch {
        return "netherite_pickaxe+silk_touch1".to_string();
    }
    if compatible && tool.fortune > 0 {
        return format!("netherite_pickaxe+fortune{}", tool.fortune.min(3));
    }
    let key = match tool.tool_type {
        // Rangs de récolte de PMMP : bois 1, or 2, pierre et cuivre 3, fer 4, diamant 5, netherite 6.
        tool_type::PICKAXE => match tool.harvest_level {
            0..=2 => "wooden_pickaxe",
            3 => "stone_pickaxe",
            4 => "iron_pickaxe",
            5 => "diamond_pickaxe",
            _ => "netherite_pickaxe",
        },
        tool_type::AXE => "wooden_axe",
        tool_type::SHOVEL => "wooden_shovel",
        tool_type::HOE => "wooden_hoe",
        tool_type::SWORD => "wooden_sword",
        tool_type::SHEARS => "shears",
        _ if tool.mace => "mace",
        _ => "main",
    };
    key.to_string()
}

/// Tire un résultat selon les fréquences observées sur BDS.
fn pick_outcome<'a>(outcomes: &'a [BdsOutcome], rng: &mut impl Rng) -> Option<&'a BdsOutcome> {
    let total: u32 = outcomes.iter().map(|o| o.weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.gen_range(0..total);
    outcomes.iter().find(|o| {
        if roll < o.weight {
            return true;
        }
        roll -= o.weight;
        false
    })
}

/// Drops d'un bloc cassé avec `tool`, en piles réseau : relevé de BDS d'abord, puis données de PMMP pour
/// les états non relevés. Pour une règle que PMMP ne fixe pas (drops aléatoires, lus dans la tuile, bloc
/// inconnu de PMMP), repli sur l'ancienne table `inventory::block_drop`, soumise à la compatibilité de
/// l'outil.
pub fn drop_stacks(runtime_id: u32, tool: &HeldTool, rng: &mut impl Rng) -> Vec<ItemStack> {
    let Some(profile) = profile(runtime_id) else {
        return Vec::new();
    };
    if let Some(outcomes) = bds_table(runtime_id).and_then(|t| t.get(&bds_tool_key(profile, tool)))
    {
        return pick_outcome(outcomes, rng)
            .map(|o| o.items.iter().filter_map(drop_stack).collect())
            .unwrap_or_default();
    }
    match profile.drops(tool) {
        DropOutcome::Items(items) => items.iter().filter_map(drop_stack).collect(),
        DropOutcome::Unported if profile.is_tool_compatible(tool) => {
            crate::inventory::block_drop(runtime_id)
                .into_iter()
                .collect()
        }
        DropOutcome::Unported => Vec::new(),
    }
}

/// Pile d'items réseau pour un drop ; `None` si l'item est absent du registre.
pub fn drop_stack(drop: &DropItem) -> Option<ItemStack> {
    let id = crate::item_registry::network_id(&drop.item)?;
    let mut stack = ItemStack::new(id, drop.count, drop.block.map_or(0, |b| b as i32));
    stack.meta = drop.meta;
    Some(stack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::block_registry::BLOCKS;
    use rand::SeedableRng;

    fn tool(name: &str) -> HeldTool {
        HeldTool::from_stack(&ItemStack::new(
            crate::item_registry::network_id(name).unwrap(),
            1,
            0,
        ))
    }

    fn first_state(name: &str) -> &'static BreakProfile {
        profile(BLOCKS.get(name)).unwrap()
    }

    fn items(outcome: DropOutcome<'_>) -> Vec<String> {
        match outcome {
            DropOutcome::Items(items) => items.iter().map(|d| d.item.clone()).collect(),
            DropOutcome::Unported => panic!("drops non portés"),
        }
    }

    #[test]
    fn every_palette_state_has_a_profile() {
        for id in 0..crate::world::block_registry_data::PALETTE_STATE_COUNT {
            assert!(profile(id).is_some(), "runtime ID {id} sans profil");
        }
    }

    #[test]
    fn every_fixed_drop_resolves_to_an_item() {
        for p in &DATA.profiles {
            let Some(drops) = &p.drops else { continue };
            for list in [
                &drops.hand,
                &drops.pickaxe,
                &drops.axe,
                &drops.shovel,
                &drops.hoe,
                &drops.sword,
                &drops.shears,
                &drops.silk_touch,
                &drops.incompatible,
                &drops.incompatible_silk_touch,
            ] {
                if let DropList::Fixed(items) = list {
                    for d in items {
                        assert!(drop_stack(d).is_some(), "{} absent du registre", d.item);
                    }
                }
            }
        }
    }

    #[test]
    fn stone_needs_a_pickaxe_and_drops_cobblestone() {
        let stone = first_state("minecraft:stone");
        assert_eq!(stone.hardness, 1.5);
        assert!(!stone.is_tool_compatible(&HeldTool::hand()));
        assert!(items(stone.drops(&HeldTool::hand())).is_empty());
        let pickaxe = tool("minecraft:wooden_pickaxe");
        assert_eq!(items(stone.drops(&pickaxe)), vec!["minecraft:cobblestone"]);
    }

    #[test]
    fn diamond_ore_needs_iron_tier() {
        let ore = first_state("minecraft:diamond_ore");
        assert!(!ore.is_tool_compatible(&tool("minecraft:stone_pickaxe")));
        assert!(ore.is_tool_compatible(&tool("minecraft:iron_pickaxe")));
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        for _ in 0..200 {
            let xp = ore.xp_drop(&tool("minecraft:iron_pickaxe"), &mut rng);
            assert!((3..=7).contains(&xp), "DiamondOre : mt_rand(3, 7)");
        }
        assert_eq!(ore.xp_drop(&tool("minecraft:stone_pickaxe"), &mut rng), 0);
    }

    #[test]
    fn break_times_match_pmmp_formula() {
        // Pierre : 1,5 × 5 / 1 à la main ; 1,5 × 1,5 / 2 à la pioche en bois.
        let stone = first_state("minecraft:stone");
        assert!((stone.break_time(&HeldTool::hand()) - 7.5).abs() < 1e-5);
        assert!((stone.break_time(&tool("minecraft:wooden_pickaxe")) - 1.125).abs() < 1e-5);
        // Toile : épée (10 × 1,5) ; cisailles 15.
        let cobweb = first_state("minecraft:web");
        assert!((cobweb.break_time(&tool("minecraft:iron_sword")) - 4.0 * 1.5 / 15.0).abs() < 1e-5);
        // Efficacité V sur pioche en diamant : 8 + 26.
        let mut pickaxe = tool("minecraft:diamond_pickaxe");
        pickaxe.efficiency = 5;
        assert!((stone.break_time(&pickaxe) - 1.5 * 1.5 / 34.0).abs() < 1e-5);
    }

    #[test]
    fn bds_overrides_apply() {
        // Barreaux de fer : n'importe quelle pioche chez BDS (PMMP : pierre).
        let bars = first_state("minecraft:iron_bars");
        assert!(bars.is_tool_compatible(&tool("minecraft:wooden_pickaxe")));
        // Œuf de dragon : récupérable à la main chez BDS.
        let egg = first_state("minecraft:dragon_egg");
        assert_eq!(
            items(egg.drops(&HeldTool::hand())),
            vec!["minecraft:dragon_egg"]
        );
        // Tailleur de pierre : pioche requise chez BDS.
        let cutter = first_state("minecraft:stonecutter_block");
        assert!(items(cutter.drops(&HeldTool::hand())).is_empty());
    }

    #[test]
    fn bedrock_is_unbreakable() {
        let bedrock = first_state("minecraft:bedrock");
        assert!(!bedrock.is_breakable());
        assert_eq!(
            break_progress_per_tick(bedrock, &HeldTool::hand(), &MinerState::default()),
            0.0
        );
    }

    #[test]
    fn progress_applies_air_water_and_effects() {
        let dirt = first_state("minecraft:dirt");
        let hand = HeldTool::hand();
        let ground = MinerState {
            on_ground: true,
            ..MinerState::default()
        };
        let base = break_progress_per_tick(dirt, &hand, &ground);
        let airborne = break_progress_per_tick(dirt, &hand, &MinerState::default());
        assert!((base / airborne - 5.0).abs() < 1e-4);
        let haste = MinerState { haste: 2, ..ground };
        let expected = base * 1.4 * 1.44;
        assert!((break_progress_per_tick(dirt, &hand, &haste) - expected).abs() < 1e-6);
    }

    #[test]
    fn tool_wear_follows_pmmp() {
        let stone = first_state("minecraft:stone");
        let grass = first_state("minecraft:short_grass");
        assert_eq!(destroy_damage(stone, &tool("minecraft:iron_pickaxe")), 1);
        assert_eq!(destroy_damage(stone, &tool("minecraft:iron_sword")), 2);
        assert_eq!(destroy_damage(grass, &tool("minecraft:iron_sword")), 0);
        assert_eq!(destroy_damage(grass, &tool("minecraft:shears")), 1);
        assert_eq!(destroy_damage(stone, &HeldTool::hand()), 0);
    }

    fn names(stacks: &[ItemStack]) -> Vec<(String, u16)> {
        stacks
            .iter()
            .map(|s| {
                (
                    crate::item_registry::item_name_by_id(s.id)
                        .unwrap()
                        .to_string(),
                    s.count,
                )
            })
            .collect()
    }

    fn bds_drop(name: &str, tool: &HeldTool, seed: u64) -> Vec<(String, u16)> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        names(&drop_stacks(BLOCKS.get(name), tool, &mut rng))
    }

    #[test]
    fn bds_survey_covers_the_palette() {
        let covered = (0..crate::world::block_registry_data::PALETTE_STATE_COUNT)
            .filter(|&id| bds_table(id).is_some())
            .count();
        assert!(covered > 20_000, "{covered} états relevés sur BDS");
    }

    #[test]
    fn every_bds_drop_resolves_to_an_item() {
        for table in &BDS_DROPS.tables {
            for outcomes in table.values() {
                for drop in outcomes.iter().flat_map(|o| &o.items) {
                    assert!(
                        drop_stack(drop).is_some(),
                        "{} absent du registre",
                        drop.item
                    );
                }
            }
        }
    }

    #[test]
    fn bds_drops_follow_the_tool() {
        assert!(bds_drop("minecraft:stone", &HeldTool::hand(), 1).is_empty());
        assert_eq!(
            bds_drop("minecraft:stone", &tool("minecraft:wooden_pickaxe"), 1),
            vec![("minecraft:cobblestone".to_string(), 1)]
        );
        // Minerai de diamant : pioche en fer requise, Toucher de soie inopérant avec un outil trop faible.
        let mut stone_pickaxe = tool("minecraft:stone_pickaxe");
        assert!(bds_drop("minecraft:diamond_ore", &stone_pickaxe, 1).is_empty());
        stone_pickaxe.silk_touch = true;
        assert!(bds_drop("minecraft:diamond_ore", &stone_pickaxe, 1).is_empty());
        let mut iron_pickaxe = tool("minecraft:iron_pickaxe");
        assert_eq!(
            bds_drop("minecraft:diamond_ore", &iron_pickaxe, 1),
            vec![("minecraft:diamond".to_string(), 1)]
        );
        iron_pickaxe.silk_touch = true;
        assert_eq!(
            bds_drop("minecraft:diamond_ore", &iron_pickaxe, 1),
            vec![("minecraft:diamond_ore".to_string(), 1)]
        );
    }

    #[test]
    fn bds_fortune_multiplies_ore_drops() {
        let plain = tool("minecraft:iron_pickaxe");
        let mut fortune = plain;
        fortune.fortune = 3;
        let mut counts = std::collections::BTreeSet::new();
        for seed in 0..200 {
            assert_eq!(
                bds_drop("minecraft:coal_ore", &plain, seed),
                vec![("minecraft:coal".to_string(), 1)]
            );
            // BDS fait apparaître une entité par charbon, chacune d'un seul item.
            let drops = bds_drop("minecraft:coal_ore", &fortune, seed);
            assert!(drops
                .iter()
                .all(|(name, count)| name == "minecraft:coal" && *count == 1));
            counts.insert(drops.len());
        }
        assert_eq!(counts, (1..=4).collect(), "Fortune III : 1 à 4 charbons");
    }

    #[test]
    fn bds_gravel_drops_flint_sometimes() {
        let table = bds_table(BLOCKS.get("minecraft:gravel")).unwrap();
        let outcomes = &table["main"];
        let total: u32 = outcomes.iter().map(|o| o.weight).sum();
        let flint: u32 = outcomes
            .iter()
            .filter(|o| o.items.iter().any(|d| d.item == "minecraft:flint"))
            .map(|o| o.weight)
            .sum();
        let share = f64::from(flint) / f64::from(total);
        assert!((0.04..0.2).contains(&share), "silex : {share} (BDS : 10 %)");
    }

    #[test]
    fn bds_candle_states_drop_their_candle_count() {
        let mut counts = std::collections::BTreeSet::new();
        // Les 8 états de la bougie rouge (candles 0 à 3, allumée ou non) se suivent dans la palette.
        let first = BLOCKS.get("minecraft:red_candle");
        for id in first..first + 8 {
            let outcomes = &bds_table(id).unwrap()["main"];
            assert_eq!(outcomes.len(), 1, "bougies : drops déterministes");
            // Une entité par bougie, comme sur BDS.
            counts.insert(outcomes[0].items.iter().map(|d| d.count).sum::<u16>());
        }
        assert_eq!(counts, (1..=4).collect());
    }

    #[test]
    fn silk_touch_returns_the_block_itself() {
        let ore = first_state("minecraft:coal_ore");
        let mut pickaxe = tool("minecraft:iron_pickaxe");
        pickaxe.silk_touch = true;
        assert_eq!(items(ore.drops(&pickaxe)), vec!["minecraft:coal_ore"]);
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        assert_eq!(ore.xp_drop(&pickaxe, &mut rng), 0);
    }
}
