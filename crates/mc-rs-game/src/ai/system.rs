//! AI tick system — evaluates behaviors and applies outputs to ECS state.

use bevy_ecs::prelude::*;

use crate::breeding;
use crate::components::*;
use crate::game_world::{GameEvent, OutgoingEvents, TickCounter};

use super::behavior::{BehaviorContext, BehaviorOutput, BehaviorType, NearestPlayerInfo};
use super::brain::BehaviorList;
use super::pathfinding;

/// Player snapshot for AI context.
struct PlayerSnapshot {
    entity: Entity,
    runtime_id: u64,
    x: f32,
    y: f32,
    z: f32,
    held_item_name: String,
}

/// Mob snapshot for AI evaluation.
struct MobSnapshot {
    entity: Entity,
    position: (f32, f32, f32),
    speed: f32,
    attack_damage: f32,
    on_ground: bool,
    last_damage_tick: Option<u64>,
    target: Option<(Entity, u64)>,
    mob_type: String,
    in_love: bool,
    is_baby: bool,
}

/// Runs AI behavior evaluation for all alive mobs with a BehaviorList.
pub fn system_ai_tick(world: &mut World) {
    // Step 1: Snapshot all player positions (including held item for tempt)
    let players: Vec<PlayerSnapshot> = {
        let mut q = world
            .query_filtered::<(Entity, &EntityId, &Position, Option<&HeldItemName>), With<Player>>(
            );
        q.iter(world)
            .map(|(e, eid, pos, held)| PlayerSnapshot {
                entity: e,
                runtime_id: eid.runtime_id,
                x: pos.x,
                y: pos.y,
                z: pos.z,
                held_item_name: held.map(|h| h.0.clone()).unwrap_or_default(),
            })
            .collect()
    };

    let current_tick = world.resource::<TickCounter>().0;

    // Step 2: Snapshot all mob entities
    let mob_snapshots: Vec<MobSnapshot> = {
        let mut q = world.query_filtered::<(
            Entity,
            &Position,
            &MovementSpeed,
            &AttackDamage,
            &OnGround,
            &LastDamageTick,
            Option<&AiTarget>,
            &MobType,
            Option<&InLove>,
            Option<&Baby>,
        ), (With<Mob>, With<BehaviorList>, Without<Dead>)>();
        q.iter(world)
            .map(
                |(entity, pos, speed, dmg, on_ground, ldt, target, mob_type, in_love, baby)| {
                    MobSnapshot {
                        entity,
                        position: (pos.x, pos.y, pos.z),
                        speed: speed.0,
                        attack_damage: dmg.0,
                        on_ground: on_ground.0,
                        last_damage_tick: ldt.0,
                        target: target.map(|t| (t.entity, t.runtime_id)),
                        mob_type: mob_type.0.clone(),
                        in_love: in_love.is_some(),
                        is_baby: baby.is_some(),
                    }
                },
            )
            .collect()
    };

    // Step 3: Evaluate behaviors for each mob
    let mut actions: Vec<(Entity, BehaviorOutput, f32)> = Vec::new();

    for mob in &mob_snapshots {
        // Find nearest player
        let nearest_player = find_nearest_player(&players, mob.position.0, mob.position.2);

        // Resolve current target position
        let current_target = mob.target.and_then(|(tent, trid)| {
            players
                .iter()
                .find(|p| p.entity == tent)
                .map(|p| (tent, trid, p.x, p.y, p.z))
        });

        // Find nearest player holding a valid tempt item for this mob type
        let nearest_tempting_player =
            find_nearest_tempting_player(&players, mob.position.0, mob.position.2, &mob.mob_type);

        // Find nearest same-type in-love mob (for breeding)
        let nearest_breed_partner = if mob.in_love && !mob.is_baby {
            find_nearest_breed_partner(&mob_snapshots, mob.entity, &mob.mob_type, mob.position)
        } else {
            None
        };

        let ctx = BehaviorContext {
            mob_position: mob.position,
            mob_speed: mob.speed,
            mob_attack_damage: mob.attack_damage,
            mob_on_ground: mob.on_ground,
            current_tick,
            last_damage_tick: mob.last_damage_tick,
            current_target,
            nearest_player,
            mob_type: mob.mob_type.clone(),
            nearest_tempting_player,
            nearest_breed_partner,
            in_love: mob.in_love,
            is_baby: mob.is_baby,
        };

        // Get BehaviorList and evaluate
        let mut blist = world.get_mut::<BehaviorList>(mob.entity).unwrap();
        let output = evaluate_behaviors(&mut blist, &ctx);
        actions.push((mob.entity, output, mob.speed));
    }

    // Step 4: Apply outputs to ECS state
    for (entity, output, speed) in actions {
        // Apply movement
        if let Some((gx, _gy, gz)) = output.move_to {
            let (cx, cz) = {
                let pos = world.get::<Position>(entity).unwrap();
                (pos.x, pos.z)
            };
            // Check if this is a panic behavior (passive mob with recent damage → use 1.25× speed)
            let effective_speed = if output.attack {
                speed // hostile mobs attacking use base speed
            } else {
                // Check for panic state (passive mob + recent damage)
                let is_panicking = {
                    let dmg = world
                        .get::<AttackDamage>(entity)
                        .map(|d| d.0)
                        .unwrap_or(0.0);
                    let ldt = world.get::<LastDamageTick>(entity).and_then(|l| l.0);
                    let tick = world.resource::<TickCounter>().0;
                    dmg == 0.0 && ldt.map(|t| tick.saturating_sub(t) < 60).unwrap_or(false)
                };
                if is_panicking {
                    speed * 1.25
                } else {
                    speed
                }
            };
            let (vx, vz) = pathfinding::move_toward_flat(cx, cz, gx, gz, effective_speed);
            if let Some(mut vel) = world.get_mut::<Velocity>(entity) {
                vel.x = vx;
                vel.z = vz;
            }
        }

        // Apply rotation
        if let Some((yaw, head_yaw)) = output.look_at {
            if let Some(mut rot) = world.get_mut::<Rotation>(entity) {
                rot.yaw = yaw;
                rot.head_yaw = head_yaw;
            }
        }

        // Apply target changes
        if let Some((target_entity, target_rid)) = output.set_target {
            world.entity_mut(entity).insert(AiTarget {
                entity: target_entity,
                runtime_id: target_rid,
            });
        }
        if output.clear_target {
            world.entity_mut(entity).remove::<AiTarget>();
        }

        // Queue mob attack event
        if output.attack {
            // Read all needed data before mutating
            let attack_data = {
                let target = world.get::<AiTarget>(entity);
                let damage = world.get::<AttackDamage>(entity).map(|d| d.0);
                let mob_rid = world.get::<EntityId>(entity).map(|e| e.runtime_id);
                let mob_pos = world.get::<Position>(entity).map(|p| (p.x, p.z));
                target.map(|t| {
                    (
                        t.runtime_id,
                        damage.unwrap_or(0.0),
                        mob_rid.unwrap_or(0),
                        mob_pos,
                    )
                })
            };

            if let Some((target_rid, damage, mob_rid, mob_pos)) = attack_data {
                // Compute knockback from mob → target using the snapshot
                let kb = if let Some((mx, mz)) = mob_pos {
                    // Find target in the player snapshots we captured earlier
                    let tp = players.iter().find(|p| p.runtime_id == target_rid);
                    if let Some(p) = tp {
                        let dx = p.x - mx;
                        let dz = p.z - mz;
                        let d = (dx * dx + dz * dz).sqrt().max(0.01);
                        (dx / d * 0.4, 0.4_f32, dz / d * 0.4)
                    } else {
                        (0.0, 0.4, 0.0)
                    }
                } else {
                    (0.0, 0.4, 0.0)
                };

                world
                    .resource_mut::<OutgoingEvents>()
                    .events
                    .push(GameEvent::MobAttackPlayer {
                        mob_runtime_id: mob_rid,
                        target_runtime_id: target_rid,
                        damage,
                        knockback: kb,
                    });
            }
        }
    }
}

/// Find the nearest player to a given XZ position.
fn find_nearest_player(players: &[PlayerSnapshot], x: f32, z: f32) -> Option<NearestPlayerInfo> {
    players
        .iter()
        .map(|p| {
            let dist = pathfinding::distance_xz(x, z, p.x, p.z);
            (p.entity, p.runtime_id, dist, (p.x, p.y, p.z))
        })
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
}

/// Find the nearest player holding a valid tempt item for this mob type.
fn find_nearest_tempting_player(
    players: &[PlayerSnapshot],
    x: f32,
    z: f32,
    mob_type: &str,
) -> Option<NearestPlayerInfo> {
    players
        .iter()
        .filter(|p| breeding::is_tempt_item(mob_type, &p.held_item_name))
        .map(|p| {
            let dist = pathfinding::distance_xz(x, z, p.x, p.z);
            (p.entity, p.runtime_id, dist, (p.x, p.y, p.z))
        })
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
}

/// Find the nearest same-type in-love mob (as a breed partner).
fn find_nearest_breed_partner(
    mobs: &[MobSnapshot],
    self_entity: Entity,
    mob_type: &str,
    self_pos: (f32, f32, f32),
) -> Option<(Entity, u64, f32, f32, f32)> {
    let mut best: Option<(Entity, u64, f32, f32, f32, f32)> = None; // + distance
    for m in mobs {
        if m.entity == self_entity || m.mob_type != mob_type || !m.in_love || m.is_baby {
            continue;
        }
        let dist = pathfinding::distance_xz(self_pos.0, self_pos.2, m.position.0, m.position.2);
        if best.as_ref().map(|b| dist < b.5).unwrap_or(true) {
            let eid = m.entity;
            // We need runtime_id but MobSnapshot doesn't have it directly.
            // We use a sentinel 0 — the actual rid is looked up in the ECS by the system.
            // Actually, let's store runtime_id. But MobSnapshot doesn't have it.
            // For BreedGoal we only need position to walk toward, not the runtime_id.
            best = Some((eid, 0, m.position.0, m.position.1, m.position.2, dist));
        }
    }
    best.map(|(e, rid, x, y, z, _)| (e, rid, x, y, z))
}

/// Evaluate all behaviors in a BehaviorList and produce a combined output.
fn evaluate_behaviors(blist: &mut BehaviorList, ctx: &BehaviorContext) -> BehaviorOutput {
    let mut combined = BehaviorOutput::default();

    // 1. Evaluate target selectors (highest priority wins)
    let mut best_ts: Option<usize> = None;
    for (i, b) in blist.behaviors.iter().enumerate() {
        if b.behavior_type() != BehaviorType::TargetSelector {
            continue;
        }
        if let Some(active_idx) = blist.active_target_selector {
            if active_idx == i && b.should_continue(ctx) {
                best_ts = Some(i);
                break; // keep current
            }
        }
        if b.can_start(ctx) && best_ts.is_none() {
            best_ts = Some(i);
        }
    }

    // Handle target selector transitions
    if best_ts != blist.active_target_selector {
        if let Some(old) = blist.active_target_selector {
            blist.behaviors[old].stop();
        }
        if let Some(new) = best_ts {
            let output = blist.behaviors[new].start(ctx);
            merge_output(&mut combined, &output);
        } else {
            // No target selector active — clear target
            combined.clear_target = true;
        }
        blist.active_target_selector = best_ts;
    } else if let Some(idx) = best_ts {
        let output = blist.behaviors[idx].tick(ctx);
        merge_output(&mut combined, &output);
    }

    // 2. Evaluate movement behaviors (highest priority wins)
    let mut best_mv: Option<usize> = None;
    for (i, b) in blist.behaviors.iter().enumerate() {
        if b.behavior_type() != BehaviorType::Movement {
            continue;
        }
        if let Some(active_idx) = blist.active_movement {
            if active_idx == i && b.should_continue(ctx) {
                best_mv = Some(i);
                break;
            }
        }
        if b.can_start(ctx)
            && (best_mv.is_none() || b.priority() < blist.behaviors[best_mv.unwrap()].priority())
        {
            best_mv = Some(i);
        }
    }

    if best_mv != blist.active_movement {
        if let Some(old) = blist.active_movement {
            blist.behaviors[old].stop();
        }
        if let Some(new) = best_mv {
            let output = blist.behaviors[new].start(ctx);
            merge_output(&mut combined, &output);
        }
        blist.active_movement = best_mv;
    } else if let Some(idx) = best_mv {
        let output = blist.behaviors[idx].tick(ctx);
        merge_output(&mut combined, &output);
    }

    // 3. Evaluate passive behaviors (all that can run)
    let mut new_passives = Vec::new();
    for (i, b) in blist.behaviors.iter().enumerate() {
        if b.behavior_type() != BehaviorType::Passive {
            continue;
        }
        let was_active = blist.active_passives.contains(&i);
        if (was_active && b.should_continue(ctx)) || (!was_active && b.can_start(ctx)) {
            new_passives.push(i);
        }
    }

    // Stop old passives
    for &old_idx in &blist.active_passives {
        if !new_passives.contains(&old_idx) {
            blist.behaviors[old_idx].stop();
        }
    }

    // Start/tick passives
    for &idx in &new_passives {
        let was_active = blist.active_passives.contains(&idx);
        if !was_active {
            let output = blist.behaviors[idx].start(ctx);
            merge_output(&mut combined, &output);
        } else {
            let output = blist.behaviors[idx].tick(ctx);
            merge_output(&mut combined, &output);
        }
    }
    blist.active_passives = new_passives;

    combined
}

/// Merge a behavior output into the combined output.
/// Later outputs overwrite earlier ones for move_to and look_at (movement wins over passive).
fn merge_output(combined: &mut BehaviorOutput, output: &BehaviorOutput) {
    if output.move_to.is_some() {
        combined.move_to = output.move_to;
    }
    if output.look_at.is_some() {
        combined.look_at = output.look_at;
    }
    if output.attack {
        combined.attack = true;
    }
    if output.set_target.is_some() {
        combined.set_target = output.set_target;
        combined.clear_target = false;
    }
    if output.clear_target {
        combined.clear_target = true;
        combined.set_target = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_world::GameWorld;

    #[test]
    fn ai_no_crash_no_players() {
        let mut gw = GameWorld::new(1);
        gw.spawn_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        // Tick several times — should not crash
        for _ in 0..20 {
            gw.tick();
        }
    }

    #[test]
    fn cow_strolls_over_time() {
        let mut gw = GameWorld::new(1);
        gw.spawn_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        // Tick many times to let the cow move
        for _ in 0..200 {
            gw.tick();
        }

        // The cow should have moved from origin (or at least emitted move events)
        let events = gw.drain_events();
        let move_count = events
            .iter()
            .filter(|e| matches!(e, GameEvent::MobMoved { .. }))
            .count();
        // Should have generated some movement events
        assert!(move_count > 0, "Cow should have moved at least once");
    }

    #[test]
    fn zombie_targets_nearby_player() {
        let mut gw = GameWorld::new(1);
        let addr: std::net::SocketAddr = "127.0.0.1:19132".parse().unwrap();
        gw.spawn_player(50, 50, (10.0, 4.0, 10.0), addr);
        let (_, zrid) = gw.spawn_mob("minecraft:zombie", 5.0, 4.0, 5.0).unwrap();
        gw.drain_events();

        // Tick a few times — zombie should detect player and move toward them
        for _ in 0..5 {
            gw.tick();
        }

        // Check zombie moved toward player (x should have increased from 5.0)
        let pos = gw.mob_position(zrid).unwrap();
        assert!(
            pos.0 > 5.0 || pos.2 > 5.0,
            "Zombie should move toward player, pos={:?}",
            pos
        );
    }
}
