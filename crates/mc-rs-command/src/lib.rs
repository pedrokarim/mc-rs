mod helpers;
mod map;
mod parser;
mod permission;
mod selector;

pub use helpers::{
    hard_enum_param, message, param, parse_coord, parse_position_triplet,
    parse_position_triplet_for_source, soft_enum_param, usage,
};
pub use map::{
    CommandDefinition, CommandDispatchError, CommandInvocation, CommandMap, CommandOverload,
    CommandParameter, ParamType, RegistrationError, VisibleCommand, VisibleCommandOverload,
    VisibleCommandParameter, VisibleParamType,
};
pub use parser::{parse_command_line, tokenize_command_line, CommandLineParseError};
pub use permission::{
    PermissionDefault, PermissionDefinition, PermissionRegistry, PermissionState,
};
pub use selector::{
    parse_selector, resolve_target_token, resolve_target_token_with_index, resolve_targets,
    resolve_targets_with_index, resolve_targets_with_seed, Selector, SelectorEntity, SelectorError,
    SelectorKind,
};

/// Minimal information the command system needs from the current sender.
pub trait CommandSender {
    fn sender_name(&self) -> &str;
    fn sender_is_player(&self) -> bool;
    fn sender_position(&self) -> [f32; 3];
    fn sender_entity_id(&self) -> Option<u64>;
    fn sender_is_op(&self) -> bool;
    fn sender_has_permission(&self, permission: &str) -> bool;
}

/// Dynamic values for Bedrock soft enums.
pub trait SoftEnumSource {
    fn soft_enum_values(&self, name: &str) -> Vec<String>;
}
