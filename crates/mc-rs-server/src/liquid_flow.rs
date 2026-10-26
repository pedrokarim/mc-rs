//! Écoulement des liquides (eau, lave) : port de PMMP `block/Liquid.php`, `Water.php`, `Lava.php` et
//! `block/utils/MinimumCostFlowCalculator.php`, avec les mises à jour programmées de `World`
//! (`scheduleDelayedBlockUpdate`, mise à jour des voisins après chaque changement de bloc).
//!
//! Ce que PMMP code bloc par bloc vient ici des données de BDS (`block_liquid`) : un liquide coule dans un bloc
//! dont la réaction (`liquidReactionOnTouch`) n'est pas `BLOCKING` (le bloc est alors cassé, avec ses drops s'il
//! est `POPPED`), et une nouvelle source d'eau se forme au-dessus d'un bloc `isSolid`.
//!
//! États Bedrock : `water`/`flowing_water`/`lava`/`flowing_lava`, 16 états chacun rangés par `liquid_depth`
//! (déclin sur 3 bits, bit 3 = chute). Un liquide modifié par l'écoulement prend la forme `flowing_*`.
//!
//! Écarts connus avec BDS, à observer : l'eau des blocs gorgés d'eau (seconde couche) ne coule pas, le courant
//! ne pousse pas les entités, pas de son de grésillement quand la lave durcit.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::LazyLock;

use crate::block_liquid::{self, LiquidReaction};
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;

/// Déclin maximal (PMMP `Liquid::MAX_DECAY`).
const MAX_DECAY: u8 = 7;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Water,
    Lava,
}

/// État d'un bloc de liquide.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Liquid {
    pub kind: Kind,
    pub still: bool,
    pub decay: u8,
    pub falling: bool,
}

struct Ids {
    water: u32,
    flowing_water: u32,
    lava: u32,
    flowing_lava: u32,
    obsidian: u32,
    cobblestone: u32,
    basalt: u32,
    soul_soil: u32,
    blue_ice: u32,
}

static IDS: LazyLock<Ids> = LazyLock::new(|| Ids {
    water: BLOCKS.get("minecraft:water"),
    flowing_water: BLOCKS.get("minecraft:flowing_water"),
    lava: BLOCKS.get("minecraft:lava"),
    flowing_lava: BLOCKS.get("minecraft:flowing_lava"),
    obsidian: BLOCKS.get("minecraft:obsidian"),
    cobblestone: BLOCKS.get("minecraft:cobblestone"),
    basalt: BLOCKS.get("minecraft:basalt"),
    soul_soil: BLOCKS.get("minecraft:soul_soil"),
    blue_ice: BLOCKS.get("minecraft:blue_ice"),
});

impl Liquid {
    /// Liquide d'un runtime ID, `None` si ce n'est pas un liquide.
    pub fn from_id(id: u32) -> Option<Self> {
        let ids = &*IDS;
        [
            (ids.water, Kind::Water, true),
            (ids.flowing_water, Kind::Water, false),
            (ids.lava, Kind::Lava, true),
            (ids.flowing_lava, Kind::Lava, false),
        ]
        .into_iter()
        .find(|&(first, _, _)| (first..first + 16).contains(&id))
        .map(|(first, kind, still)| {
            let depth = id - first;
            Liquid {
                kind,
                still,
                decay: (depth & 7) as u8,
                falling: depth & 8 != 0,
            }
        })
    }

    pub fn id(self) -> u32 {
        let ids = &*IDS;
        let first = match (self.kind, self.still) {
            (Kind::Water, true) => ids.water,
            (Kind::Water, false) => ids.flowing_water,
            (Kind::Lava, true) => ids.lava,
            (Kind::Lava, false) => ids.flowing_lava,
        };
        first + self.decay as u32 + if self.falling { 8 } else { 0 }
    }

    /// PMMP `isSource`.
    pub fn is_source(self) -> bool {
        !self.falling && self.decay == 0
    }

    /// Forme qui coule, avec ce déclin et cette chute (PMMP : déclin 0 quand le liquide tombe).
    fn flowing(kind: Kind, decay: u8, falling: bool) -> Self {
        Liquid {
            kind,
            still: false,
            decay: if falling { 0 } else { decay },
            falling,
        }
    }
}

/// Faces horizontales dans l'ordre de PMMP `Facing::HORIZONTAL` (nord, sud, ouest, est).
const HORIZONTAL: [[i32; 3]; 4] = [[0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

/// Face opposée (indice dans `HORIZONTAL`).
fn opposite(face: usize) -> usize {
    face ^ 1
}

fn add(p: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [p[0] + d[0], p[1] + d[1], p[2] + d[2]]
}

/// Ce qui a changé pendant un tick, à envoyer aux joueurs de la dimension.
#[derive(Default)]
pub struct FlowOutput {
    /// Positions dont le bloc (première couche) a changé.
    pub changed: Vec<[i32; 3]>,
    /// Positions dont l'eau de la seconde couche a disparu (bloc gorgé d'eau emporté).
    pub liquid_cleared: Vec<[i32; 3]>,
    /// Blocs cassés par un liquide qui lâchent leurs drops (`POPPED`) : position et bloc.
    pub drops: Vec<([i32; 3], u32)>,
}

/// Moteur d'écoulement d'une dimension : mises à jour programmées (PMMP `scheduledBlockUpdateQueue`).
pub struct LiquidFlow {
    nether: bool,
    tick: u64,
    queue: BinaryHeap<Reverse<(u64, [i32; 3])>>,
    due: HashMap<[i32; 3], u64>,
}

impl LiquidFlow {
    pub fn new(nether: bool) -> Self {
        LiquidFlow {
            nether,
            tick: 0,
            queue: BinaryHeap::new(),
            due: HashMap::new(),
        }
    }

    /// Nombre de mises à jour en attente.
    pub fn pending(&self) -> usize {
        self.due.len()
    }

    /// PMMP `tickRate` : eau 5, lave 30 (10 dans le Nether).
    fn tick_rate(&self, kind: Kind) -> u64 {
        match kind {
            Kind::Water => 5,
            Kind::Lava if self.nether => 10,
            Kind::Lava => 30,
        }
    }

    /// PMMP `getFlowDecayPerBlock` : eau 1, lave 2 (1 dans le Nether).
    fn decay_per_block(&self, kind: Kind) -> u8 {
        match kind {
            Kind::Lava if !self.nether => 2,
            _ => 1,
        }
    }

    /// PMMP `World::scheduleDelayedBlockUpdate` : garde l'échéance la plus proche.
    fn schedule(&mut self, pos: [i32; 3], delay: u64) {
        let at = self.tick + delay;
        if self.due.get(&pos).is_some_and(|&t| t <= at) {
            return;
        }
        self.due.insert(pos, at);
        self.queue.push(Reverse((at, pos)));
    }

    /// Un tick de jeu (PMMP `World::tick`) : mises à jour programmées échues, puis mise à jour des voisins de
    /// chaque bloc modifié (par un joueur, une commande, une explosion ou l'écoulement lui-même).
    pub fn tick(&mut self, cache: &mut ChunkCache) -> FlowOutput {
        self.tick += 1;
        let mut out = FlowOutput::default();
        while let Some(&Reverse((at, pos))) = self.queue.peek() {
            if at > self.tick {
                break;
            }
            self.queue.pop();
            if self.due.get(&pos) != Some(&at) {
                continue;
            }
            self.due.remove(&pos);
            self.scheduled_update(cache, pos, &mut out);
        }
        // PMMP `tryAddToNeighbourUpdateQueue` : le bloc modifié et ses six voisins.
        let mut seen = std::collections::HashSet::new();
        for changed in cache.take_changes() {
            for d in [
                [0, 0, 0],
                [0, -1, 0],
                [0, 1, 0],
                [0, 0, -1],
                [0, 0, 1],
                [-1, 0, 0],
                [1, 0, 0],
            ] {
                let pos = add(changed, d);
                if seen.insert(pos) {
                    self.nearby_block_change(cache, pos, &mut out);
                }
            }
        }
        out
    }

    /// Bloc à une position, `None` hors du monde ou dans un chunk non chargé (l'eau n'y entre pas).
    fn get(cache: &mut ChunkCache, p: [i32; 3]) -> Option<u32> {
        if !cache.in_height(p[1]) || !cache.is_loaded(p[0], p[2]) {
            return None;
        }
        Some(cache.get_block(p[0], p[1], p[2]))
    }

    fn set(cache: &mut ChunkCache, p: [i32; 3], id: u32, out: &mut FlowOutput) {
        if cache.get_liquid(p[0], p[1], p[2]) != BLOCKS.air && !block_liquid::can_contain_liquid(id)
        {
            out.liquid_cleared.push(p);
        }
        cache.set_block(p[0], p[1], p[2], id);
        out.changed.push(p);
    }

    /// PMMP `Liquid::onNearbyBlockChange`.
    fn nearby_block_change(&mut self, cache: &mut ChunkCache, pos: [i32; 3], out: &mut FlowOutput) {
        let Some(liquid) = Self::get(cache, pos).and_then(Liquid::from_id) else {
            return;
        };
        // Nether : l'eau s'évapore (dragonfly `Water::NeighbourUpdateTick`, `WaterEvaporates` ; les particules
        // sont produites par le client).
        if self.nether && liquid.kind == Kind::Water {
            Self::set(cache, pos, BLOCKS.air, out);
            return;
        }
        if !self.check_for_harden(cache, pos, liquid, out) {
            self.schedule(pos, self.tick_rate(liquid.kind));
        }
    }

    /// PMMP `canBeFlowedInto`, d'après BDS : tout bloc qui n'arrête pas les liquides.
    fn can_be_flowed_into(id: u32) -> bool {
        block_liquid::liquid_reaction(id) != LiquidReaction::Blocking
    }

    /// PMMP `Liquid::canFlowInto`.
    fn can_flow_into(cache: &mut ChunkCache, p: [i32; 3]) -> bool {
        Self::get(cache, p).is_some_and(|id| {
            Self::can_be_flowed_into(id) && !Liquid::from_id(id).is_some_and(Liquid::is_source)
        })
    }

    /// PMMP `getEffectiveFlowDecay` : déclin d'un liquide du même type (0 s'il tombe), sinon -1.
    fn effective_decay(kind: Kind, id: Option<u32>) -> i32 {
        match id.and_then(Liquid::from_id) {
            Some(l) if l.kind == kind => {
                if l.falling {
                    0
                } else {
                    l.decay as i32
                }
            }
            _ => -1,
        }
    }

    /// PMMP `Liquid::onScheduledUpdate`.
    fn scheduled_update(&mut self, cache: &mut ChunkCache, pos: [i32; 3], out: &mut FlowOutput) {
        let Some(mut this) = Self::get(cache, pos).and_then(Liquid::from_id) else {
            return;
        };
        let kind = this.kind;
        let multiplier = self.decay_per_block(kind);
        let [x, y, z] = pos;

        if !this.is_source() {
            let mut adjacent_sources = 0;
            let mut smallest = -100i32;
            for side in [[x, y, z - 1], [x, y, z + 1], [x - 1, y, z], [x + 1, y, z]] {
                // PMMP `getSmallestFlowDecay`.
                let Some(l) = Self::get(cache, side).and_then(Liquid::from_id) else {
                    continue;
                };
                if l.kind != kind {
                    continue;
                }
                let mut block_decay = l.decay as i32;
                if l.is_source() {
                    adjacent_sources += 1;
                } else if l.falling {
                    block_decay = 0;
                }
                smallest = if smallest >= 0 && block_decay >= smallest {
                    smallest
                } else {
                    block_decay
                };
            }

            let mut new_decay = smallest + multiplier as i32;
            let mut falling = false;
            if new_decay > MAX_DECAY as i32 || smallest < 0 {
                new_decay = -1;
            }
            if Self::effective_decay(kind, Self::get(cache, [x, y + 1, z])) >= 0 {
                falling = true;
            }
            // Eau : une source se forme entre deux sources, au-dessus d'un bloc solide ou d'une source.
            if kind == Kind::Water && adjacent_sources >= 2 {
                let below = Self::get(cache, [x, y - 1, z]);
                let solid = below.is_some_and(block_liquid::is_solid);
                let source = below
                    .and_then(Liquid::from_id)
                    .is_some_and(|l| l.kind == kind && l.is_source());
                if solid || source {
                    new_decay = 0;
                    falling = false;
                }
            }

            if falling != this.falling || (!falling && new_decay != this.decay as i32) {
                if !falling && new_decay < 0 {
                    Self::set(cache, pos, BLOCKS.air, out);
                    return;
                }
                this = Liquid::flowing(kind, new_decay.max(0) as u8, falling);
                Self::set(cache, pos, this.id(), out);
            }
        }

        let below = [x, y - 1, z];
        self.flow_into_block(cache, below, this, 0, true, out);

        let below_open = Self::get(cache, below).is_some_and(Self::can_be_flowed_into);
        if this.is_source() || !below_open {
            let adjacent_decay = if this.falling {
                1 // un liquide qui tombe se comporte comme une source
            } else {
                this.decay + multiplier
            };
            if adjacent_decay <= MAX_DECAY {
                for face in Self::optimal_flow_directions(cache, pos, multiplier) {
                    self.flow_into_block(
                        cache,
                        add(pos, HORIZONTAL[face]),
                        this,
                        adjacent_decay,
                        false,
                        out,
                    );
                }
            }
        }

        self.check_for_harden(cache, pos, this, out);
    }

    /// PMMP `Liquid::flowIntoBlock` (et `Lava::flowIntoBlock` : la lave change l'eau en pierre).
    fn flow_into_block(
        &mut self,
        cache: &mut ChunkCache,
        target: [i32; 3],
        this: Liquid,
        decay: u8,
        falling: bool,
        out: &mut FlowOutput,
    ) {
        let Some(id) = Self::get(cache, target) else {
            return;
        };
        let target_liquid = Liquid::from_id(id);
        if this.kind == Kind::Lava && target_liquid.is_some_and(|l| l.kind == Kind::Water) {
            Self::set(cache, target, BLOCKS.stone, out);
            return;
        }
        if !Self::can_flow_into(cache, target) || target_liquid.is_some() {
            return;
        }
        if id != BLOCKS.air && block_liquid::liquid_reaction(id) == LiquidReaction::Popped {
            out.drops.push((target, id));
        }
        Self::set(
            cache,
            target,
            Liquid::flowing(this.kind, decay, falling).id(),
            out,
        );
    }

    /// PMMP `Lava::checkForHarden` (l'eau ne durcit pas) : la lave au contact de l'eau devient obsidienne
    /// (source) ou galet (déclin ≤ 4) ; sur de la terre des âmes au contact de glace bleue, basalte.
    fn check_for_harden(
        &mut self,
        cache: &mut ChunkCache,
        pos: [i32; 3],
        this: Liquid,
        out: &mut FlowOutput,
    ) -> bool {
        if this.kind != Kind::Lava || this.falling {
            return false;
        }
        let ids = &*IDS;
        let sides: Vec<Option<u32>> = [[0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]]
            .iter()
            .map(|&d| Self::get(cache, add(pos, d)))
            .collect();
        let touches_water = sides.iter().any(|&b| {
            b.and_then(Liquid::from_id)
                .is_some_and(|l| l.kind == Kind::Water)
        });
        if touches_water {
            if this.decay == 0 {
                Self::set(cache, pos, ids.obsidian, out);
                return true;
            } else if this.decay <= 4 {
                Self::set(cache, pos, ids.cobblestone, out);
                return true;
            }
        }
        if Self::get(cache, add(pos, [0, -1, 0])) == Some(ids.soul_soil)
            && sides.contains(&Some(ids.blue_ice))
        {
            Self::set(cache, pos, ids.basalt, out);
            return true;
        }
        false
    }

    /// PMMP `MinimumCostFlowCalculator::getOptimalFlowDirections` : faces (indices de `HORIZONTAL`) qui
    /// mènent au plus court vers un trou où le liquide peut tomber.
    fn optimal_flow_directions(
        cache: &mut ChunkCache,
        origin: [i32; 3],
        decay_per_block: u8,
    ) -> Vec<usize> {
        let mut visited: HashMap<[i32; 3], FlowStatus> = HashMap::new();
        let mut cost = [1000i32; 4];
        let mut max_cost = 4 / decay_per_block as i32;
        for face in 0..4 {
            let p = add(origin, HORIZONTAL[face]);
            if !Self::can_flow_into(cache, p) {
                visited.insert(p, FlowStatus::Blocked);
            } else if Self::get(cache, add(p, [0, -1, 0])).is_some_and(Self::can_be_flowed_into) {
                visited.insert(p, FlowStatus::CanFlowDown);
                cost[face] = 0;
                max_cost = 0;
            } else if max_cost > 0 {
                visited.insert(p, FlowStatus::CanFlow);
                let o = opposite(face);
                cost[face] = Self::flow_cost(cache, &mut visited, p, 1, max_cost, o, o);
                max_cost = max_cost.min(cost[face]);
            }
        }
        let min = *cost.iter().min().unwrap();
        (0..4).filter(|&f| cost[f] == min).collect()
    }

    /// PMMP `MinimumCostFlowCalculator::calculateFlowCost`.
    fn flow_cost(
        cache: &mut ChunkCache,
        visited: &mut HashMap<[i32; 3], FlowStatus>,
        p: [i32; 3],
        accumulated: i32,
        max_cost: i32,
        origin_opposite: usize,
        last_opposite: usize,
    ) -> i32 {
        let mut cost = 1000;
        for (face, &offset) in HORIZONTAL.iter().enumerate() {
            if face == origin_opposite || face == last_opposite {
                continue;
            }
            let q = add(p, offset);
            let status = match visited.get(&q) {
                Some(&s) => s,
                None => {
                    let s = if !Self::can_flow_into(cache, q) {
                        FlowStatus::Blocked
                    } else if Self::get(cache, add(q, [0, -1, 0]))
                        .is_some_and(Self::can_be_flowed_into)
                    {
                        FlowStatus::CanFlowDown
                    } else {
                        FlowStatus::CanFlow
                    };
                    visited.insert(q, s);
                    s
                }
            };
            match status {
                FlowStatus::Blocked => continue,
                FlowStatus::CanFlowDown => return accumulated,
                FlowStatus::CanFlow => {}
            }
            if accumulated >= max_cost {
                continue;
            }
            let real = Self::flow_cost(
                cache,
                visited,
                q,
                accumulated + 1,
                max_cost,
                origin_opposite,
                opposite(face),
            );
            cost = cost.min(real);
        }
        cost
    }
}

#[derive(Clone, Copy)]
enum FlowStatus {
    CanFlowDown,
    CanFlow,
    Blocked,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_cache(name: &str) -> ChunkCache {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mc-rs-flow-{name}-{unique}"));
        let mut cache = ChunkCache::new(&dir, 1, "flat");
        // Charger la zone de test.
        for cx in -2..=2 {
            for cz in -2..=2 {
                cache.get_block(cx * 16, 0, cz * 16);
            }
        }
        cache
    }

    /// Hauteur du sol du monde plat : premier bloc d'air au-dessus de la colonne (0, 0).
    fn ground(cache: &mut ChunkCache) -> i32 {
        (-64..320)
            .find(|&y| cache.get_block(0, y, 0) == BLOCKS.air)
            .unwrap()
    }

    fn run(flow: &mut LiquidFlow, cache: &mut ChunkCache, ticks: u32) {
        for _ in 0..ticks {
            flow.tick(cache);
        }
    }

    #[test]
    fn states_round_trip() {
        for id in [
            IDS.water,
            IDS.flowing_water + 9,
            IDS.lava + 3,
            IDS.flowing_lava + 15,
        ] {
            assert_eq!(Liquid::from_id(id).unwrap().id(), id);
        }
        assert!(Liquid::from_id(BLOCKS.stone).is_none());
        let l = Liquid::from_id(IDS.flowing_water + 8 + 2).unwrap();
        assert!(l.falling && l.decay == 2 && !l.still);
    }

    /// Une source d'eau posée sur le sol s'étale sur 7 blocs, avec un déclin croissant, puis s'arrête.
    #[test]
    fn water_spreads_seven_blocks_on_flat_ground() {
        let mut cache = flat_cache("spread");
        let y = ground(&mut cache);
        cache.set_block(0, y, 0, IDS.water);
        let mut flow = LiquidFlow::new(false);
        run(&mut flow, &mut cache, 200);
        for d in 1..=7 {
            let l = Liquid::from_id(cache.get_block(d, y, 0))
                .unwrap_or_else(|| panic!("pas d'eau à {d}"));
            assert_eq!(l.decay, d as u8, "déclin à {d}");
        }
        assert_eq!(cache.get_block(8, y, 0), BLOCKS.air);
        assert_eq!(flow.pending(), 0, "l'écoulement doit se stabiliser");
    }

    /// Retirer la source : l'eau qui coulait disparaît.
    #[test]
    fn water_dries_when_source_removed() {
        let mut cache = flat_cache("dry");
        let y = ground(&mut cache);
        cache.set_block(0, y, 0, IDS.water);
        let mut flow = LiquidFlow::new(false);
        run(&mut flow, &mut cache, 200);
        cache.set_block(0, y, 0, BLOCKS.air);
        run(&mut flow, &mut cache, 400);
        for d in 0..=8 {
            assert_eq!(cache.get_block(d, y, 0), BLOCKS.air, "eau restante à {d}");
        }
    }

    /// Deux sources séparées d'un bloc : une nouvelle source se forme entre elles (eau infinie).
    #[test]
    fn two_sources_form_a_third() {
        let mut cache = flat_cache("infinite");
        let y = ground(&mut cache);
        cache.set_block(0, y, 0, IDS.water);
        cache.set_block(2, y, 0, IDS.water);
        let mut flow = LiquidFlow::new(false);
        run(&mut flow, &mut cache, 100);
        assert!(Liquid::from_id(cache.get_block(1, y, 0))
            .unwrap()
            .is_source());
    }

    /// L'eau ne coule que vers le trou le plus proche (chemin de moindre coût), puis y tombe.
    #[test]
    fn water_flows_toward_hole_and_falls() {
        let mut cache = flat_cache("fall");
        let y = ground(&mut cache);
        cache.set_block(3, y - 1, 0, BLOCKS.air);
        cache.set_block(0, y, 0, IDS.water);
        let mut flow = LiquidFlow::new(false);
        run(&mut flow, &mut cache, 300);
        let fallen = Liquid::from_id(cache.get_block(3, y - 1, 0)).expect("eau dans le trou");
        assert!(fallen.falling);
        assert_eq!(
            cache.get_block(-1, y, 0),
            BLOCKS.air,
            "l'eau ne part pas à l'opposé du trou"
        );
    }

    /// Une source de lave touchée par l'eau devient de l'obsidienne.
    #[test]
    fn water_hardens_lava_source_into_obsidian() {
        let mut cache = flat_cache("harden");
        let y = ground(&mut cache);
        cache.set_block(0, y, 0, IDS.water);
        cache.set_block(0, y, 5, IDS.lava);
        let mut flow = LiquidFlow::new(false);
        run(&mut flow, &mut cache, 300);
        assert_eq!(cache.get_block(0, y, 5), IDS.obsidian);
    }

    /// L'eau casse une torche (bloc `POPPED`) et la signale pour ses drops ; elle n'entre pas dans la pierre.
    #[test]
    fn water_pops_torch_and_stops_at_stone() {
        let mut cache = flat_cache("pop");
        let y = ground(&mut cache);
        let torch = BLOCKS.get("minecraft:torch");
        cache.set_block(2, y, 0, torch);
        cache.set_block(-1, y, 0, BLOCKS.stone);
        cache.set_block(0, y, 0, IDS.water);
        let mut flow = LiquidFlow::new(false);
        let mut drops = Vec::new();
        for _ in 0..100 {
            drops.extend(flow.tick(&mut cache).drops);
        }
        assert!(drops.contains(&([2, y, 0], torch)));
        assert!(Liquid::from_id(cache.get_block(2, y, 0)).is_some());
        assert_eq!(cache.get_block(-1, y, 0), BLOCKS.stone);
    }
}
