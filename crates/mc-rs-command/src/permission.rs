use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDefault {
    True,
    False,
    Op,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDefinition {
    pub default: PermissionDefault,
    pub children: HashMap<String, bool>,
}

impl PermissionDefinition {
    pub fn new(default: PermissionDefault) -> Self {
        Self {
            default,
            children: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionState {
    pub explicit: HashMap<String, bool>,
    pub is_op: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PermissionRegistry {
    definitions: HashMap<String, PermissionDefinition>,
}

impl PermissionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: impl Into<String>, definition: PermissionDefinition) {
        self.definitions
            .insert(name.into().to_ascii_lowercase(), definition);
    }

    pub fn set_default(
        &mut self,
        name: impl Into<String>,
        default: PermissionDefault,
    ) -> &mut PermissionDefinition {
        let key = name.into().to_ascii_lowercase();
        self.definitions
            .entry(key)
            .or_insert_with(|| PermissionDefinition::new(default))
    }

    pub fn definition(&self, name: &str) -> Option<&PermissionDefinition> {
        self.definitions.get(&name.to_ascii_lowercase())
    }

    pub fn has_permission(&self, state: &PermissionState, permission: &str) -> bool {
        let key = permission.to_ascii_lowercase();
        let effective = self.effective_permissions(state);
        effective
            .get(&key)
            .copied()
            .unwrap_or_else(|| self.default_grant(&key, state.is_op))
    }

    pub fn effective_permissions(&self, state: &PermissionState) -> HashMap<String, bool> {
        let mut effective = state
            .explicit
            .iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), *value))
            .collect::<HashMap<_, _>>();

        for (name, definition) in &self.definitions {
            effective
                .entry(name.clone())
                .or_insert_with(|| self.default_grant(name, state.is_op));
            if !state.explicit.contains_key(name)
                && matches!(definition.default, PermissionDefault::True)
            {
                effective.insert(name.clone(), true);
            }
            if !state.explicit.contains_key(name)
                && matches!(definition.default, PermissionDefault::Op)
                && state.is_op
            {
                effective.insert(name.clone(), true);
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            let granted = effective
                .iter()
                .filter_map(|(name, value)| value.then_some(name.clone()))
                .collect::<Vec<_>>();

            for permission in granted {
                if let Some(definition) = self.definitions.get(&permission) {
                    for (child, value) in &definition.children {
                        let child = child.to_ascii_lowercase();
                        let old = effective.insert(child, *value);
                        if old != Some(*value) {
                            changed = true;
                        }
                    }
                }
            }
        }

        effective
    }

    fn default_grant(&self, permission: &str, is_op: bool) -> bool {
        match self
            .definitions
            .get(permission)
            .map(|definition| definition.default)
            .unwrap_or(PermissionDefault::False)
        {
            PermissionDefault::True => true,
            PermissionDefault::False => false,
            PermissionDefault::Op => is_op,
        }
    }

    pub fn permission_names(&self) -> HashSet<String> {
        self.definitions.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_op_default_and_children() {
        let mut registry = PermissionRegistry::new();
        let mut admin = PermissionDefinition::new(PermissionDefault::Op);
        admin.children.insert("server.command.stop".into(), true);
        registry.register("server.admin", admin);
        registry.register(
            "server.command.stop",
            PermissionDefinition::new(PermissionDefault::False),
        );

        let op_state = PermissionState {
            explicit: HashMap::new(),
            is_op: true,
        };
        assert!(registry.has_permission(&op_state, "server.admin"));
        assert!(registry.has_permission(&op_state, "server.command.stop"));
    }

    #[test]
    fn explicit_values_override_defaults() {
        let mut registry = PermissionRegistry::new();
        registry.register(
            "server.command.help",
            PermissionDefinition::new(PermissionDefault::True),
        );
        let mut state = PermissionState::default();
        state
            .explicit
            .insert("server.command.help".to_string(), false);
        assert!(!registry.has_permission(&state, "server.command.help"));
    }
}
