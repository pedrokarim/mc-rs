//! Entity visibility culling par distance — Phase 7 perf.
//!
//! Filtre les broadcasts d'entités (Add/Move/Motion/Remove) selon la distance
//! au joueur destinataire. Sans culling, chaque mouvement de mob est diffusé
//! à TOUS les joueurs in_game, ce qui sature le réseau dès qu'il y a beaucoup
//! d'entités. PMMP utilise un système équivalent via `World::getViewersForPosition`.
//!
//! Stratégie :
//! - Chaque `Connection` maintient un `visible_entities: HashSet<i64>` des
//!   entity_unique_ids qu'il "voit" actuellement.
//! - À chaque broadcast :
//!   - dans vue + déjà visible → envoyer la mise à jour
//!   - dans vue + pas visible → marquer visible + envoyer Add + mise à jour
//!   - hors vue + visible → marquer invisible + envoyer Remove
//!   - hors vue + pas visible → skip
//! - Un scan périodique (toutes les 10 ticks) revisite les transitions pour
//!   les entités stationnaires (sinon un mob immobile ne deviendrait jamais
//!   visible à l'approche du joueur).

/// Distance horizontale (XZ) au carré — la composante Y est ignorée pour
/// que les entités au sol restent visibles depuis une grande altitude.
pub fn dist_sq_xz(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dz = a[2] - b[2];
    dx * dx + dz * dz
}

/// Rayon de vue d'une entité, en blocs, dérivé du view_distance du joueur
/// (en chunks). On clamp à un minimum raisonnable pour éviter qu'un joueur
/// avec view_distance=1 ne voie plus aucun mob.
pub fn entity_view_radius_blocks(view_distance_chunks: i32) -> f32 {
    let chunks = view_distance_chunks.max(4);
    (chunks * 16) as f32
}

/// Vrai si `entity_pos` est dans le rayon de vue du joueur `conn`.
pub fn is_within_view_for(conn: &crate::connection::Connection, entity_pos: [f32; 3]) -> bool {
    let radius = entity_view_radius_blocks(conn.view_distance);
    dist_sq_xz(conn.position, entity_pos) <= radius * radius
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityTransition {
    /// L'entité reste hors de vue — ne rien envoyer.
    StillHidden,
    /// L'entité reste visible — envoyer Move/Motion/SetActorData habituels.
    StillVisible,
    /// L'entité entre dans la vue — envoyer Add puis Move/Motion.
    JustEntered,
    /// L'entité sort de la vue — envoyer Remove.
    JustLeft,
}

/// Décide la transition de visibilité pour `entity_id` à la position `entity_pos`
/// vu depuis `player_pos` avec `view_distance_chunks`. Mute le set
/// `visible_entities` pour refléter la nouvelle visibilité.
pub fn classify_transition(
    visible_entities: &mut std::collections::HashSet<i64>,
    entity_id: i64,
    entity_pos: [f32; 3],
    player_pos: [f32; 3],
    view_distance_chunks: i32,
) -> VisibilityTransition {
    let radius = entity_view_radius_blocks(view_distance_chunks);
    let in_view = dist_sq_xz(player_pos, entity_pos) <= radius * radius;
    let was_visible = visible_entities.contains(&entity_id);
    match (was_visible, in_view) {
        (false, false) => VisibilityTransition::StillHidden,
        (true, true) => VisibilityTransition::StillVisible,
        (false, true) => {
            visible_entities.insert(entity_id);
            VisibilityTransition::JustEntered
        }
        (true, false) => {
            visible_entities.remove(&entity_id);
            VisibilityTransition::JustLeft
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn dist_sq_ignores_y() {
        let a = [0.0, 100.0, 0.0];
        let b = [3.0, 0.0, 4.0];
        // 3^2 + 4^2 = 25 (la composante Y de 100 est ignorée)
        assert!((dist_sq_xz(a, b) - 25.0).abs() < 0.0001);
    }

    #[test]
    fn radius_clamped_to_minimum() {
        assert!(entity_view_radius_blocks(1) >= 64.0);
        assert!(entity_view_radius_blocks(8) >= 128.0);
    }

    #[test]
    fn transitions_cover_all_quadrants() {
        let mut set = HashSet::new();
        let player = [0.0, 0.0, 0.0];

        // hors vue + pas visible → StillHidden
        let t = classify_transition(&mut set, 1, [1000.0, 0.0, 0.0], player, 4);
        assert_eq!(t, VisibilityTransition::StillHidden);
        assert!(!set.contains(&1));

        // dans vue + pas visible → JustEntered
        let t = classify_transition(&mut set, 1, [10.0, 0.0, 0.0], player, 4);
        assert_eq!(t, VisibilityTransition::JustEntered);
        assert!(set.contains(&1));

        // dans vue + visible → StillVisible
        let t = classify_transition(&mut set, 1, [12.0, 0.0, 0.0], player, 4);
        assert_eq!(t, VisibilityTransition::StillVisible);

        // hors vue + visible → JustLeft
        let t = classify_transition(&mut set, 1, [1000.0, 0.0, 0.0], player, 4);
        assert_eq!(t, VisibilityTransition::JustLeft);
        assert!(!set.contains(&1));
    }
}
