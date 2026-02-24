//! Command parsing, registry, and built-in commands.

pub mod selector;

use std::collections::HashMap;

/// Context passed to a command handler.
pub struct CommandContext {
    /// Name of the player executing the command.
    pub sender_name: String,
    /// Arguments after the command name.
    pub args: Vec<String>,
}

/// Result returned by a command handler.
pub struct CommandResult {
    /// Whether the command executed successfully.
    pub success: bool,
    /// Messages to send back to the command sender.
    pub messages: Vec<String>,
    /// Optional message to broadcast to all players.
    pub broadcast: Option<String>,
    /// If true, the server should shut down.
    pub should_stop: bool,
}

impl CommandResult {
    /// Create a successful result with a single message.
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            messages: vec![message.into()],
            broadcast: None,
            should_stop: false,
        }
    }

    /// Create a failed result with a single message.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            messages: vec![message.into()],
            broadcast: None,
            should_stop: false,
        }
    }
}

/// Function pointer type for command handlers.
pub type CommandFn = fn(&CommandContext) -> CommandResult;

/// A registered command.
pub struct CommandEntry {
    pub name: String,
    pub description: String,
    pub handler: CommandFn,
}

/// Registry of available server commands.
pub struct CommandRegistry {
    commands: HashMap<String, CommandEntry>,
}

impl CommandRegistry {
    /// Create a new registry with the 4 built-in commands.
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        registry.register("help", "List available commands", cmd_help);
        registry.register("list", "Show online players", cmd_list);
        registry.register("say", "Broadcast a message to all players", cmd_say);
        registry.register("stop", "Stop the server", cmd_stop);
        registry
    }

    /// Register a command for autocomplete only (no-op handler).
    /// Used for server commands handled directly in connection.rs.
    pub fn register_stub(&mut self, name: &str, description: &str) {
        self.register(name, description, |_| {
            CommandResult::err("This command is handled internally.")
        });
    }

    /// Register a command.
    fn register(&mut self, name: &str, description: &str, handler: CommandFn) {
        self.commands.insert(
            name.to_string(),
            CommandEntry {
                name: name.to_string(),
                description: description.to_string(),
                handler,
            },
        );
    }

    /// Execute a command by name.
    pub fn execute(&self, name: &str, ctx: &CommandContext) -> CommandResult {
        match self.commands.get(name) {
            Some(entry) => (entry.handler)(ctx),
            None => CommandResult::err(format!(
                "Unknown command: {name}. Type /help for a list of commands."
            )),
        }
    }

    /// Get a reference to all registered commands.
    pub fn get_commands(&self) -> &HashMap<String, CommandEntry> {
        &self.commands
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Built-in commands
// ---------------------------------------------------------------------------

fn cmd_help(ctx: &CommandContext) -> CommandResult {
    // The help command receives the command list via args as "name:description" pairs.
    // This is injected by the server before calling execute.
    let mut lines = vec!["Available commands:".to_string()];
    for arg in &ctx.args {
        if let Some((name, desc)) = arg.split_once(':') {
            lines.push(format!("  /{name} - {desc}"));
        }
    }
    CommandResult {
        success: true,
        messages: lines,
        broadcast: None,
        should_stop: false,
    }
}

fn cmd_list(ctx: &CommandContext) -> CommandResult {
    // args contains the list of online player names (injected by the server).
    let count = ctx.args.len();
    let names = if ctx.args.is_empty() {
        String::new()
    } else {
        format!(": {}", ctx.args.join(", "))
    };
    CommandResult::ok(format!(
        "There {verb} {count} player{s} online{names}",
        verb = if count == 1 { "is" } else { "are" },
        s = if count == 1 { "" } else { "s" },
    ))
}

fn cmd_say(ctx: &CommandContext) -> CommandResult {
    if ctx.args.is_empty() {
        return CommandResult::err("Usage: /say <message>");
    }
    let message = ctx.args.join(" ");
    CommandResult {
        success: true,
        messages: vec![],
        broadcast: Some(format!("[{}] {}", ctx.sender_name, message)),
        should_stop: false,
    }
}

fn cmd_stop(_ctx: &CommandContext) -> CommandResult {
    CommandResult {
        success: true,
        messages: vec!["Stopping the server...".to_string()],
        broadcast: None,
        should_stop: true,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(sender: &str, args: Vec<&str>) -> CommandContext {
        CommandContext {
            sender_name: sender.to_string(),
            args: args.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn registry_has_4_builtins() {
        let reg = CommandRegistry::new();
        assert_eq!(reg.get_commands().len(), 4);
        assert!(reg.get_commands().contains_key("help"));
        assert!(reg.get_commands().contains_key("list"));
        assert!(reg.get_commands().contains_key("say"));
        assert!(reg.get_commands().contains_key("stop"));
    }

    #[test]
    fn unknown_command() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx("Steve", vec![]);
        let result = reg.execute("teleport", &ctx);
        assert!(!result.success);
        assert!(result.messages[0].contains("Unknown command"));
    }

    #[test]
    fn help_lists_commands() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx(
            "Steve",
            vec![
                "help:List available commands",
                "say:Broadcast a message to all players",
            ],
        );
        let result = reg.execute("help", &ctx);
        assert!(result.success);
        assert!(result.messages.len() >= 2);
        assert!(result.messages[0].contains("Available commands"));
    }

    #[test]
    fn list_empty() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx("Steve", vec![]);
        let result = reg.execute("list", &ctx);
        assert!(result.success);
        assert!(result.messages[0].contains("0 players online"));
    }

    #[test]
    fn list_with_players() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx("Steve", vec!["Steve", "Alex"]);
        let result = reg.execute("list", &ctx);
        assert!(result.success);
        assert!(result.messages[0].contains("2 players"));
        assert!(result.messages[0].contains("Steve"));
        assert!(result.messages[0].contains("Alex"));
    }

    #[test]
    fn list_single_player() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx("Steve", vec!["Steve"]);
        let result = reg.execute("list", &ctx);
        assert!(result.success);
        assert!(result.messages[0].contains("1 player online"));
        assert!(!result.messages[0].contains("players")); // singular
    }

    #[test]
    fn say_broadcasts() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx("Steve", vec!["hello", "world"]);
        let result = reg.execute("say", &ctx);
        assert!(result.success);
        assert_eq!(result.broadcast, Some("[Steve] hello world".to_string()));
    }

    #[test]
    fn say_empty_fails() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx("Steve", vec![]);
        let result = reg.execute("say", &ctx);
        assert!(!result.success);
        assert!(result.messages[0].contains("Usage"));
    }

    #[test]
    fn stop_flags_shutdown() {
        let reg = CommandRegistry::new();
        let ctx = make_ctx("Steve", vec![]);
        let result = reg.execute("stop", &ctx);
        assert!(result.success);
        assert!(result.should_stop);
    }

    #[test]
    fn register_stub_adds_command() {
        let mut reg = CommandRegistry::new();
        reg.register_stub("test_cmd", "A test command");
        assert!(reg.get_commands().contains_key("test_cmd"));
        // Stub handler returns error
        let ctx = make_ctx("Steve", vec![]);
        let result = reg.execute("test_cmd", &ctx);
        assert!(!result.success);
    }

    #[test]
    fn result_helpers() {
        let ok = CommandResult::ok("success");
        assert!(ok.success);
        assert_eq!(ok.messages[0], "success");

        let err = CommandResult::err("failed");
        assert!(!err.success);
        assert_eq!(err.messages[0], "failed");
    }
}
