//! Effets de potions — port sélectif de `.reference/PocketMine-MP/src/entity/effect/*`.
//!
//! Un effet = `EffectType` + durée + amplifier (0, 1, 2...) + visibility + is_ambient.
//! Les effets sont appliqués tick par tick par l'`EffectManager`. Certains sont
//! "instant" (applied une fois puis fini) — InstantHealth, InstantDamage.

use crate::attribute::{ids, AttributeMap};
use std::collections::HashMap;

/// Types d'effets vanilla PMMP (IDs réseau Bedrock).
/// Port de `src/entity/effect/VanillaEffects.php`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EffectKind {
    Speed = 1,
    Slowness = 2,
    Haste = 3,
    MiningFatigue = 4,
    Strength = 5,
    InstantHealth = 6,
    InstantDamage = 7,
    JumpBoost = 8,
    Nausea = 9,
    Regeneration = 10,
    Resistance = 11,
    FireResistance = 12,
    WaterBreathing = 13,
    Invisibility = 14,
    Blindness = 15,
    NightVision = 16,
    Hunger = 17,
    Weakness = 18,
    Poison = 19,
    Wither = 20,
    HealthBoost = 21,
    Absorption = 22,
    Saturation = 23,
    Levitation = 24,
    FatalPoison = 25,
    ConduitPower = 26,
    SlowFalling = 27,
    BadOmen = 28,
    HeroOfTheVillage = 29,
    Darkness = 30,
}

impl EffectKind {
    /// Parse "minecraft:speed" / "speed" / un id numérique vers EffectKind.
    pub fn from_name_or_id(token: &str) -> Option<Self> {
        if let Ok(id) = token.parse::<u8>() {
            return Self::from_id(id);
        }
        let short = token
            .strip_prefix("minecraft:")
            .unwrap_or(token)
            .to_ascii_lowercase();
        match short.as_str() {
            "speed" => Some(Self::Speed),
            "slowness" => Some(Self::Slowness),
            "haste" => Some(Self::Haste),
            "mining_fatigue" => Some(Self::MiningFatigue),
            "strength" => Some(Self::Strength),
            "instant_health" | "healing" => Some(Self::InstantHealth),
            "instant_damage" | "harming" => Some(Self::InstantDamage),
            "jump_boost" | "jump" => Some(Self::JumpBoost),
            "nausea" | "confusion" => Some(Self::Nausea),
            "regeneration" => Some(Self::Regeneration),
            "resistance" | "damage_resistance" => Some(Self::Resistance),
            "fire_resistance" => Some(Self::FireResistance),
            "water_breathing" => Some(Self::WaterBreathing),
            "invisibility" => Some(Self::Invisibility),
            "blindness" => Some(Self::Blindness),
            "night_vision" => Some(Self::NightVision),
            "hunger" => Some(Self::Hunger),
            "weakness" => Some(Self::Weakness),
            "poison" => Some(Self::Poison),
            "wither" => Some(Self::Wither),
            "health_boost" => Some(Self::HealthBoost),
            "absorption" => Some(Self::Absorption),
            "saturation" => Some(Self::Saturation),
            "levitation" => Some(Self::Levitation),
            "fatal_poison" => Some(Self::FatalPoison),
            "conduit_power" => Some(Self::ConduitPower),
            "slow_falling" => Some(Self::SlowFalling),
            "bad_omen" => Some(Self::BadOmen),
            "hero_of_the_village" => Some(Self::HeroOfTheVillage),
            "darkness" => Some(Self::Darkness),
            _ => None,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::Speed),
            2 => Some(Self::Slowness),
            3 => Some(Self::Haste),
            4 => Some(Self::MiningFatigue),
            5 => Some(Self::Strength),
            6 => Some(Self::InstantHealth),
            7 => Some(Self::InstantDamage),
            8 => Some(Self::JumpBoost),
            9 => Some(Self::Nausea),
            10 => Some(Self::Regeneration),
            11 => Some(Self::Resistance),
            12 => Some(Self::FireResistance),
            13 => Some(Self::WaterBreathing),
            14 => Some(Self::Invisibility),
            15 => Some(Self::Blindness),
            16 => Some(Self::NightVision),
            17 => Some(Self::Hunger),
            18 => Some(Self::Weakness),
            19 => Some(Self::Poison),
            20 => Some(Self::Wither),
            21 => Some(Self::HealthBoost),
            22 => Some(Self::Absorption),
            23 => Some(Self::Saturation),
            24 => Some(Self::Levitation),
            25 => Some(Self::FatalPoison),
            26 => Some(Self::ConduitPower),
            27 => Some(Self::SlowFalling),
            28 => Some(Self::BadOmen),
            29 => Some(Self::HeroOfTheVillage),
            30 => Some(Self::Darkness),
            _ => None,
        }
    }

    pub fn id(&self) -> u8 {
        *self as u8
    }

    /// Effets "instantanés" qui ne s'appliquent qu'une fois au ajout.
    /// Port PMMP `InstantEffect::isInstantEffect()`.
    pub fn is_instant(&self) -> bool {
        matches!(self, Self::InstantHealth | Self::InstantDamage)
    }

    /// Retourne la couleur RGB de l'effet (utile pour les particules).
    /// PMMP `Effect::getColor()`.
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Speed => (124, 175, 198),
            Self::Slowness => (90, 108, 129),
            Self::Haste => (217, 192, 67),
            Self::MiningFatigue => (74, 66, 23),
            Self::Strength => (147, 36, 36),
            Self::InstantHealth => (248, 36, 35),
            Self::InstantDamage => (67, 10, 9),
            Self::JumpBoost => (34, 255, 76),
            Self::Nausea => (85, 29, 74),
            Self::Regeneration => (205, 92, 171),
            Self::Resistance => (153, 69, 58),
            Self::FireResistance => (228, 154, 58),
            Self::WaterBreathing => (46, 82, 153),
            Self::Invisibility => (127, 131, 146),
            Self::Blindness => (31, 31, 35),
            Self::NightVision => (31, 31, 161),
            Self::Hunger => (88, 118, 53),
            Self::Weakness => (72, 77, 72),
            Self::Poison => (78, 147, 49),
            Self::Wither => (53, 42, 39),
            Self::HealthBoost => (248, 125, 35),
            Self::Absorption => (36, 107, 251),
            Self::Saturation => (255, 0, 255),
            Self::Levitation => (206, 255, 255),
            Self::FatalPoison => (78, 147, 49),
            Self::ConduitPower => (29, 192, 213),
            Self::SlowFalling => (240, 240, 240),
            Self::BadOmen => (116, 129, 61),
            Self::HeroOfTheVillage => (68, 255, 68),
            Self::Darkness => (41, 39, 35),
        }
    }
}

/// Instance d'un effet appliqué sur une entité.
/// Port PMMP `EffectInstance.php`.
#[derive(Debug, Clone)]
pub struct EffectInstance {
    pub kind: EffectKind,
    /// Durée restante en ticks (20 TPS).
    pub duration_ticks: i32,
    /// Niveau d'amplification (0 = I, 1 = II, 2 = III...).
    pub amplifier: u8,
    pub visible: bool,
    pub ambient: bool,
}

impl EffectInstance {
    pub fn new(kind: EffectKind, duration_ticks: i32, amplifier: u8) -> Self {
        Self {
            kind,
            duration_ticks,
            amplifier,
            visible: true,
            ambient: false,
        }
    }

    /// Niveau affiché (1-based). PMMP `EffectInstance::getEffectLevel()`.
    pub fn effect_level(&self) -> u8 {
        self.amplifier + 1
    }
}

/// Manager d'effets par-entité. Port de `EffectManager.php` + `EffectCollection.php`.
#[derive(Debug, Clone, Default)]
pub struct EffectManager {
    pub effects: HashMap<EffectKind, EffectInstance>,
}

impl EffectManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// PMMP `EffectManager::add()`. Si un effet même kind existe avec un
    /// amplifier inférieur ou une durée inférieure → remplace.
    pub fn add(&mut self, mut eff: EffectInstance) -> bool {
        // Instant effects : apply immediately then drop.
        if eff.kind.is_instant() {
            return true;
        }
        match self.effects.get(&eff.kind) {
            Some(existing)
                if existing.amplifier > eff.amplifier
                    || (existing.amplifier == eff.amplifier
                        && existing.duration_ticks > eff.duration_ticks) =>
            {
                // Ne remplace pas : l'existant est meilleur.
                false
            }
            _ => {
                eff.duration_ticks = eff.duration_ticks.max(0);
                self.effects.insert(eff.kind, eff);
                true
            }
        }
    }

    pub fn remove(&mut self, kind: EffectKind) -> Option<EffectInstance> {
        self.effects.remove(&kind)
    }

    pub fn clear(&mut self) {
        self.effects.clear();
    }

    pub fn has(&self, kind: EffectKind) -> bool {
        self.effects.contains_key(&kind)
    }

    pub fn get(&self, kind: EffectKind) -> Option<&EffectInstance> {
        self.effects.get(&kind)
    }

    /// Tick tous les effets. Décrémente durée, applique les effets périodiques.
    /// Retourne `(healed_damage, applied_damage)` — valeurs à appliquer aux
    /// attrs (HEALTH).
    /// PMMP `EffectManager::tick()`.
    pub fn tick(&mut self, attrs: &mut AttributeMap) -> EffectTickOutcome {
        let mut outcome = EffectTickOutcome::default();
        let mut expired: Vec<EffectKind> = Vec::new();
        for (kind, inst) in self.effects.iter_mut() {
            if inst.duration_ticks <= 0 {
                expired.push(*kind);
                continue;
            }
            inst.duration_ticks -= 1;

            match kind {
                EffectKind::Regeneration => {
                    // Heal toutes les 50 >> amplifier ticks (PMMP).
                    let interval = (50 >> inst.amplifier).max(1);
                    if inst.duration_ticks % interval == 0 {
                        outcome.heal += 1.0;
                    }
                }
                EffectKind::Poison => {
                    let interval = (25 >> inst.amplifier).max(1);
                    if inst.duration_ticks % interval == 0 {
                        let hp = attrs.must_get(ids::HEALTH).current_value;
                        if hp > 1.0 {
                            outcome.damage_magic += 1.0;
                        }
                    }
                }
                EffectKind::Wither => {
                    let interval = (40 >> inst.amplifier).max(1);
                    if inst.duration_ticks % interval == 0 {
                        outcome.damage_magic += 1.0;
                    }
                }
                EffectKind::Hunger => {
                    // Exhaustion augmentée (géré par HungerManager via scale).
                    outcome.hunger_exhaust += 0.005 * (inst.amplifier + 1) as f32;
                }
                EffectKind::Saturation => {
                    outcome.saturation_gain += inst.amplifier as f32 + 1.0;
                }
                EffectKind::HealthBoost | EffectKind::Absorption => {
                    // Géré via attribute modifier (non implémenté ici).
                }
                _ => {}
            }
        }
        for k in expired {
            self.effects.remove(&k);
        }
        outcome
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EffectTickOutcome {
    pub heal: f32,
    pub damage_magic: f32,
    pub hunger_exhaust: f32,
    pub saturation_gain: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_stronger_replaces() {
        let mut mgr = EffectManager::new();
        mgr.add(EffectInstance::new(EffectKind::Speed, 100, 0));
        let added = mgr.add(EffectInstance::new(EffectKind::Speed, 100, 1));
        assert!(added);
        assert_eq!(mgr.get(EffectKind::Speed).unwrap().amplifier, 1);
    }

    #[test]
    fn add_weaker_rejected() {
        let mut mgr = EffectManager::new();
        mgr.add(EffectInstance::new(EffectKind::Speed, 100, 2));
        let added = mgr.add(EffectInstance::new(EffectKind::Speed, 200, 0));
        assert!(!added);
        assert_eq!(mgr.get(EffectKind::Speed).unwrap().amplifier, 2);
    }

    #[test]
    fn regeneration_heals_periodically() {
        let mut mgr = EffectManager::new();
        let mut attrs = AttributeMap::default_for_player();
        attrs.must_get_mut(ids::HEALTH).set_value(10.0, true);
        mgr.add(EffectInstance::new(EffectKind::Regeneration, 200, 0));
        let mut total_heal = 0.0;
        for _ in 0..200 {
            let o = mgr.tick(&mut attrs);
            total_heal += o.heal;
        }
        assert!(total_heal > 0.0);
    }

    #[test]
    fn effect_expires() {
        let mut mgr = EffectManager::new();
        let mut attrs = AttributeMap::default_for_player();
        mgr.add(EffectInstance::new(EffectKind::Speed, 5, 0));
        for _ in 0..10 {
            mgr.tick(&mut attrs);
        }
        assert!(!mgr.has(EffectKind::Speed));
    }

    #[test]
    fn instant_effects_not_stored() {
        let mut mgr = EffectManager::new();
        let added = mgr.add(EffectInstance::new(EffectKind::InstantHealth, 1, 0));
        assert!(added);
        assert!(!mgr.has(EffectKind::InstantHealth));
    }
}
