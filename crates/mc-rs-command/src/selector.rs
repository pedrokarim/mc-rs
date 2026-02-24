//! Entity selector resolution: @s, @a, @p, @r, @e.
//!
//! Parses and resolves Bedrock-style entity selectors to player names.
//! No bracket arguments (e.g. `[r=10]`) in this version.

use std::cmp::Ordering;

/// A parsed entity selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selector {
    /// @s — the command sender.
    Sender,
    /// @a — all online players.
    AllPlayers,
    /// @p — nearest player to the sender.
    NearestPlayer,
    /// @r — a random online player.
    RandomPlayer,
    /// @e — all entities (currently same as @a).
    AllEntities,
}

/// Simplified player info for selector resolution.
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Try to parse a string as an entity selector.
///
/// Returns `None` if the string is not a valid selector.
pub fn parse_selector(s: &str) -> Option<Selector> {
    match s {
        "@s" => Some(Selector::Sender),
        "@a" => Some(Selector::AllPlayers),
        "@p" => Some(Selector::NearestPlayer),
        "@r" => Some(Selector::RandomPlayer),
        "@e" => Some(Selector::AllEntities),
        _ => None,
    }
}

/// Resolve a selector to a list of matching player names.
pub fn resolve_selector(
    selector: Selector,
    sender: &str,
    sender_pos: (f32, f32, f32),
    players: &[PlayerInfo],
) -> Result<Vec<String>, String> {
    match selector {
        Selector::Sender => Ok(vec![sender.to_string()]),
        Selector::AllPlayers | Selector::AllEntities => {
            let names: Vec<String> = players.iter().map(|p| p.name.clone()).collect();
            if names.is_empty() {
                Err("No targets found".to_string())
            } else {
                Ok(names)
            }
        }
        Selector::NearestPlayer => players
            .iter()
            .min_by(|a, b| {
                let da = distance_sq(sender_pos, (a.x, a.y, a.z));
                let db = distance_sq(sender_pos, (b.x, b.y, b.z));
                da.partial_cmp(&db).unwrap_or(Ordering::Equal)
            })
            .map(|p| vec![p.name.clone()])
            .ok_or_else(|| "No targets found".to_string()),
        Selector::RandomPlayer => {
            if players.is_empty() {
                Err("No targets found".to_string())
            } else {
                let idx = simple_random(players.len());
                Ok(vec![players[idx].name.clone()])
            }
        }
    }
}

/// Resolve a target argument: either a selector (@s, @a, etc.) or a literal player name.
pub fn resolve_target(
    target: &str,
    sender: &str,
    sender_pos: (f32, f32, f32),
    players: &[PlayerInfo],
) -> Result<Vec<String>, String> {
    if let Some(selector) = parse_selector(target) {
        resolve_selector(selector, sender, sender_pos, players)
    } else if players.iter().any(|p| p.name == target) {
        Ok(vec![target.to_string()])
    } else {
        Err(format!("Player not found: {target}"))
    }
}

fn distance_sq(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)
}

fn simple_random(max: usize) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    nanos % max
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_players() -> Vec<PlayerInfo> {
        vec![
            PlayerInfo {
                name: "Alice".into(),
                x: 10.0,
                y: 64.0,
                z: 10.0,
            },
            PlayerInfo {
                name: "Bob".into(),
                x: 100.0,
                y: 64.0,
                z: 100.0,
            },
            PlayerInfo {
                name: "Charlie".into(),
                x: 50.0,
                y: 64.0,
                z: 50.0,
            },
        ]
    }

    #[test]
    fn parse_all_selectors() {
        assert_eq!(parse_selector("@s"), Some(Selector::Sender));
        assert_eq!(parse_selector("@a"), Some(Selector::AllPlayers));
        assert_eq!(parse_selector("@p"), Some(Selector::NearestPlayer));
        assert_eq!(parse_selector("@r"), Some(Selector::RandomPlayer));
        assert_eq!(parse_selector("@e"), Some(Selector::AllEntities));
    }

    #[test]
    fn parse_invalid() {
        assert_eq!(parse_selector("Steve"), None);
        assert_eq!(parse_selector("@x"), None);
        assert_eq!(parse_selector("@"), None);
        assert_eq!(parse_selector(""), None);
    }

    #[test]
    fn resolve_sender() {
        let players = make_players();
        let result =
            resolve_selector(Selector::Sender, "Alice", (10.0, 64.0, 10.0), &players).unwrap();
        assert_eq!(result, vec!["Alice"]);
    }

    #[test]
    fn resolve_all_players() {
        let players = make_players();
        let result =
            resolve_selector(Selector::AllPlayers, "Alice", (0.0, 0.0, 0.0), &players).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"Alice".to_string()));
        assert!(result.contains(&"Bob".to_string()));
        assert!(result.contains(&"Charlie".to_string()));
    }

    #[test]
    fn resolve_all_entities() {
        let players = make_players();
        let result =
            resolve_selector(Selector::AllEntities, "Alice", (0.0, 0.0, 0.0), &players).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn resolve_nearest_player() {
        let players = make_players();
        // Sender at (0, 64, 0) — Alice at (10, 64, 10) is closest
        let result =
            resolve_selector(Selector::NearestPlayer, "Alice", (0.0, 64.0, 0.0), &players).unwrap();
        assert_eq!(result, vec!["Alice"]);
    }

    #[test]
    fn resolve_nearest_player_different_pos() {
        let players = make_players();
        // Sender at (90, 64, 90) — Bob at (100, 64, 100) is closest
        let result = resolve_selector(
            Selector::NearestPlayer,
            "Alice",
            (90.0, 64.0, 90.0),
            &players,
        )
        .unwrap();
        assert_eq!(result, vec!["Bob"]);
    }

    #[test]
    fn resolve_random_player() {
        let players = make_players();
        let result =
            resolve_selector(Selector::RandomPlayer, "Alice", (0.0, 0.0, 0.0), &players).unwrap();
        assert_eq!(result.len(), 1);
        assert!(players.iter().any(|p| p.name == result[0]));
    }

    #[test]
    fn resolve_empty_players() {
        let empty: Vec<PlayerInfo> = vec![];
        assert!(resolve_selector(Selector::AllPlayers, "Alice", (0.0, 0.0, 0.0), &empty).is_err());
        assert!(
            resolve_selector(Selector::NearestPlayer, "Alice", (0.0, 0.0, 0.0), &empty).is_err()
        );
        assert!(
            resolve_selector(Selector::RandomPlayer, "Alice", (0.0, 0.0, 0.0), &empty).is_err()
        );
    }

    #[test]
    fn resolve_target_literal() {
        let players = make_players();
        let result = resolve_target("Bob", "Alice", (0.0, 0.0, 0.0), &players).unwrap();
        assert_eq!(result, vec!["Bob"]);
    }

    #[test]
    fn resolve_target_not_found() {
        let players = make_players();
        assert!(resolve_target("Unknown", "Alice", (0.0, 0.0, 0.0), &players).is_err());
    }

    #[test]
    fn resolve_target_selector() {
        let players = make_players();
        let result = resolve_target("@s", "Alice", (0.0, 0.0, 0.0), &players).unwrap();
        assert_eq!(result, vec!["Alice"]);
    }
}
