use std::fmt;

use crate::CommandSender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorKind {
    Sender,
    NearestPlayer,
    RandomPlayer,
    AllPlayers,
    AllEntities,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub kind: SelectorKind,
    pub name: Option<String>,
    pub entity_type: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub dx: Option<f32>,
    pub dy: Option<f32>,
    pub dz: Option<f32>,
    pub r: Option<f32>,
    pub rm: Option<f32>,
    pub c: Option<usize>,
    pub gamemode: Option<i32>,
}

impl Selector {
    fn origin(&self, sender: &dyn CommandSender) -> [f32; 3] {
        let sender_pos = sender.sender_position();
        [
            self.x.unwrap_or(sender_pos[0]),
            self.y.unwrap_or(sender_pos[1]),
            self.z.unwrap_or(sender_pos[2]),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectorEntity {
    pub id: u64,
    pub name: Option<String>,
    pub entity_type: String,
    pub position: [f32; 3],
    pub gamemode: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorError {
    InvalidSelector(String),
    UnsupportedSelector(String),
    SenderHasNoEntity,
    NoTargetsMatched,
    InvalidTarget(String),
}

impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectorError::InvalidSelector(message) => write!(f, "{message}"),
            SelectorError::UnsupportedSelector(message) => write!(f, "{message}"),
            SelectorError::SenderHasNoEntity => {
                write!(f, "This command requires an in-game sender")
            }
            SelectorError::NoTargetsMatched => write!(f, "No targets matched"),
            SelectorError::InvalidTarget(token) => write!(f, "Unknown target: {token}"),
        }
    }
}

pub fn parse_selector(token: &str) -> Result<Option<Selector>, SelectorError> {
    if !token.starts_with('@') {
        return Ok(None);
    }

    let (selector_token, filter_blob) = if let Some(bracket_start) = token.find('[') {
        if !token.ends_with(']') {
            return Err(SelectorError::InvalidSelector(format!(
                "Invalid selector syntax: {token}"
            )));
        }
        (
            &token[..bracket_start],
            Some(&token[bracket_start + 1..token.len() - 1]),
        )
    } else {
        (token, None)
    };

    let mut selector = Selector {
        kind: match selector_token {
            "@s" => SelectorKind::Sender,
            "@p" => SelectorKind::NearestPlayer,
            "@r" => SelectorKind::RandomPlayer,
            "@a" => SelectorKind::AllPlayers,
            "@e" => SelectorKind::AllEntities,
            _ => {
                return Err(SelectorError::InvalidSelector(format!(
                    "Unsupported selector: {selector_token}"
                )));
            }
        },
        name: None,
        entity_type: None,
        x: None,
        y: None,
        z: None,
        dx: None,
        dy: None,
        dz: None,
        r: None,
        rm: None,
        c: None,
        gamemode: None,
    };

    if let Some(filter_blob) = filter_blob {
        if filter_blob.trim().is_empty() {
            return Ok(Some(selector));
        }

        for filter in filter_blob.split(',') {
            let (key, value) = filter.split_once('=').ok_or_else(|| {
                SelectorError::InvalidSelector(format!("Invalid filter: {filter}"))
            })?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" => selector.name = Some(value.to_string()),
                "type" => selector.entity_type = Some(value.to_ascii_lowercase()),
                "x" => selector.x = Some(parse_float(key, value)?),
                "y" => selector.y = Some(parse_float(key, value)?),
                "z" => selector.z = Some(parse_float(key, value)?),
                "dx" => selector.dx = Some(parse_float(key, value)?),
                "dy" => selector.dy = Some(parse_float(key, value)?),
                "dz" => selector.dz = Some(parse_float(key, value)?),
                "r" => selector.r = Some(parse_float(key, value)?),
                "rm" => selector.rm = Some(parse_float(key, value)?),
                "c" => {
                    let count = value.parse::<i32>().map_err(|_| {
                        SelectorError::InvalidSelector(format!("Invalid c value: {value}"))
                    })?;
                    if count <= 0 {
                        return Err(SelectorError::InvalidSelector(
                            "Selector count must be greater than zero".to_string(),
                        ));
                    }
                    selector.c = Some(count as usize);
                }
                "m" => {
                    selector.gamemode = Some(value.parse::<i32>().map_err(|_| {
                        SelectorError::InvalidSelector(format!("Invalid gamemode filter: {value}"))
                    })?);
                }
                other => {
                    return Err(SelectorError::UnsupportedSelector(format!(
                        "Unsupported selector filter: {other}"
                    )));
                }
            }
        }
    }

    Ok(Some(selector))
}

fn parse_float(key: &str, value: &str) -> Result<f32, SelectorError> {
    value
        .parse::<f32>()
        .map_err(|_| SelectorError::InvalidSelector(format!("Invalid {key} value: {value}")))
}

pub fn resolve_targets(
    selector: &Selector,
    sender: &dyn CommandSender,
    candidates: &[SelectorEntity],
) -> Result<Vec<SelectorEntity>, SelectorError> {
    resolve_targets_with_index(selector, sender, candidates, 0)
}

pub fn resolve_targets_with_seed(
    selector: &Selector,
    sender: &dyn CommandSender,
    candidates: &[SelectorEntity],
    seed: u64,
) -> Result<Vec<SelectorEntity>, SelectorError> {
    resolve_targets_with_index(selector, sender, candidates, seed as usize)
}

pub fn resolve_targets_with_index(
    selector: &Selector,
    sender: &dyn CommandSender,
    candidates: &[SelectorEntity],
    random_index: usize,
) -> Result<Vec<SelectorEntity>, SelectorError> {
    let origin = selector.origin(sender);
    let sender_entity_id = sender.sender_entity_id();

    let mut filtered = candidates
        .iter()
        .filter(|entity| match selector.kind {
            SelectorKind::Sender => sender_entity_id == Some(entity.id),
            SelectorKind::NearestPlayer | SelectorKind::RandomPlayer | SelectorKind::AllPlayers => {
                entity.entity_type.eq_ignore_ascii_case("player")
            }
            SelectorKind::AllEntities => true,
        })
        .cloned()
        .collect::<Vec<_>>();

    if matches!(selector.kind, SelectorKind::Sender) && sender_entity_id.is_none() {
        return Err(SelectorError::SenderHasNoEntity);
    }

    filtered.retain(|entity| entity_matches(selector, entity, origin));

    if filtered.is_empty() {
        return Err(SelectorError::NoTargetsMatched);
    }

    match selector.kind {
        SelectorKind::NearestPlayer => filtered.sort_by(|left, right| {
            distance_sq(left.position, origin)
                .partial_cmp(&distance_sq(right.position, origin))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
        }),
        SelectorKind::RandomPlayer => {
            filtered.sort_by_key(|entity| entity.id);
            if !filtered.is_empty() {
                let len = filtered.len();
                filtered.rotate_left(random_index % len);
            }
        }
        SelectorKind::AllPlayers | SelectorKind::AllEntities | SelectorKind::Sender => {
            filtered.sort_by_key(|entity| entity.id);
        }
    }

    if let Some(count) = selector.c {
        filtered.truncate(count);
    } else if matches!(
        selector.kind,
        SelectorKind::NearestPlayer | SelectorKind::RandomPlayer
    ) {
        filtered.truncate(1);
    }

    Ok(filtered)
}

pub fn resolve_target_token(
    token: &str,
    sender: &dyn CommandSender,
    candidates: &[SelectorEntity],
) -> Result<Vec<SelectorEntity>, SelectorError> {
    resolve_target_token_with_index(token, sender, candidates, 0)
}

pub fn resolve_target_token_with_index(
    token: &str,
    sender: &dyn CommandSender,
    candidates: &[SelectorEntity],
    random_index: usize,
) -> Result<Vec<SelectorEntity>, SelectorError> {
    if let Some(selector) = parse_selector(token)? {
        return resolve_targets_with_index(&selector, sender, candidates, random_index);
    }

    let token_lower = token.to_ascii_lowercase();
    let mut exact = candidates
        .iter()
        .filter(|entity| {
            entity
                .name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(&token_lower))
        })
        .cloned()
        .collect::<Vec<_>>();
    if exact.is_empty() {
        exact = candidates
            .iter()
            .filter(|entity| {
                entity
                    .name
                    .as_deref()
                    .is_some_and(|name| name.to_ascii_lowercase().starts_with(&token_lower))
            })
            .cloned()
            .collect::<Vec<_>>();
    }
    exact.sort_by_key(|entity| entity.id);
    if exact.is_empty() {
        Err(SelectorError::InvalidTarget(token.to_string()))
    } else {
        Ok(exact)
    }
}

fn entity_matches(selector: &Selector, entity: &SelectorEntity, origin: [f32; 3]) -> bool {
    if let Some(name) = &selector.name {
        let Some(entity_name) = entity.name.as_deref() else {
            return false;
        };
        if !entity_name.eq_ignore_ascii_case(name) {
            return false;
        }
    }

    if let Some(entity_type) = &selector.entity_type {
        if !entity.entity_type.eq_ignore_ascii_case(entity_type) {
            return false;
        }
    }

    if let Some(gamemode) = selector.gamemode {
        if entity.gamemode != Some(gamemode) {
            return false;
        }
    }

    let distance_sq = distance_sq(entity.position, origin);
    if let Some(max_radius) = selector.r {
        if distance_sq > max_radius * max_radius {
            return false;
        }
    }
    if let Some(min_radius) = selector.rm {
        if distance_sq < min_radius * min_radius {
            return false;
        }
    }

    if selector.dx.is_some() || selector.dy.is_some() || selector.dz.is_some() {
        let dx = selector.dx.unwrap_or(0.0);
        let dy = selector.dy.unwrap_or(0.0);
        let dz = selector.dz.unwrap_or(0.0);
        let min_x = origin[0].min(origin[0] + dx);
        let max_x = origin[0].max(origin[0] + dx);
        let min_y = origin[1].min(origin[1] + dy);
        let max_y = origin[1].max(origin[1] + dy);
        let min_z = origin[2].min(origin[2] + dz);
        let max_z = origin[2].max(origin[2] + dz);
        if entity.position[0] < min_x
            || entity.position[0] > max_x
            || entity.position[1] < min_y
            || entity.position[1] > max_y
            || entity.position[2] < min_z
            || entity.position[2] > max_z
        {
            return false;
        }
    }

    true
}

fn distance_sq(position: [f32; 3], origin: [f32; 3]) -> f32 {
    let dx = position[0] - origin[0];
    let dy = position[1] - origin[1];
    let dz = position[2] - origin[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandSender, SoftEnumSource};

    struct TestSender;

    impl CommandSender for TestSender {
        fn sender_name(&self) -> &str {
            "Tester"
        }

        fn sender_is_player(&self) -> bool {
            true
        }

        fn sender_position(&self) -> [f32; 3] {
            [0.0, 64.0, 0.0]
        }

        fn sender_entity_id(&self) -> Option<u64> {
            Some(1)
        }

        fn sender_is_op(&self) -> bool {
            true
        }

        fn sender_has_permission(&self, _permission: &str) -> bool {
            true
        }
    }

    impl SoftEnumSource for TestSender {
        fn soft_enum_values(&self, _name: &str) -> Vec<String> {
            Vec::new()
        }
    }

    fn candidates() -> Vec<SelectorEntity> {
        vec![
            SelectorEntity {
                id: 1,
                name: Some("Tester".to_string()),
                entity_type: "player".to_string(),
                position: [0.0, 64.0, 0.0],
                gamemode: Some(1),
            },
            SelectorEntity {
                id: 2,
                name: Some("Alex".to_string()),
                entity_type: "player".to_string(),
                position: [4.0, 64.0, 0.0],
                gamemode: Some(0),
            },
            SelectorEntity {
                id: 3,
                name: None,
                entity_type: "item".to_string(),
                position: [1.0, 64.0, 1.0],
                gamemode: None,
            },
        ]
    }

    #[test]
    fn parses_selector_filters() {
        let selector = parse_selector("@a[x=10,y=70,z=10,r=8,m=1]")
            .unwrap()
            .unwrap();
        assert_eq!(selector.kind, SelectorKind::AllPlayers);
        assert_eq!(selector.x, Some(10.0));
        assert_eq!(selector.gamemode, Some(1));
    }

    #[test]
    fn resolves_nearest_player() {
        let selector = parse_selector("@p").unwrap().unwrap();
        let resolved = resolve_targets(&selector, &TestSender, &candidates()).unwrap();
        assert_eq!(resolved[0].name.as_deref(), Some("Tester"));
    }

    #[test]
    fn resolves_all_entities_with_filters() {
        let selector = parse_selector("@e[type=item,r=4]").unwrap().unwrap();
        let resolved = resolve_targets(&selector, &TestSender, &candidates()).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].entity_type, "item");
    }

    #[test]
    fn resolves_plain_player_prefix() {
        let resolved = resolve_target_token("Al", &TestSender, &candidates()).unwrap();
        assert_eq!(resolved[0].name.as_deref(), Some("Alex"));
    }
}
