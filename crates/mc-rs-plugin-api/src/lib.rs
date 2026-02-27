//! Plugin API: traits, events, and server API for plugin authors.
//!
//! This crate defines the interface that plugin runtimes (Lua, WASM) implement.
//! It has no dependency on mc-rs-server or mc-rs-proto.

// ─── Types ───────────────────────────────────────────────────────────────────

/// Information about an online player, passed to plugins in events.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginPlayer {
    pub name: String,
    pub uuid: String,
    pub runtime_id: u64,
    pub position: (f32, f32, f32),
    pub gamemode: i32,
    pub health: f32,
}

/// Block position for plugin events (decoupled from mc-rs-proto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginBlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Cause of damage for PlayerDamage events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DamageCause {
    Attack,
    Fall,
    Drowning,
    Lava,
    Fire,
    Suffocation,
    Starvation,
    Void,
    Other,
}

/// Log level for plugin logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// Result of dispatching an event to a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventResult {
    /// Continue normal handling.
    Continue,
    /// Event was cancelled by this plugin.
    Cancelled,
}

// ─── Forms ───────────────────────────────────────────────────────────────────

/// Response from a form displayed to a player.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FormResponse {
    /// The player closed the form without responding.
    Cancelled,
    /// The player clicked a button on a SimpleForm.
    Simple { button_index: u32 },
    /// The player responded to a ModalForm (true = button1, false = button2).
    Modal { accepted: bool },
    /// The player submitted a CustomForm with an array of values.
    Custom { values: Vec<serde_json::Value> },
}

/// An image for a form button.
#[derive(Debug, Clone)]
pub struct FormImage {
    /// Image type: "path" (resource pack) or "url" (web URL).
    pub image_type: String,
    /// Image path or URL.
    pub data: String,
}

/// A button in a SimpleForm.
#[derive(Debug, Clone)]
pub struct FormButton {
    pub text: String,
    pub image: Option<FormImage>,
}

/// An element in a CustomForm.
#[derive(Debug, Clone)]
pub enum FormElement {
    Label {
        text: String,
    },
    Input {
        text: String,
        placeholder: String,
        default: String,
    },
    Toggle {
        text: String,
        default: bool,
    },
    Dropdown {
        text: String,
        options: Vec<String>,
        default: usize,
    },
    Slider {
        text: String,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    StepSlider {
        text: String,
        steps: Vec<String>,
        default: usize,
    },
}

/// Builder for a SimpleForm (list of buttons).
pub struct SimpleFormBuilder {
    title: String,
    content: String,
    buttons: Vec<FormButton>,
}

impl SimpleFormBuilder {
    pub fn new(title: &str, content: &str) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
            buttons: Vec::new(),
        }
    }

    pub fn button(mut self, text: &str) -> Self {
        self.buttons.push(FormButton {
            text: text.to_string(),
            image: None,
        });
        self
    }

    pub fn button_with_image(mut self, text: &str, image_type: &str, image_data: &str) -> Self {
        self.buttons.push(FormButton {
            text: text.to_string(),
            image: Some(FormImage {
                image_type: image_type.to_string(),
                data: image_data.to_string(),
            }),
        });
        self
    }

    pub fn to_json(&self) -> String {
        let buttons: Vec<serde_json::Value> = self
            .buttons
            .iter()
            .map(|b| {
                let mut btn = serde_json::json!({ "text": b.text });
                if let Some(img) = &b.image {
                    btn["image"] = serde_json::json!({
                        "type": img.image_type,
                        "data": img.data,
                    });
                }
                btn
            })
            .collect();
        serde_json::json!({
            "type": "form",
            "title": self.title,
            "content": self.content,
            "buttons": buttons,
        })
        .to_string()
    }
}

/// Builder for a ModalForm (two-button yes/no dialog).
pub struct ModalFormBuilder {
    title: String,
    content: String,
    button1: String,
    button2: String,
}

impl ModalFormBuilder {
    pub fn new(title: &str, content: &str, button1: &str, button2: &str) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
            button1: button1.to_string(),
            button2: button2.to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({
            "type": "modal",
            "title": self.title,
            "content": self.content,
            "button1": self.button1,
            "button2": self.button2,
        })
        .to_string()
    }
}

/// Builder for a CustomForm (complex form with various input elements).
pub struct CustomFormBuilder {
    title: String,
    elements: Vec<FormElement>,
}

impl CustomFormBuilder {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            elements: Vec::new(),
        }
    }

    pub fn label(mut self, text: &str) -> Self {
        self.elements.push(FormElement::Label {
            text: text.to_string(),
        });
        self
    }

    pub fn input(mut self, text: &str, placeholder: &str, default: &str) -> Self {
        self.elements.push(FormElement::Input {
            text: text.to_string(),
            placeholder: placeholder.to_string(),
            default: default.to_string(),
        });
        self
    }

    pub fn toggle(mut self, text: &str, default: bool) -> Self {
        self.elements.push(FormElement::Toggle {
            text: text.to_string(),
            default,
        });
        self
    }

    pub fn dropdown(mut self, text: &str, options: Vec<String>, default: usize) -> Self {
        self.elements.push(FormElement::Dropdown {
            text: text.to_string(),
            options,
            default,
        });
        self
    }

    pub fn slider(mut self, text: &str, min: f64, max: f64, step: f64, default: f64) -> Self {
        self.elements.push(FormElement::Slider {
            text: text.to_string(),
            min,
            max,
            step,
            default,
        });
        self
    }

    pub fn step_slider(mut self, text: &str, steps: Vec<String>, default: usize) -> Self {
        self.elements.push(FormElement::StepSlider {
            text: text.to_string(),
            steps,
            default,
        });
        self
    }

    pub fn to_json(&self) -> String {
        let content: Vec<serde_json::Value> = self
            .elements
            .iter()
            .map(|e| match e {
                FormElement::Label { text } => serde_json::json!({
                    "type": "label",
                    "text": text,
                }),
                FormElement::Input {
                    text,
                    placeholder,
                    default,
                } => serde_json::json!({
                    "type": "input",
                    "text": text,
                    "placeholder": placeholder,
                    "default": default,
                }),
                FormElement::Toggle { text, default } => serde_json::json!({
                    "type": "toggle",
                    "text": text,
                    "default": default,
                }),
                FormElement::Dropdown {
                    text,
                    options,
                    default,
                } => serde_json::json!({
                    "type": "dropdown",
                    "text": text,
                    "options": options,
                    "default": default,
                }),
                FormElement::Slider {
                    text,
                    min,
                    max,
                    step,
                    default,
                } => serde_json::json!({
                    "type": "slider",
                    "text": text,
                    "min": min,
                    "max": max,
                    "step": step,
                    "default": default,
                }),
                FormElement::StepSlider {
                    text,
                    steps,
                    default,
                } => serde_json::json!({
                    "type": "step_slider",
                    "text": text,
                    "steps": steps,
                    "default": default,
                }),
            })
            .collect();
        serde_json::json!({
            "type": "custom_form",
            "title": self.title,
            "content": content,
        })
        .to_string()
    }
}

/// Parse a form response string based on form type.
pub fn parse_form_response(form_type: &str, response_data: Option<&str>) -> FormResponse {
    let data = match response_data {
        Some(d) => d,
        None => return FormResponse::Cancelled,
    };

    match form_type {
        "simple" => {
            if let Ok(idx) = data.trim().parse::<u32>() {
                FormResponse::Simple { button_index: idx }
            } else {
                FormResponse::Cancelled
            }
        }
        "modal" => {
            let accepted = data.trim() == "true";
            FormResponse::Modal { accepted }
        }
        "custom" => {
            if let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(data) {
                FormResponse::Custom { values }
            } else {
                FormResponse::Cancelled
            }
        }
        _ => FormResponse::Cancelled,
    }
}

// ─── Events ──────────────────────────────────────────────────────────────────

/// All events that plugins can listen to.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PluginEvent {
    // --- Player events (8) ---
    PlayerJoin {
        player: PluginPlayer,
    },
    PlayerQuit {
        player: PluginPlayer,
    },
    PlayerChat {
        player: PluginPlayer,
        message: String,
    },
    PlayerCommand {
        player: PluginPlayer,
        command: String,
        args: Vec<String>,
    },
    PlayerMove {
        player: PluginPlayer,
        from: (f32, f32, f32),
        to: (f32, f32, f32),
    },
    PlayerDeath {
        player: PluginPlayer,
        message: String,
    },
    PlayerDamage {
        player: PluginPlayer,
        damage: f32,
        cause: DamageCause,
    },
    PlayerRespawn {
        player: PluginPlayer,
    },

    // --- Block events (2) ---
    BlockBreak {
        player: PluginPlayer,
        position: PluginBlockPos,
        block_id: u32,
    },
    BlockPlace {
        player: PluginPlayer,
        position: PluginBlockPos,
        block_id: u32,
    },

    // --- Entity events (3) ---
    MobSpawn {
        mob_type: String,
        runtime_id: u64,
        position: (f32, f32, f32),
    },
    MobDeath {
        mob_type: String,
        runtime_id: u64,
        killer_runtime_id: Option<u64>,
    },
    EntityDamage {
        runtime_id: u64,
        damage: f32,
        attacker_runtime_id: Option<u64>,
    },

    // --- World events (2) ---
    WeatherChange {
        raining: bool,
        thundering: bool,
    },
    TimeChange {
        new_time: i64,
    },

    // --- Form events (1) ---
    FormResponse {
        player: PluginPlayer,
        form_id: u32,
        response: FormResponse,
    },

    // --- Server events (2) ---
    ServerStarted,
    ServerStopping,
}

impl PluginEvent {
    /// Whether this event type can be cancelled by a plugin.
    pub fn is_cancellable(&self) -> bool {
        matches!(
            self,
            PluginEvent::PlayerChat { .. }
                | PluginEvent::PlayerCommand { .. }
                | PluginEvent::PlayerMove { .. }
                | PluginEvent::PlayerDamage { .. }
                | PluginEvent::BlockBreak { .. }
                | PluginEvent::BlockPlace { .. }
                | PluginEvent::MobSpawn { .. }
                | PluginEvent::EntityDamage { .. }
                | PluginEvent::WeatherChange { .. }
                | PluginEvent::TimeChange { .. }
        )
    }
}

// ─── Plugin trait ────────────────────────────────────────────────────────────

/// Metadata about a plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

/// The Plugin trait: implemented by Lua/WASM runtimes or built-in plugins.
pub trait Plugin: Send {
    /// Return plugin metadata.
    fn info(&self) -> PluginInfo;

    /// Called when the plugin is loaded. Use `api` to register commands, schedule tasks.
    fn on_enable(&mut self, api: &mut dyn ServerApi);

    /// Called when the plugin is unloaded.
    fn on_disable(&mut self) {}

    /// Called for every dispatched event. Return `Cancelled` to cancel cancellable events.
    fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult {
        let _ = (event, api);
        EventResult::Continue
    }

    /// Called when a scheduled task fires.
    fn on_task(&mut self, task_id: u32, api: &mut dyn ServerApi) {
        let _ = (task_id, api);
    }

    /// Called when a plugin-registered command is executed. Return a response message.
    fn on_command(
        &mut self,
        command: &str,
        args: &[String],
        sender: &str,
        api: &mut dyn ServerApi,
    ) -> Option<String> {
        let _ = (command, args, sender, api);
        None
    }

    /// Return a default config as JSON. If `Some`, the plugin gets a config file.
    fn default_config(&self) -> Option<serde_json::Value> {
        None
    }

    /// Called with the loaded config (from `plugins/<name>/config.json`).
    fn load_config(&mut self, _config: serde_json::Value) {}
}

// ─── Server API ──────────────────────────────────────────────────────────────

/// Safe read/write access to server state, passed to plugins during callbacks.
///
/// Read methods return data immediately. Write methods are deferred (applied
/// after the plugin callback returns).
pub trait ServerApi {
    // --- Players ---
    fn online_players(&self) -> Vec<PluginPlayer>;
    fn get_player(&self, name: &str) -> Option<PluginPlayer>;
    fn send_message(&mut self, player_name: &str, message: &str);
    fn broadcast_message(&mut self, message: &str);
    fn kick_player(&mut self, player_name: &str, reason: &str);
    fn set_player_health(&mut self, player_name: &str, health: f32);
    fn set_player_food(&mut self, player_name: &str, food: i32);
    fn teleport_player(&mut self, player_name: &str, x: f32, y: f32, z: f32);

    // --- World ---
    fn get_time(&self) -> i64;
    fn set_time(&mut self, time: i64);
    fn is_raining(&self) -> bool;

    // --- Entities ---
    fn spawn_mob(&mut self, mob_type: &str, x: f32, y: f32, z: f32);
    fn remove_mob(&mut self, runtime_id: u64);

    // --- Server ---
    fn get_tick(&self) -> u64;
    fn log(&self, level: LogLevel, message: &str);

    // --- Scheduler ---
    fn schedule_delayed(&mut self, plugin_name: &str, delay_ticks: u64, task_id: u32);
    fn schedule_repeating(
        &mut self,
        plugin_name: &str,
        delay_ticks: u64,
        interval_ticks: u64,
        task_id: u32,
    );
    fn cancel_task(&mut self, plugin_name: &str, task_id: u32);

    // --- Commands ---
    fn register_command(&mut self, name: &str, description: &str, plugin_name: &str);

    // --- Forms ---
    fn show_simple_form(&mut self, player_name: &str, form_id: u32, json: &str);
    fn show_modal_form(&mut self, player_name: &str, form_id: u32, json: &str);
    fn show_custom_form(&mut self, player_name: &str, form_id: u32, json: &str);
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_player() -> PluginPlayer {
        PluginPlayer {
            name: "TestPlayer".into(),
            uuid: "00000000-0000-0000-0000-000000000001".into(),
            runtime_id: 1,
            position: (0.5, 65.62, 0.5),
            gamemode: 0,
            health: 20.0,
        }
    }

    // Minimal ServerApi implementation for testing.
    struct MockApi {
        messages: Vec<(String, String)>,
        broadcasts: Vec<String>,
        commands: Vec<(String, String)>,
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                messages: Vec::new(),
                broadcasts: Vec::new(),
                commands: Vec::new(),
            }
        }
    }

    impl ServerApi for MockApi {
        fn online_players(&self) -> Vec<PluginPlayer> {
            vec![test_player()]
        }
        fn get_player(&self, name: &str) -> Option<PluginPlayer> {
            if name == "TestPlayer" {
                Some(test_player())
            } else {
                None
            }
        }
        fn send_message(&mut self, player_name: &str, message: &str) {
            self.messages
                .push((player_name.to_string(), message.to_string()));
        }
        fn broadcast_message(&mut self, message: &str) {
            self.broadcasts.push(message.to_string());
        }
        fn kick_player(&mut self, _player_name: &str, _reason: &str) {}
        fn set_player_health(&mut self, _player_name: &str, _health: f32) {}
        fn set_player_food(&mut self, _player_name: &str, _food: i32) {}
        fn teleport_player(&mut self, _player_name: &str, _x: f32, _y: f32, _z: f32) {}
        fn get_time(&self) -> i64 {
            6000
        }
        fn set_time(&mut self, _time: i64) {}
        fn is_raining(&self) -> bool {
            false
        }
        fn spawn_mob(&mut self, _mob_type: &str, _x: f32, _y: f32, _z: f32) {}
        fn remove_mob(&mut self, _runtime_id: u64) {}
        fn get_tick(&self) -> u64 {
            100
        }
        fn log(&self, _level: LogLevel, _message: &str) {}
        fn schedule_delayed(&mut self, _plugin_name: &str, _delay_ticks: u64, _task_id: u32) {}
        fn schedule_repeating(
            &mut self,
            _plugin_name: &str,
            _delay_ticks: u64,
            _interval_ticks: u64,
            _task_id: u32,
        ) {
        }
        fn cancel_task(&mut self, _plugin_name: &str, _task_id: u32) {}
        fn register_command(&mut self, name: &str, description: &str, _plugin_name: &str) {
            self.commands
                .push((name.to_string(), description.to_string()));
        }
        fn show_simple_form(&mut self, _player_name: &str, _form_id: u32, _json: &str) {}
        fn show_modal_form(&mut self, _player_name: &str, _form_id: u32, _json: &str) {}
        fn show_custom_form(&mut self, _player_name: &str, _form_id: u32, _json: &str) {}
    }

    // A simple test plugin.
    struct HelloPlugin {
        greet_on_join: bool,
    }

    impl HelloPlugin {
        fn new() -> Self {
            Self {
                greet_on_join: true,
            }
        }
    }

    impl Plugin for HelloPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                name: "HelloPlugin".into(),
                version: "1.0.0".into(),
                description: "Greets players on join".into(),
                author: "Test".into(),
            }
        }

        fn on_enable(&mut self, api: &mut dyn ServerApi) {
            api.register_command("greet", "Greet a player", "HelloPlugin");
            api.log(LogLevel::Info, "HelloPlugin enabled!");
        }

        fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult {
            match event {
                PluginEvent::PlayerJoin { player } if self.greet_on_join => {
                    api.send_message(&player.name, &format!("Welcome, {}!", player.name));
                    EventResult::Continue
                }
                PluginEvent::PlayerChat { message, .. } if message.contains("bad") => {
                    EventResult::Cancelled
                }
                _ => EventResult::Continue,
            }
        }

        fn on_command(
            &mut self,
            command: &str,
            args: &[String],
            sender: &str,
            api: &mut dyn ServerApi,
        ) -> Option<String> {
            if command == "greet" {
                let target = args.first().map(|s| s.as_str()).unwrap_or(sender);
                api.send_message(target, "Hello from plugin!");
                Some(format!("Greeted {target}"))
            } else {
                None
            }
        }

        fn default_config(&self) -> Option<serde_json::Value> {
            Some(serde_json::json!({ "greet_on_join": true }))
        }

        fn load_config(&mut self, config: serde_json::Value) {
            if let Some(v) = config.get("greet_on_join").and_then(|v| v.as_bool()) {
                self.greet_on_join = v;
            }
        }
    }

    #[test]
    fn plugin_info() {
        let plugin = HelloPlugin::new();
        let info = plugin.info();
        assert_eq!(info.name, "HelloPlugin");
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn plugin_on_enable_registers_command() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        plugin.on_enable(&mut api);
        assert_eq!(api.commands.len(), 1);
        assert_eq!(api.commands[0].0, "greet");
    }

    #[test]
    fn plugin_greets_on_join() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let event = PluginEvent::PlayerJoin {
            player: test_player(),
        };
        let result = plugin.on_event(&event, &mut api);
        assert_eq!(result, EventResult::Continue);
        assert_eq!(api.messages.len(), 1);
        assert_eq!(api.messages[0].0, "TestPlayer");
        assert!(api.messages[0].1.contains("Welcome"));
    }

    #[test]
    fn plugin_cancels_bad_chat() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let event = PluginEvent::PlayerChat {
            player: test_player(),
            message: "this is bad word".into(),
        };
        let result = plugin.on_event(&event, &mut api);
        assert_eq!(result, EventResult::Cancelled);
    }

    #[test]
    fn plugin_allows_good_chat() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let event = PluginEvent::PlayerChat {
            player: test_player(),
            message: "hello everyone".into(),
        };
        let result = plugin.on_event(&event, &mut api);
        assert_eq!(result, EventResult::Continue);
    }

    #[test]
    fn plugin_command_greet() {
        let mut plugin = HelloPlugin::new();
        let mut api = MockApi::new();
        let response = plugin.on_command("greet", &["Alice".into()], "Bob", &mut api);
        assert_eq!(response, Some("Greeted Alice".into()));
        assert_eq!(api.messages.len(), 1);
        assert_eq!(api.messages[0].0, "Alice");
    }

    #[test]
    fn plugin_config_roundtrip() {
        let mut plugin = HelloPlugin::new();
        assert!(plugin.greet_on_join);

        let config = serde_json::json!({ "greet_on_join": false });
        plugin.load_config(config);
        assert!(!plugin.greet_on_join);

        let default = plugin.default_config().unwrap();
        assert_eq!(default["greet_on_join"], true);
    }

    #[test]
    fn event_cancellable_flags() {
        assert!(PluginEvent::PlayerChat {
            player: test_player(),
            message: String::new()
        }
        .is_cancellable());
        assert!(PluginEvent::BlockBreak {
            player: test_player(),
            position: PluginBlockPos { x: 0, y: 0, z: 0 },
            block_id: 0,
        }
        .is_cancellable());
        assert!(PluginEvent::PlayerDamage {
            player: test_player(),
            damage: 0.0,
            cause: DamageCause::Attack,
        }
        .is_cancellable());
        assert!(!PluginEvent::PlayerJoin {
            player: test_player()
        }
        .is_cancellable());
        assert!(!PluginEvent::PlayerQuit {
            player: test_player()
        }
        .is_cancellable());
        assert!(!PluginEvent::ServerStarted.is_cancellable());
        assert!(!PluginEvent::ServerStopping.is_cancellable());
        assert!(!PluginEvent::MobDeath {
            mob_type: String::new(),
            runtime_id: 0,
            killer_runtime_id: None,
        }
        .is_cancellable());
        assert!(!PluginEvent::FormResponse {
            player: test_player(),
            form_id: 0,
            response: FormResponse::Cancelled,
        }
        .is_cancellable());
    }

    #[test]
    fn simple_form_builder_json() {
        let json = SimpleFormBuilder::new("Title", "Content")
            .button("Button 1")
            .button("Button 2")
            .to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "form");
        assert_eq!(v["title"], "Title");
        assert_eq!(v["content"], "Content");
        assert_eq!(v["buttons"].as_array().unwrap().len(), 2);
        assert_eq!(v["buttons"][0]["text"], "Button 1");
        assert_eq!(v["buttons"][1]["text"], "Button 2");
    }

    #[test]
    fn simple_form_button_with_image() {
        let json = SimpleFormBuilder::new("T", "C")
            .button_with_image("Btn", "url", "https://example.com/icon.png")
            .to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["buttons"][0]["image"]["type"], "url");
        assert_eq!(
            v["buttons"][0]["image"]["data"],
            "https://example.com/icon.png"
        );
    }

    #[test]
    fn modal_form_builder_json() {
        let json = ModalFormBuilder::new("Confirm", "Are you sure?", "Yes", "No").to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "modal");
        assert_eq!(v["title"], "Confirm");
        assert_eq!(v["content"], "Are you sure?");
        assert_eq!(v["button1"], "Yes");
        assert_eq!(v["button2"], "No");
    }

    #[test]
    fn custom_form_builder_json() {
        let json = CustomFormBuilder::new("Settings")
            .label("Configure your settings:")
            .input("Name", "Enter name", "Steve")
            .toggle("Enable PvP", true)
            .dropdown(
                "Difficulty",
                vec!["Easy".into(), "Normal".into(), "Hard".into()],
                1,
            )
            .slider("Volume", 0.0, 100.0, 1.0, 50.0)
            .step_slider("Mode", vec!["Survival".into(), "Creative".into()], 0)
            .to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "custom_form");
        assert_eq!(v["title"], "Settings");
        let content = v["content"].as_array().unwrap();
        assert_eq!(content.len(), 6);
        assert_eq!(content[0]["type"], "label");
        assert_eq!(content[1]["type"], "input");
        assert_eq!(content[1]["default"], "Steve");
        assert_eq!(content[2]["type"], "toggle");
        assert_eq!(content[2]["default"], true);
        assert_eq!(content[3]["type"], "dropdown");
        assert_eq!(content[3]["options"].as_array().unwrap().len(), 3);
        assert_eq!(content[4]["type"], "slider");
        assert_eq!(content[5]["type"], "step_slider");
    }

    #[test]
    fn parse_simple_form_response() {
        let resp = parse_form_response("simple", Some("2"));
        match resp {
            FormResponse::Simple { button_index } => assert_eq!(button_index, 2),
            other => panic!("Expected Simple, got {other:?}"),
        }
    }

    #[test]
    fn parse_modal_form_response() {
        let resp = parse_form_response("modal", Some("true"));
        match resp {
            FormResponse::Modal { accepted } => assert!(accepted),
            other => panic!("Expected Modal, got {other:?}"),
        }
        let resp2 = parse_form_response("modal", Some("false"));
        match resp2 {
            FormResponse::Modal { accepted } => assert!(!accepted),
            other => panic!("Expected Modal, got {other:?}"),
        }
    }

    #[test]
    fn parse_custom_form_response() {
        let resp = parse_form_response("custom", Some(r#"["Steve", true, 1, 50.0]"#));
        match resp {
            FormResponse::Custom { values } => {
                assert_eq!(values.len(), 4);
                assert_eq!(values[0], "Steve");
                assert_eq!(values[1], true);
            }
            other => panic!("Expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn parse_cancelled_form_response() {
        let resp = parse_form_response("simple", None);
        assert!(matches!(resp, FormResponse::Cancelled));
    }
}
