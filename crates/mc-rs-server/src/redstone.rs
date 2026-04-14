//! Redstone system — port conceptuel de `.reference/PocketMine-MP/src/block/*`
//! (RedstoneWire, RedstoneTorch, Piston, etc.). PMMP a une implémentation
//! très partielle. Ici on modélise la propagation de signal 0-15 et les
//! composants de base.

use std::collections::HashMap;

pub type BlockPos = (i32, i32, i32);

/// Signal strength 0..15.
pub type Signal = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedstoneComponent {
    /// Alimentation directe (levier, bouton pressé, pressure plate).
    PowerSource,
    /// Wire : propage le signal en diminuant de 1 à chaque bloc.
    Wire,
    /// Torch : émet 15 si non inversé par signal arrivant dessus.
    Torch { inverted: bool },
    /// Repeater : propage sans perte après `delay` ticks.
    Repeater { delay: u8, facing: Facing },
    /// Comparator.
    Comparator { subtract_mode: bool },
    /// Piston : s'étend si signal > 0.
    Piston { sticky: bool },
    /// Lamp : s'allume si signal > 0.
    Lamp,
    /// Door : s'ouvre si signal > 0.
    Door,
    /// Dispenser/Dropper : émet un item si signal edge +.
    Dispenser,
    /// Noteblock : joue une note si signal edge +.
    NoteBlock { instrument: u8, pitch: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Facing {
    pub fn offset(&self) -> (i32, i32, i32) {
        match self {
            Self::North => (0, 0, -1),
            Self::South => (0, 0, 1),
            Self::East => (1, 0, 0),
            Self::West => (-1, 0, 0),
            Self::Up => (0, 1, 0),
            Self::Down => (0, -1, 0),
        }
    }
}

/// Réseau redstone d'un chunk/zone. Évalue les signaux via flood-fill.
#[derive(Default)]
pub struct RedstoneNetwork {
    pub components: HashMap<BlockPos, RedstoneComponent>,
    pub signals: HashMap<BlockPos, Signal>,
}

impl RedstoneNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn place(&mut self, pos: BlockPos, component: RedstoneComponent) {
        self.components.insert(pos, component);
    }

    pub fn remove(&mut self, pos: BlockPos) {
        self.components.remove(&pos);
        self.signals.remove(&pos);
    }

    /// Recalcule les signaux par propagation. Algorithme simple flood-fill.
    /// À appeler après tout changement de composant.
    pub fn recompute_signals(&mut self) {
        self.signals.clear();
        // Set signaux des PowerSource à 15.
        let sources: Vec<BlockPos> = self
            .components
            .iter()
            .filter(|(_, c)| matches!(c, RedstoneComponent::PowerSource))
            .map(|(p, _)| *p)
            .collect();
        for pos in sources {
            self.signals.insert(pos, 15);
        }

        // Torches émettent 15 si pas inversées.
        let torches: Vec<(BlockPos, bool)> = self
            .components
            .iter()
            .filter_map(|(p, c)| match c {
                RedstoneComponent::Torch { inverted } => Some((*p, *inverted)),
                _ => None,
            })
            .collect();
        for (pos, inverted) in torches {
            if !inverted {
                self.signals.insert(pos, 15);
            }
        }

        // Propagation greedy (jusqu'à 15 passes = max signal range).
        for _pass in 0..15 {
            let mut changed = false;
            let wires: Vec<BlockPos> = self
                .components
                .iter()
                .filter(|(_, c)| matches!(c, RedstoneComponent::Wire))
                .map(|(p, _)| *p)
                .collect();
            for wire_pos in wires {
                let current = self.signals.get(&wire_pos).copied().unwrap_or(0);
                let neighbors = [
                    (wire_pos.0 + 1, wire_pos.1, wire_pos.2),
                    (wire_pos.0 - 1, wire_pos.1, wire_pos.2),
                    (wire_pos.0, wire_pos.1, wire_pos.2 + 1),
                    (wire_pos.0, wire_pos.1, wire_pos.2 - 1),
                    (wire_pos.0, wire_pos.1 + 1, wire_pos.2),
                    (wire_pos.0, wire_pos.1 - 1, wire_pos.2),
                ];
                let max_in = neighbors
                    .iter()
                    .filter_map(|n| self.signals.get(n).copied())
                    .max()
                    .unwrap_or(0);
                let propagated = max_in.saturating_sub(1);
                if propagated > current {
                    self.signals.insert(wire_pos, propagated);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    pub fn signal_at(&self, pos: BlockPos) -> Signal {
        self.signals.get(&pos).copied().unwrap_or(0)
    }

    pub fn is_powered(&self, pos: BlockPos) -> bool {
        self.signal_at(pos) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_source_signal_15() {
        let mut net = RedstoneNetwork::new();
        net.place((0, 0, 0), RedstoneComponent::PowerSource);
        net.recompute_signals();
        assert_eq!(net.signal_at((0, 0, 0)), 15);
    }

    #[test]
    fn wire_propagates_with_decrease() {
        let mut net = RedstoneNetwork::new();
        net.place((0, 0, 0), RedstoneComponent::PowerSource);
        for i in 1..=5 {
            net.place((i, 0, 0), RedstoneComponent::Wire);
        }
        net.recompute_signals();
        assert_eq!(net.signal_at((1, 0, 0)), 14);
        assert_eq!(net.signal_at((2, 0, 0)), 13);
        assert_eq!(net.signal_at((5, 0, 0)), 10);
    }

    #[test]
    fn wire_cuts_at_15_blocks() {
        let mut net = RedstoneNetwork::new();
        net.place((0, 0, 0), RedstoneComponent::PowerSource);
        for i in 1..=20 {
            net.place((i, 0, 0), RedstoneComponent::Wire);
        }
        net.recompute_signals();
        assert_eq!(net.signal_at((15, 0, 0)), 0);
    }
}
