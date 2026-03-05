//! Behavior implementations for mob AI.

use rand::Rng;

use super::behavior::{Behavior, BehaviorContext, BehaviorOutput, BehaviorType};
use super::pathfinding;

// ---------------------------------------------------------------------------
// Float (Movement, priority 0) — swim up if below floor
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct Float;

impl Float {
    pub fn new() -> Self {
        Self
    }
}

impl Behavior for Float {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::Movement
    }

    fn priority(&self) -> u32 {
        0
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        ctx.mob_position.1 < 3.5 // below the flat world floor
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        BehaviorOutput {
            move_to: Some((ctx.mob_position.0, 4.0, ctx.mob_position.2)),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// RandomStroll (Movement, priority 7) — wander aimlessly
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct RandomStroll {
    /// Current stroll destination, if any.
    goal: Option<(f32, f32)>,
    /// Tick when the mob can pick a new destination.
    cooldown_until: u64,
}

impl RandomStroll {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Behavior for RandomStroll {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::Movement
    }

    fn priority(&self) -> u32 {
        7
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        ctx.mob_on_ground && ctx.current_tick >= self.cooldown_until && self.goal.is_none()
    }

    fn should_continue(&self, _ctx: &BehaviorContext) -> bool {
        self.goal.is_some()
    }

    fn start(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        let mut rng = rand::thread_rng();
        let dx: f32 = rng.gen_range(-10.0..10.0);
        let dz: f32 = rng.gen_range(-10.0..10.0);
        let gx = ctx.mob_position.0 + dx;
        let gz = ctx.mob_position.2 + dz;
        self.goal = Some((gx, gz));

        let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, gx, gz);
        BehaviorOutput {
            move_to: Some((gx, 4.0, gz)),
            look_at: Some((yaw, yaw)),
            ..Default::default()
        }
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        let (gx, gz) = match self.goal {
            Some(g) => g,
            None => return BehaviorOutput::default(),
        };

        let dist = pathfinding::distance_xz(ctx.mob_position.0, ctx.mob_position.2, gx, gz);
        if dist < 0.5 {
            // Arrived — set cooldown and clear goal
            self.goal = None;
            let mut rng = rand::thread_rng();
            self.cooldown_until = ctx.current_tick + rng.gen_range(40..120);
            return BehaviorOutput::default();
        }

        let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, gx, gz);
        BehaviorOutput {
            move_to: Some((gx, 4.0, gz)),
            look_at: Some((yaw, yaw)),
            ..Default::default()
        }
    }

    fn stop(&mut self) {
        self.goal = None;
    }
}

// ---------------------------------------------------------------------------
// LookAtPlayer (Passive, priority 8) — face nearest player
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct LookAtPlayer {
    /// Maximum detection range (blocks).
    range: f32,
}

impl LookAtPlayer {
    pub fn new(range: f32) -> Self {
        Self { range }
    }
}

impl Behavior for LookAtPlayer {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::Passive
    }

    fn priority(&self) -> u32 {
        8
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        match &ctx.nearest_player {
            Some((_, _, dist, _)) => *dist <= self.range,
            None => false,
        }
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        if let Some((_, _, _, (px, _, pz))) = &ctx.nearest_player {
            let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, *px, *pz);
            BehaviorOutput {
                look_at: Some((yaw, yaw)),
                ..Default::default()
            }
        } else {
            BehaviorOutput::default()
        }
    }
}

// ---------------------------------------------------------------------------
// MeleeAttack (Movement, priority 2) — chase and hit target
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct MeleeAttack {
    /// Ticks between attacks.
    attack_interval: u64,
    /// Tick of last attack.
    last_attack_tick: u64,
}

impl MeleeAttack {
    pub fn new(attack_interval: u64) -> Self {
        Self {
            attack_interval,
            last_attack_tick: 0,
        }
    }
}

impl Behavior for MeleeAttack {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::Movement
    }

    fn priority(&self) -> u32 {
        2
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        ctx.current_target.is_some()
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        let (_, _, tx, ty, tz) = match ctx.current_target {
            Some(t) => t,
            None => return BehaviorOutput::default(),
        };

        let dist = pathfinding::distance_xz(ctx.mob_position.0, ctx.mob_position.2, tx, tz);
        let attack_range = 2.0;

        let mut output = BehaviorOutput {
            move_to: Some((tx, ty, tz)),
            look_at: Some({
                let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, tx, tz);
                (yaw, yaw)
            }),
            ..Default::default()
        };

        if dist <= attack_range
            && ctx.current_tick.saturating_sub(self.last_attack_tick) >= self.attack_interval
        {
            output.attack = true;
            self.last_attack_tick = ctx.current_tick;
        }

        output
    }
}

// ---------------------------------------------------------------------------
// NearestAttackableTarget (TargetSelector, priority 1)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct NearestAttackableTarget {
    /// Maximum detection range.
    range: f32,
}

impl NearestAttackableTarget {
    pub fn new(range: f32) -> Self {
        Self { range }
    }
}

impl Behavior for NearestAttackableTarget {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::TargetSelector
    }

    fn priority(&self) -> u32 {
        1
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        ctx.current_target.is_none()
            && ctx
                .nearest_player
                .as_ref()
                .map(|(_, _, d, _)| *d <= self.range)
                .unwrap_or(false)
    }

    fn should_continue(&self, ctx: &BehaviorContext) -> bool {
        // Keep targeting as long as target exists and is within 2× range
        ctx.current_target
            .map(|(_, _, tx, _, tz)| {
                pathfinding::distance_xz(ctx.mob_position.0, ctx.mob_position.2, tx, tz)
                    <= self.range * 2.0
            })
            .unwrap_or(false)
    }

    fn start(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        if let Some((entity, rid, _, _)) = &ctx.nearest_player {
            BehaviorOutput {
                set_target: Some((*entity, *rid)),
                ..Default::default()
            }
        } else {
            BehaviorOutput::default()
        }
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        // Re-check if target is still valid; if not, clear
        if ctx.current_target.is_none() {
            if let Some((entity, rid, dist, _)) = &ctx.nearest_player {
                if *dist <= self.range {
                    return BehaviorOutput {
                        set_target: Some((*entity, *rid)),
                        ..Default::default()
                    };
                }
            }
            return BehaviorOutput {
                clear_target: true,
                ..Default::default()
            };
        }
        BehaviorOutput::default()
    }

    fn stop(&mut self) {}
}

// ---------------------------------------------------------------------------
// HurtByTarget (TargetSelector, priority 0) — target attacker after damage
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct HurtByTarget {
    /// Ticks after damage during which this behavior is active.
    memory_ticks: u64,
}

impl Default for HurtByTarget {
    fn default() -> Self {
        Self { memory_ticks: 60 }
    }
}

impl HurtByTarget {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Behavior for HurtByTarget {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::TargetSelector
    }

    fn priority(&self) -> u32 {
        0
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        if let Some(last) = ctx.last_damage_tick {
            ctx.current_tick.saturating_sub(last) < self.memory_ticks
                && ctx.nearest_player.is_some()
        } else {
            false
        }
    }

    fn should_continue(&self, ctx: &BehaviorContext) -> bool {
        if let Some(last) = ctx.last_damage_tick {
            ctx.current_tick.saturating_sub(last) < self.memory_ticks * 2
                && ctx.current_target.is_some()
        } else {
            false
        }
    }

    fn start(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        if let Some((entity, rid, _, _)) = &ctx.nearest_player {
            BehaviorOutput {
                set_target: Some((*entity, *rid)),
                ..Default::default()
            }
        } else {
            BehaviorOutput::default()
        }
    }

    fn tick(&mut self, _ctx: &BehaviorContext) -> BehaviorOutput {
        BehaviorOutput::default()
    }

    fn stop(&mut self) {}
}

// ---------------------------------------------------------------------------
// Panic (Movement, priority 1) — flee after taking damage (passive mobs)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Panic {
    /// Duration of panic in ticks.
    duration: u64,
    /// Speed multiplier during panic.
    speed_mult: f32,
    /// Flee destination.
    flee_goal: Option<(f32, f32)>,
    /// Tick when panic started.
    start_tick: u64,
}

impl Default for Panic {
    fn default() -> Self {
        Self {
            duration: 60,
            speed_mult: 1.25,
            flee_goal: None,
            start_tick: 0,
        }
    }
}

impl Panic {
    pub fn new() -> Self {
        Self::default()
    }

    /// Speed multiplier getter (for testing).
    pub fn speed_multiplier(&self) -> f32 {
        self.speed_mult
    }
}

impl Behavior for Panic {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::Movement
    }

    fn priority(&self) -> u32 {
        1
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        // Only passive mobs panic (attack_damage == 0)
        ctx.mob_attack_damage == 0.0
            && ctx
                .last_damage_tick
                .map(|t| ctx.current_tick.saturating_sub(t) < self.duration)
                .unwrap_or(false)
    }

    fn should_continue(&self, ctx: &BehaviorContext) -> bool {
        ctx.current_tick.saturating_sub(self.start_tick) < self.duration
    }

    fn start(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        self.start_tick = ctx.current_tick;
        let mut rng = rand::thread_rng();
        let dx: f32 = rng.gen_range(-10.0..10.0);
        let dz: f32 = rng.gen_range(-10.0..10.0);
        let gx = ctx.mob_position.0 + dx;
        let gz = ctx.mob_position.2 + dz;
        self.flee_goal = Some((gx, gz));

        let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, gx, gz);
        BehaviorOutput {
            move_to: Some((gx, 4.0, gz)),
            look_at: Some((yaw, yaw)),
            ..Default::default()
        }
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        let (gx, gz) = match self.flee_goal {
            Some(g) => g,
            None => return BehaviorOutput::default(),
        };

        let dist = pathfinding::distance_xz(ctx.mob_position.0, ctx.mob_position.2, gx, gz);
        if dist < 0.5 {
            // Pick new flee destination
            let mut rng = rand::thread_rng();
            let dx: f32 = rng.gen_range(-10.0..10.0);
            let dz: f32 = rng.gen_range(-10.0..10.0);
            let new_gx = ctx.mob_position.0 + dx;
            let new_gz = ctx.mob_position.2 + dz;
            self.flee_goal = Some((new_gx, new_gz));
        }

        let (gx, gz) = self.flee_goal.unwrap();
        let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, gx, gz);
        BehaviorOutput {
            move_to: Some((gx, 4.0, gz)),
            look_at: Some((yaw, yaw)),
            ..Default::default()
        }
    }

    fn stop(&mut self) {
        self.flee_goal = None;
    }
}

// ---------------------------------------------------------------------------
// TemptGoal (Movement, priority 3) — follow player holding tempt food
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct TemptGoal;

impl TemptGoal {
    pub fn new() -> Self {
        Self
    }
}

impl Behavior for TemptGoal {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::Movement
    }

    fn priority(&self) -> u32 {
        3
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        ctx.nearest_tempting_player
            .as_ref()
            .map(|(_, _, d, _)| *d <= 10.0)
            .unwrap_or(false)
    }

    fn should_continue(&self, ctx: &BehaviorContext) -> bool {
        ctx.nearest_tempting_player
            .as_ref()
            .map(|(_, _, d, _)| *d <= 12.0)
            .unwrap_or(false)
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        if let Some((_, _, _, (px, py, pz))) = &ctx.nearest_tempting_player {
            let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, *px, *pz);
            BehaviorOutput {
                move_to: Some((*px, *py, *pz)),
                look_at: Some((yaw, yaw)),
                ..Default::default()
            }
        } else {
            BehaviorOutput::default()
        }
    }
}

// ---------------------------------------------------------------------------
// BreedGoal (Movement, priority 4) — walk toward in-love partner
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct BreedGoal;

impl BreedGoal {
    pub fn new() -> Self {
        Self
    }
}

impl Behavior for BreedGoal {
    fn behavior_type(&self) -> BehaviorType {
        BehaviorType::Movement
    }

    fn priority(&self) -> u32 {
        4
    }

    fn can_start(&self, ctx: &BehaviorContext) -> bool {
        ctx.in_love && !ctx.is_baby && ctx.nearest_breed_partner.is_some()
    }

    fn should_continue(&self, ctx: &BehaviorContext) -> bool {
        if !ctx.in_love || ctx.is_baby {
            return false;
        }
        ctx.nearest_breed_partner
            .map(|(_, _, px, _, pz)| {
                pathfinding::distance_xz(ctx.mob_position.0, ctx.mob_position.2, px, pz) <= 16.0
            })
            .unwrap_or(false)
    }

    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput {
        if let Some((_, _, px, py, pz)) = ctx.nearest_breed_partner {
            let yaw = pathfinding::yaw_toward(ctx.mob_position.0, ctx.mob_position.2, px, pz);
            BehaviorOutput {
                move_to: Some((px, py, pz)),
                look_at: Some((yaw, yaw)),
                ..Default::default()
            }
        } else {
            BehaviorOutput::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::Entity;

    fn base_ctx() -> BehaviorContext {
        BehaviorContext {
            mob_position: (0.0, 4.0, 0.0),
            mob_speed: 0.25,
            mob_attack_damage: 0.0,
            mob_on_ground: true,
            current_tick: 100,
            last_damage_tick: None,
            current_target: None,
            nearest_player: None,
            mob_type: "minecraft:cow".to_string(),
            nearest_tempting_player: None,
            nearest_breed_partner: None,
            in_love: false,
            is_baby: false,
        }
    }

    fn dummy_entity() -> Entity {
        Entity::from_raw(42)
    }

    #[test]
    fn stroll_can_start_when_ready() {
        let stroll = RandomStroll::new();
        let ctx = base_ctx();
        assert!(stroll.can_start(&ctx));
    }

    #[test]
    fn stroll_cant_start_during_cooldown() {
        let mut stroll = RandomStroll::new();
        stroll.cooldown_until = 200;
        let ctx = base_ctx(); // tick=100 < 200
        assert!(!stroll.can_start(&ctx));
    }

    #[test]
    fn stroll_start_sets_goal() {
        let mut stroll = RandomStroll::new();
        let ctx = base_ctx();
        let output = stroll.start(&ctx);
        assert!(output.move_to.is_some());
        assert!(stroll.goal.is_some());
    }

    #[test]
    fn look_at_player_within_range() {
        let look = LookAtPlayer::new(8.0);
        let mut ctx = base_ctx();
        ctx.nearest_player = Some((dummy_entity(), 1, 5.0, (5.0, 4.0, 0.0)));
        assert!(look.can_start(&ctx));
    }

    #[test]
    fn look_at_player_out_of_range() {
        let look = LookAtPlayer::new(8.0);
        let mut ctx = base_ctx();
        ctx.nearest_player = Some((dummy_entity(), 1, 20.0, (20.0, 4.0, 0.0)));
        assert!(!look.can_start(&ctx));
    }

    #[test]
    fn look_at_player_tick_returns_yaw() {
        let mut look = LookAtPlayer::new(8.0);
        let mut ctx = base_ctx();
        ctx.nearest_player = Some((dummy_entity(), 1, 5.0, (5.0, 4.0, 0.0)));
        let output = look.tick(&ctx);
        assert!(output.look_at.is_some());
    }

    #[test]
    fn float_below_floor() {
        let float = Float::new();
        let mut ctx = base_ctx();
        ctx.mob_position = (0.0, 2.0, 0.0);
        assert!(float.can_start(&ctx));
    }

    #[test]
    fn float_at_floor() {
        let float = Float::new();
        let ctx = base_ctx(); // y=4.0
        assert!(!float.can_start(&ctx));
    }

    #[test]
    fn melee_can_start_with_target() {
        let melee = MeleeAttack::new(20);
        let mut ctx = base_ctx();
        ctx.current_target = Some((dummy_entity(), 1, 5.0, 4.0, 0.0));
        assert!(melee.can_start(&ctx));
    }

    #[test]
    fn melee_attacks_in_range() {
        let mut melee = MeleeAttack::new(20);
        let mut ctx = base_ctx();
        ctx.current_target = Some((dummy_entity(), 1, 1.0, 4.0, 0.0));
        ctx.current_tick = 100;
        let output = melee.tick(&ctx);
        assert!(output.attack);
    }

    #[test]
    fn melee_cooldown_respected() {
        let mut melee = MeleeAttack::new(20);
        melee.last_attack_tick = 90;
        let mut ctx = base_ctx();
        ctx.current_target = Some((dummy_entity(), 1, 1.0, 4.0, 0.0));
        ctx.current_tick = 100; // only 10 ticks since last attack, need 20
        let output = melee.tick(&ctx);
        assert!(!output.attack);
    }

    #[test]
    fn nearest_target_selects_player() {
        let mut selector = NearestAttackableTarget::new(16.0);
        let mut ctx = base_ctx();
        ctx.nearest_player = Some((dummy_entity(), 42, 10.0, (10.0, 4.0, 0.0)));
        assert!(selector.can_start(&ctx));
        let output = selector.start(&ctx);
        assert!(output.set_target.is_some());
        assert_eq!(output.set_target.unwrap().1, 42);
    }

    #[test]
    fn hurt_by_target_activates_after_damage() {
        let hbt = HurtByTarget::new();
        let mut ctx = base_ctx();
        ctx.last_damage_tick = Some(80); // 20 ticks ago
        ctx.nearest_player = Some((dummy_entity(), 1, 5.0, (5.0, 4.0, 0.0)));
        assert!(hbt.can_start(&ctx));
    }

    #[test]
    fn hurt_by_target_inactive_without_damage() {
        let hbt = HurtByTarget::new();
        let ctx = base_ctx(); // no damage
        assert!(!hbt.can_start(&ctx));
    }

    #[test]
    fn panic_starts_after_damage() {
        let panic = Panic::new();
        let mut ctx = base_ctx();
        ctx.last_damage_tick = Some(80);
        assert!(panic.can_start(&ctx));
    }

    #[test]
    fn panic_not_for_hostile() {
        let panic = Panic::new();
        let mut ctx = base_ctx();
        ctx.mob_attack_damage = 3.0; // hostile mob
        ctx.last_damage_tick = Some(80);
        assert!(!panic.can_start(&ctx));
    }

    #[test]
    fn panic_stops_after_timeout() {
        let mut panic = Panic::new();
        panic.start_tick = 10;
        let mut ctx = base_ctx();
        ctx.current_tick = 200; // well past 60-tick duration
        assert!(!panic.should_continue(&ctx));
    }

    // TemptGoal tests

    #[test]
    fn tempt_can_start_with_tempting_player() {
        let tempt = TemptGoal::new();
        let mut ctx = base_ctx();
        ctx.nearest_tempting_player = Some((dummy_entity(), 1, 5.0, (5.0, 4.0, 0.0)));
        assert!(tempt.can_start(&ctx));
    }

    #[test]
    fn tempt_cant_start_without_tempting_player() {
        let tempt = TemptGoal::new();
        let ctx = base_ctx();
        assert!(!tempt.can_start(&ctx));
    }

    #[test]
    fn tempt_cant_start_too_far() {
        let tempt = TemptGoal::new();
        let mut ctx = base_ctx();
        ctx.nearest_tempting_player = Some((dummy_entity(), 1, 15.0, (15.0, 4.0, 0.0)));
        assert!(!tempt.can_start(&ctx));
    }

    #[test]
    fn tempt_tick_moves_toward_player() {
        let mut tempt = TemptGoal::new();
        let mut ctx = base_ctx();
        ctx.nearest_tempting_player = Some((dummy_entity(), 1, 5.0, (5.0, 4.0, 0.0)));
        let output = tempt.tick(&ctx);
        assert!(output.move_to.is_some());
        assert!(output.look_at.is_some());
    }

    // BreedGoal tests

    #[test]
    fn breed_can_start_when_in_love_with_partner() {
        let breed = BreedGoal::new();
        let mut ctx = base_ctx();
        ctx.in_love = true;
        ctx.nearest_breed_partner = Some((dummy_entity(), 2, 5.0, 4.0, 0.0));
        assert!(breed.can_start(&ctx));
    }

    #[test]
    fn breed_cant_start_without_love() {
        let breed = BreedGoal::new();
        let mut ctx = base_ctx();
        ctx.nearest_breed_partner = Some((dummy_entity(), 2, 5.0, 4.0, 0.0));
        assert!(!breed.can_start(&ctx));
    }

    #[test]
    fn breed_cant_start_if_baby() {
        let breed = BreedGoal::new();
        let mut ctx = base_ctx();
        ctx.in_love = true;
        ctx.is_baby = true;
        ctx.nearest_breed_partner = Some((dummy_entity(), 2, 5.0, 4.0, 0.0));
        assert!(!breed.can_start(&ctx));
    }

    #[test]
    fn breed_tick_moves_toward_partner() {
        let mut breed = BreedGoal::new();
        let mut ctx = base_ctx();
        ctx.in_love = true;
        ctx.nearest_breed_partner = Some((dummy_entity(), 2, 5.0, 4.0, 3.0));
        let output = breed.tick(&ctx);
        assert!(output.move_to.is_some());
        let (mx, _, mz) = output.move_to.unwrap();
        assert!((mx - 5.0).abs() < 0.01);
        assert!((mz - 3.0).abs() < 0.01);
    }
}
