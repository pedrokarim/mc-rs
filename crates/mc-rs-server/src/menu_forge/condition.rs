//! Évaluation des conditions du format, côté serveur : mêmes règles que le
//! studio (`model/conditions.ts`) et la lib Java (`ConditionContext`).

use std::collections::{BTreeMap, BTreeSet};

use super::catalog::{Condition, StateValue};

/// État et drapeaux d’un menu au moment de l’évaluation.
pub struct ConditionContext<'a> {
    pub state: &'a BTreeMap<String, StateValue>,
    pub flags: &'a BTreeSet<String>,
}

/// Une condition absente est toujours vraie.
pub fn evaluate(condition: Option<&Condition>, context: &ConditionContext<'_>) -> bool {
    let Some(condition) = condition else {
        return true;
    };
    match condition {
        Condition::All { all } => all.iter().all(|child| evaluate(Some(child), context)),
        Condition::Any { any } => any.iter().any(|child| evaluate(Some(child), context)),
        Condition::Not { not } => !evaluate(Some(not), context),
        Condition::Flag { flag } => context.flags.contains(flag),
        Condition::Is { state, is } => context.state.get(state) == Some(is),
        Condition::In { state, values } => context
            .state
            .get(state)
            .is_some_and(|value| values.contains(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition(json: serde_json::Value) -> Condition {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn evaluates_like_the_studio() {
        let state = BTreeMap::from([
            ("tab".to_string(), StateValue::Text("b".into())),
            ("count".to_string(), StateValue::Number(2.0)),
        ]);
        let flags = BTreeSet::from(["viewer.op".to_string()]);
        let context = ConditionContext {
            state: &state,
            flags: &flags,
        };
        let check = |json: serde_json::Value| evaluate(Some(&condition(json)), &context);

        assert!(evaluate(None, &context));
        assert!(check(serde_json::json!({"state": "tab", "is": "b"})));
        assert!(!check(serde_json::json!({"state": "tab", "is": "a"})));
        assert!(check(serde_json::json!({"state": "count", "is": 2})));
        assert!(check(serde_json::json!({"state": "tab", "in": ["a", "b"]})));
        assert!(!check(serde_json::json!({"state": "missing", "in": ["a"]})));
        assert!(check(serde_json::json!({"flag": "viewer.op"})));
        assert!(!check(serde_json::json!({"flag": "page.hasNext"})));
        assert!(check(
            serde_json::json!({"all": [{"flag": "viewer.op"}, {"not": {"state": "tab", "is": "a"}}]})
        ));
        assert!(!check(
            serde_json::json!({"any": [{"flag": "x"}, {"state": "tab", "is": "a"}]})
        ));
        assert!(check(serde_json::json!({"all": []})));
        assert!(!check(serde_json::json!({"any": []})));
    }
}
