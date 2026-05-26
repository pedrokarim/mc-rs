//! Helpers génériques pour les handlers de commandes — pures fonctions sans
//! dépendance au runtime serveur. Utilisables par n'importe quel binaire
//! basé sur ce crate.

use crate::map::{CommandDispatchError, CommandParameter, ParamType};
use crate::CommandSender;

/// Renvoie `Err(CommandDispatchError::Usage)` avec le message donné. Sucre
/// syntaxique pour les handlers.
pub fn usage<T>(message: &str) -> Result<T, CommandDispatchError> {
    Err(CommandDispatchError::Usage(message.to_string()))
}

/// Renvoie `Err(CommandDispatchError::Message)` avec le message donné.
pub fn message<T>(message: impl Into<String>) -> Result<T, CommandDispatchError> {
    Err(CommandDispatchError::Message(message.into()))
}

/// Parse une coordonnée Minecraft : `~` = current, `~N` = current+N, sinon
/// valeur absolue. Format vanilla des commandes Bedrock (cf
/// minecraft.makecode.com /reference/positions).
///
/// Le caret `^` (forward facing) n'est pas géré ici — il dépend du yaw du
/// sender, et est principalement utile pour /execute.
pub fn parse_coord(token: &str, current: f32) -> Option<f32> {
    if token == "~" {
        Some(current)
    } else if let Some(offset) = token.strip_prefix('~') {
        if offset.is_empty() {
            Some(current)
        } else {
            offset.parse::<f32>().ok().map(|value| current + value)
        }
    } else {
        token.parse::<f32>().ok()
    }
}

/// Construit un `CommandParameter` basique.
pub fn param(name: &str, param_type: ParamType, optional: bool) -> CommandParameter {
    CommandParameter {
        name: name.into(),
        param_type,
        optional,
    }
}

/// Construit un `CommandParameter` HardEnum (valeurs fixes proposées en
/// autocomplete côté client).
pub fn hard_enum_param(
    name: &str,
    enum_name: &str,
    values: &[&str],
    optional: bool,
) -> CommandParameter {
    param(
        name,
        ParamType::HardEnum {
            name: enum_name.into(),
            values: values.iter().map(|value| (*value).to_string()).collect(),
        },
        optional,
    )
}

/// Construit un `CommandParameter` SoftEnum (valeurs dynamiques résolues
/// à la volée par `SoftEnumSource::soft_enum_values`).
pub fn soft_enum_param(name: &str, enum_name: &str, optional: bool) -> CommandParameter {
    param(
        name,
        ParamType::SoftEnum {
            name: enum_name.into(),
        },
        optional,
    )
}

/// Parse un triplet de coordonnées (x, y, z) relatives à l'origine donnée.
/// `~` = composante de origin, `~N` = origin+N, sinon absolu. Voir
/// [`parse_coord`].
pub fn parse_position_triplet(
    origin: [f32; 3],
    x: &str,
    y: &str,
    z: &str,
) -> Result<[f32; 3], CommandDispatchError> {
    Ok([
        parse_coord(x, origin[0])
            .ok_or_else(|| CommandDispatchError::Message(format!("Invalid X coordinate: {x}")))?,
        parse_coord(y, origin[1])
            .ok_or_else(|| CommandDispatchError::Message(format!("Invalid Y coordinate: {y}")))?,
        parse_coord(z, origin[2])
            .ok_or_else(|| CommandDispatchError::Message(format!("Invalid Z coordinate: {z}")))?,
    ])
}

/// Variante de [`parse_position_triplet`] qui rejette les coordonnées
/// relatives (`~`) si le sender n'est pas un joueur — la console n'a pas de
/// position pour les résoudre.
pub fn parse_position_triplet_for_source(
    sender: &dyn CommandSender,
    player_origin: Option<[f32; 3]>,
    x: &str,
    y: &str,
    z: &str,
) -> Result<[f32; 3], CommandDispatchError> {
    if sender.sender_is_player() {
        let origin = player_origin.ok_or_else(|| {
            CommandDispatchError::Message("Sender position is unavailable.".to_string())
        })?;
        parse_position_triplet(origin, x, y, z)
    } else {
        if [x, y, z].iter().any(|token| token.starts_with('~')) {
            return Err(CommandDispatchError::Message(
                "Console must use absolute coordinates.".to_string(),
            ));
        }
        parse_position_triplet([0.0, 0.0, 0.0], x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coord_absolute() {
        assert_eq!(parse_coord("42", 10.0), Some(42.0));
        assert_eq!(parse_coord("-3.5", 0.0), Some(-3.5));
    }

    #[test]
    fn coord_relative() {
        assert_eq!(parse_coord("~", 5.0), Some(5.0));
        assert_eq!(parse_coord("~10", 5.0), Some(15.0));
        assert_eq!(parse_coord("~-3", 5.0), Some(2.0));
    }

    #[test]
    fn coord_invalid() {
        assert_eq!(parse_coord("abc", 0.0), None);
        assert_eq!(parse_coord("~xyz", 0.0), None);
    }

    #[test]
    fn position_triplet_relative() {
        let pos = parse_position_triplet([10.0, 20.0, 30.0], "~", "~5", "~-3").unwrap();
        assert_eq!(pos, [10.0, 25.0, 27.0]);
    }

    #[test]
    fn position_triplet_absolute() {
        let pos = parse_position_triplet([0.0; 3], "1", "2", "3").unwrap();
        assert_eq!(pos, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn param_builders() {
        let p = param("x", ParamType::Int, false);
        assert_eq!(p.name, "x");
        assert!(!p.optional);
        let hp = hard_enum_param("mode", "gm", &["s", "c"], true);
        assert!(hp.optional);
        match hp.param_type {
            ParamType::HardEnum { name, values } => {
                assert_eq!(name, "gm");
                assert_eq!(values, vec!["s".to_string(), "c".to_string()]);
            }
            _ => panic!("expected HardEnum"),
        }
    }
}
