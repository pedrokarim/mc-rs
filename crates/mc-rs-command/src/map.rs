use std::collections::HashMap;
use std::fmt;

use tracing::info;

use crate::parser::{parse_command_line, CommandLineParseError};
use crate::{CommandSender, SoftEnumSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
    Int,
    Float,
    String,
    Target,
    Position,
    Message,
    RawText,
    Json,
    Command,
    HardEnum { name: String, values: Vec<String> },
    SoftEnum { name: String },
}

impl ParamType {
    pub fn type_id(&self) -> Option<u32> {
        match self {
            ParamType::Int => Some(1),
            ParamType::Float => Some(3),
            ParamType::String => Some(56),
            ParamType::Target => Some(8),
            ParamType::Position => Some(65),
            ParamType::Message => Some(68),
            ParamType::RawText => Some(70),
            ParamType::Json => Some(74),
            ParamType::Command => Some(87),
            ParamType::HardEnum { .. } | ParamType::SoftEnum { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandParameter {
    pub name: String,
    pub param_type: ParamType,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandOverload {
    pub parameters: Vec<CommandParameter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDefinition {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub aliases: Vec<String>,
    pub permissions: Vec<String>,
    pub permission_message: Option<String>,
    pub overloads: Vec<CommandOverload>,
    pub owner: Option<String>,
}

impl CommandDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            usage: String::new(),
            aliases: Vec::new(),
            permissions: Vec::new(),
            permission_message: None,
            overloads: Vec::new(),
            owner: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    pub original: String,
    pub label: String,
    pub command_name: String,
    pub args: Vec<String>,
    pub raw_args: String,
}

impl CommandInvocation {
    pub fn arg(&self, index: usize) -> Option<&str> {
        self.args.get(index).map(String::as_str)
    }

    pub fn tail(&self, from: usize) -> String {
        if from >= self.args.len() {
            String::new()
        } else {
            self.args[from..].join(" ")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandDispatchError {
    Parse(CommandLineParseError),
    NotFound(String),
    PermissionDenied(String),
    Usage(String),
    Message(String),
}

impl fmt::Display for CommandDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandDispatchError::Parse(error) => write!(f, "{error}"),
            CommandDispatchError::NotFound(label) => {
                write!(f, "Unknown command: {label}. Type /help for help.")
            }
            CommandDispatchError::PermissionDenied(message)
            | CommandDispatchError::Usage(message)
            | CommandDispatchError::Message(message) => write!(f, "{message}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationError {
    DuplicateName(String),
    EmptyName,
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistrationError::DuplicateName(name) => {
                write!(f, "Command or alias is already registered: {name}")
            }
            RegistrationError::EmptyName => write!(f, "Command names cannot be empty"),
        }
    }
}

type CommandHandlerFn<R> = dyn for<'r, 'a> Fn(&'r mut R, &'a CommandInvocation) -> Result<(), CommandDispatchError>
    + Send
    + Sync;

struct CommandEntry<R: ?Sized> {
    definition: CommandDefinition,
    handler: Box<CommandHandlerFn<R>>,
}

pub struct CommandMap<R: ?Sized> {
    commands: Vec<CommandEntry<R>>,
    name_to_index: HashMap<String, usize>,
}

impl<R: CommandSender + SoftEnumSource + ?Sized> Default for CommandMap<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: CommandSender + SoftEnumSource + ?Sized> CommandMap<R> {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            name_to_index: HashMap::new(),
        }
    }

    pub fn register<H>(
        &mut self,
        mut definition: CommandDefinition,
        handler: H,
    ) -> Result<(), RegistrationError>
    where
        H: for<'r, 'a> Fn(&'r mut R, &'a CommandInvocation) -> Result<(), CommandDispatchError>
            + Send
            + Sync
            + 'static,
    {
        let name = normalize_label(&definition.name);
        if name.is_empty() {
            return Err(RegistrationError::EmptyName);
        }
        if self.name_to_index.contains_key(&name) {
            return Err(RegistrationError::DuplicateName(name));
        }

        definition.name = name.clone();
        definition.aliases = definition
            .aliases
            .into_iter()
            .map(|alias| normalize_label(&alias))
            .collect::<Vec<_>>();
        definition.permissions = definition
            .permissions
            .into_iter()
            .map(|permission| permission.to_ascii_lowercase())
            .collect();

        let index = self.commands.len();
        self.name_to_index.insert(name, index);

        for alias in &definition.aliases {
            if alias.is_empty() {
                return Err(RegistrationError::EmptyName);
            }
            if self.name_to_index.contains_key(alias) {
                return Err(RegistrationError::DuplicateName(alias.clone()));
            }
            self.name_to_index.insert(alias.clone(), index);
        }

        self.commands.push(CommandEntry {
            definition,
            handler: Box::new(handler),
        });
        Ok(())
    }

    pub fn unregister(&mut self, name: &str) -> Option<CommandDefinition> {
        let canonical = normalize_label(name);
        let index = *self.name_to_index.get(&canonical)?;
        let removed = self.commands.remove(index);
        self.rebuild_name_index();
        Some(removed.definition)
    }

    pub fn definitions(&self) -> impl Iterator<Item = &CommandDefinition> {
        self.commands.iter().map(|entry| &entry.definition)
    }

    pub fn definition(&self, name: &str) -> Option<&CommandDefinition> {
        let index = self.name_to_index.get(&normalize_label(name)).copied()?;
        self.commands.get(index).map(|entry| &entry.definition)
    }

    pub fn dispatch(
        &self,
        runtime: &mut R,
        command_line: &str,
    ) -> Result<(), CommandDispatchError> {
        let parsed = parse_command_line(command_line).map_err(CommandDispatchError::Parse)?;
        let Some(index) = self.name_to_index.get(&parsed.label).copied() else {
            return Err(CommandDispatchError::NotFound(parsed.label));
        };
        let entry = &self.commands[index];

        if !entry.definition.permissions.is_empty()
            && !entry
                .definition
                .permissions
                .iter()
                .any(|permission| runtime.sender_has_permission(permission))
        {
            return Err(CommandDispatchError::PermissionDenied(
                entry
                    .definition
                    .permission_message
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "You do not have permission to use /{}.",
                            entry.definition.name
                        )
                    }),
            ));
        }

        info!(
            "[CMD] {} executed /{} {}",
            runtime.sender_name(),
            entry.definition.name,
            parsed.raw_args
        );

        let invocation = CommandInvocation {
            original: parsed.original,
            label: parsed.label,
            command_name: entry.definition.name.clone(),
            args: parsed.args,
            raw_args: parsed.raw_args,
        };
        (entry.handler)(runtime, &invocation)
    }

    pub fn build_visible_commands<S>(&self, sender: &S) -> Vec<VisibleCommand>
    where
        S: CommandSender + SoftEnumSource + ?Sized,
    {
        self.commands
            .iter()
            .filter(|entry| {
                entry.definition.permissions.is_empty()
                    || entry
                        .definition
                        .permissions
                        .iter()
                        .any(|permission| sender.sender_has_permission(permission))
            })
            .map(|entry| VisibleCommand {
                name: entry.definition.name.clone(),
                description: entry.definition.description.clone(),
                aliases: entry.definition.aliases.clone(),
                overloads: entry
                    .definition
                    .overloads
                    .iter()
                    .map(|overload| VisibleCommandOverload {
                        parameters: overload
                            .parameters
                            .iter()
                            .map(|parameter| VisibleCommandParameter {
                                name: parameter.name.clone(),
                                param_type: match &parameter.param_type {
                                    ParamType::Int => VisibleParamType::Basic(1),
                                    ParamType::Float => VisibleParamType::Basic(3),
                                    ParamType::String => VisibleParamType::Basic(56),
                                    ParamType::Target => VisibleParamType::Basic(8),
                                    ParamType::Position => VisibleParamType::Basic(65),
                                    ParamType::Message => VisibleParamType::Basic(68),
                                    ParamType::RawText => VisibleParamType::Basic(70),
                                    ParamType::Json => VisibleParamType::Basic(74),
                                    ParamType::Command => VisibleParamType::Basic(87),
                                    ParamType::HardEnum { name, values } => {
                                        VisibleParamType::HardEnum {
                                            name: name.clone(),
                                            values: values.clone(),
                                        }
                                    }
                                    ParamType::SoftEnum { name } => VisibleParamType::SoftEnum {
                                        name: name.clone(),
                                        values: sender.soft_enum_values(name),
                                    },
                                },
                                optional: parameter.optional,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    fn rebuild_name_index(&mut self) {
        self.name_to_index.clear();
        for (index, entry) in self.commands.iter().enumerate() {
            self.name_to_index
                .insert(entry.definition.name.clone(), index);
            for alias in &entry.definition.aliases {
                self.name_to_index.insert(alias.clone(), index);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleCommand {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub overloads: Vec<VisibleCommandOverload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleCommandOverload {
    pub parameters: Vec<VisibleCommandParameter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleCommandParameter {
    pub name: String,
    pub param_type: VisibleParamType,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibleParamType {
    Basic(u32),
    HardEnum { name: String, values: Vec<String> },
    SoftEnum { name: String, values: Vec<String> },
}

fn normalize_label(label: &str) -> String {
    label.trim().trim_start_matches('/').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandSender, SoftEnumSource};

    #[derive(Default)]
    struct TestRuntime {
        log: Vec<String>,
        allow_admin: bool,
    }

    impl CommandSender for TestRuntime {
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
            self.allow_admin
        }

        fn sender_has_permission(&self, permission: &str) -> bool {
            permission == "server.command.help" || self.allow_admin
        }
    }

    impl SoftEnumSource for TestRuntime {
        fn soft_enum_values(&self, name: &str) -> Vec<String> {
            if name == "online_players" {
                vec!["Tester".to_string(), "Alex".to_string()]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn dispatches_alias_without_leading_slash() {
        let mut map = CommandMap::<TestRuntime>::new();
        let mut def = CommandDefinition::new("help", "Show help");
        def.aliases = vec!["?".into()];
        map.register(
            def,
            |runtime: &mut TestRuntime, invocation: &CommandInvocation| {
                runtime.log.push(invocation.command_name.clone());
                Ok(())
            },
        )
        .unwrap();

        let mut runtime = TestRuntime::default();
        map.dispatch(&mut runtime, "?").unwrap();
        assert_eq!(runtime.log, vec!["help".to_string()]);
    }

    #[test]
    fn filters_visible_commands_by_permission() {
        let mut map = CommandMap::<TestRuntime>::new();
        let mut help = CommandDefinition::new("help", "Show help");
        help.permissions = vec!["server.command.help".into()];
        map.register(help, |_: &mut TestRuntime, _: &CommandInvocation| Ok(()))
            .unwrap();

        let mut stop = CommandDefinition::new("stop", "Stop");
        stop.permissions = vec!["server.command.stop".into()];
        map.register(stop, |_: &mut TestRuntime, _: &CommandInvocation| Ok(()))
            .unwrap();

        let visible = map.build_visible_commands(&TestRuntime::default());
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "help");
    }

    #[test]
    fn resolves_soft_enum_values() {
        let mut map = CommandMap::<TestRuntime>::new();
        let mut tell = CommandDefinition::new("tell", "Private message");
        tell.overloads.push(CommandOverload {
            parameters: vec![CommandParameter {
                name: "target".into(),
                param_type: ParamType::SoftEnum {
                    name: "online_players".into(),
                },
                optional: false,
            }],
        });
        map.register(tell, |_: &mut TestRuntime, _: &CommandInvocation| Ok(()))
            .unwrap();

        let visible = map.build_visible_commands(&TestRuntime::default());
        assert!(matches!(
            visible[0].overloads[0].parameters[0].param_type,
            VisibleParamType::SoftEnum { .. }
        ));
    }
}
