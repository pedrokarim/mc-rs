use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use mc_rs_command::{
    CommandDefinition, CommandInvocation, CommandOverload, CommandParameter, ParamType,
    PermissionDefault, PermissionDefinition, PermissionRegistry,
};
use mlua::{Function, Lua, RegistryKey, Table, Value as LuaValue};
use serde::de::{self, Deserializer};
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::commands::{ServerCommandRuntime, ServerCommandSystem};

const MANIFEST_FILE_NAMES: [&str; 2] = ["plugin.yml", "plugin.yaml"];
const PLUGIN_API_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluginLoadOrder {
    Startup,
    #[default]
    PostWorld,
}

impl<'de> Deserialize<'de> for PluginLoadOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.trim().to_ascii_uppercase().as_str() {
            "STARTUP" => Ok(Self::Startup),
            "POSTWORLD" | "POST_WORLD" => Ok(Self::PostWorld),
            other => Err(de::Error::custom(format!(
                "invalid plugin load order: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluginPermissionDefault {
    True,
    False,
    #[default]
    Op,
}

impl From<PluginPermissionDefault> for PermissionDefault {
    fn from(value: PluginPermissionDefault) -> Self {
        match value {
            PluginPermissionDefault::True => PermissionDefault::True,
            PluginPermissionDefault::False => PermissionDefault::False,
            PluginPermissionDefault::Op => PermissionDefault::Op,
        }
    }
}

impl<'de> Deserialize<'de> for PluginPermissionDefault {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum PermissionDefaultValue {
            Bool(bool),
            String(String),
        }

        match PermissionDefaultValue::deserialize(deserializer)? {
            PermissionDefaultValue::Bool(true) => Ok(PluginPermissionDefault::True),
            PermissionDefaultValue::Bool(false) => Ok(PluginPermissionDefault::False),
            PermissionDefaultValue::String(value) => {
                match value.trim().to_ascii_lowercase().as_str() {
                    "true" => Ok(PluginPermissionDefault::True),
                    "false" => Ok(PluginPermissionDefault::False),
                    "op" => Ok(PluginPermissionDefault::Op),
                    other => Err(de::Error::custom(format!(
                        "invalid permission default: {other}"
                    ))),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginCommandManifest {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub usage: String,
    #[serde(default, deserialize_with = "deserialize_aliases")]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(rename = "permission-message", default)]
    pub permission_message: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginPermissionManifest {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default: PluginPermissionDefault,
    #[serde(default)]
    pub children: BTreeMap<String, bool>,
}

#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub main: String,
    pub api: Vec<String>,
    pub description: String,
    pub authors: Vec<String>,
    pub website: Option<String>,
    pub load: PluginLoadOrder,
    pub prefix: Option<String>,
    pub depend: Vec<String>,
    pub softdepend: Vec<String>,
    pub loadbefore: Vec<String>,
    pub mcpe_protocol: Option<u32>,
    pub commands: BTreeMap<String, PluginCommandManifest>,
    pub permissions: BTreeMap<String, PluginPermissionManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Discovered,
    Enabled,
    Disabled,
    Failed(String),
}

#[derive(Debug)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub root_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub data_dir: PathBuf,
    pub status: PluginStatus,
    runtime: Option<PluginRuntime>,
}

#[derive(Debug, Default)]
pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
}

#[derive(Debug, Deserialize)]
struct RawPluginManifest {
    name: String,
    version: String,
    main: String,
    #[serde(deserialize_with = "deserialize_string_or_list")]
    api: Vec<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    website: Option<String>,
    #[serde(default)]
    load: PluginLoadOrder,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_list")]
    depend: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_list")]
    softdepend: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_list")]
    loadbefore: Vec<String>,
    #[serde(rename = "mcpe-protocol", default)]
    mcpe_protocol: Option<u32>,
    #[serde(default)]
    commands: BTreeMap<String, PluginCommandManifest>,
    #[serde(default)]
    permissions: BTreeMap<String, PluginPermissionManifest>,
}

#[derive(Debug)]
struct PluginRuntime {
    lua: Lua,
    tick_counter: u64,
    scheduled_tasks: Vec<ScheduledTask>,
    event_handlers: HashMap<String, RegistryKey>,
}

impl PluginRuntime {
    fn new(lua: Lua) -> Self {
        Self {
            lua,
            tick_counter: 0,
            scheduled_tasks: Vec::new(),
            event_handlers: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct LuaHostData {
    plugin_name: String,
    plugin_prefix: String,
    root_dir: PathBuf,
    data_dir: PathBuf,
    manifest_commands: HashSet<String>,
    command_handlers: HashMap<String, RegistryKey>,
    actions: Vec<PluginAction>,
}

#[derive(Debug)]
enum PluginAction {
    Log {
        level: PluginLogLevel,
        message: String,
    },
    Broadcast {
        message: String,
    },
    /// Schedule a Lua callback to fire after `delay_ticks` server ticks.
    Schedule {
        delay_ticks: u64,
        callback_key: mlua::RegistryKey,
    },
    /// Register a Lua event handler. `event_name` ex: "PlayerJoin".
    RegisterEvent {
        event_name: String,
        callback_key: mlua::RegistryKey,
    },
}

#[derive(Debug)]
enum PluginLogLevel {
    Info,
    Warn,
    Error,
}

impl RawPluginManifest {
    fn into_manifest(self) -> Result<PluginManifest, String> {
        let name = self.name.trim().to_string();
        let version = self.version.trim().to_string();
        let main = self.main.trim().to_string();
        if name.is_empty() {
            return Err("plugin name cannot be empty".into());
        }
        if version.is_empty() {
            return Err(format!("plugin {name} must declare a version"));
        }
        if main.is_empty() {
            return Err(format!("plugin {name} must declare a main entrypoint"));
        }
        if self.api.is_empty() {
            return Err(format!(
                "plugin {name} must declare at least one api version"
            ));
        }

        let mut authors = self
            .authors
            .into_iter()
            .map(|author| author.trim().to_string())
            .filter(|author| !author.is_empty())
            .collect::<Vec<_>>();
        if let Some(author) = self.author {
            let author = author.trim().to_string();
            if !author.is_empty() && !authors.iter().any(|existing| existing == &author) {
                authors.push(author);
            }
        }

        let api = self
            .api
            .into_iter()
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty())
            .collect::<Vec<_>>();
        if api.is_empty() {
            return Err(format!(
                "plugin {name} must declare at least one non-empty api version"
            ));
        }

        Ok(PluginManifest {
            name,
            version,
            main,
            api,
            description: self.description.trim().to_string(),
            authors,
            website: self
                .website
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            load: self.load,
            prefix: self
                .prefix
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            depend: normalize_name_list(self.depend),
            softdepend: normalize_name_list(self.softdepend),
            loadbefore: normalize_name_list(self.loadbefore),
            mcpe_protocol: self.mcpe_protocol,
            commands: self.commands,
            permissions: self.permissions,
        })
    }
}

impl PluginManager {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn load_from_dir(plugins_dir: &Path) -> Self {
        if let Err(error) = fs::create_dir_all(plugins_dir) {
            warn!(
                "Failed to ensure plugins directory exists at {}: {}",
                plugins_dir.display(),
                error
            );
            return Self::default();
        }

        let data_root = plugins_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("plugin_data");
        if let Err(error) = fs::create_dir_all(&data_root) {
            warn!(
                "Failed to ensure plugin data directory exists at {}: {}",
                data_root.display(),
                error
            );
        }

        let mut discovered = HashMap::<String, LoadedPlugin>::new();
        let entries = match fs::read_dir(plugins_dir) {
            Ok(entries) => entries,
            Err(error) => {
                warn!(
                    "Failed to read plugins directory at {}: {}",
                    plugins_dir.display(),
                    error
                );
                return Self::default();
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(manifest_path) = manifest_path_for_dir(&path) else {
                continue;
            };

            match load_plugin_candidate(&manifest_path, &path, &data_root) {
                Ok(plugin) => {
                    let key = plugin.manifest.name.to_ascii_lowercase();
                    if let std::collections::hash_map::Entry::Vacant(entry) = discovered.entry(key)
                    {
                        entry.insert(plugin);
                    } else {
                        warn!(
                            "Skipping duplicate plugin manifest {} because another manifest with the same name is already loaded",
                            plugin.manifest.name,
                        );
                    }
                }
                Err(error) => warn!("{error}"),
            }
        }

        let available_names = discovered.keys().cloned().collect::<Vec<_>>();
        let retained = discovered
            .into_values()
            .filter(|plugin| {
                let missing = plugin
                    .manifest
                    .depend
                    .iter()
                    .filter(|dependency| {
                        let dependency = dependency.to_ascii_lowercase();
                        !available_names.iter().any(|name| name == &dependency)
                    })
                    .collect::<Vec<_>>();
                if missing.is_empty() {
                    true
                } else {
                    warn!(
                        "Skipping plugin {} because required dependencies are missing: {}",
                        plugin.manifest.name,
                        missing
                            .iter()
                            .map(|dependency| dependency.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    false
                }
            })
            .collect::<Vec<_>>();

        let plugins = order_plugins(retained);
        if plugins.is_empty() {
            info!("No plugin manifests loaded from {}", plugins_dir.display());
        } else {
            info!(
                "Discovered {} plugin manifest(s): {}",
                plugins.len(),
                plugins
                    .iter()
                    .map(|plugin| plugin.manifest.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        Self { plugins }
    }

    pub fn register_permissions(&self, registry: &mut PermissionRegistry) {
        for plugin in &self.plugins {
            for (name, permission) in &plugin.manifest.permissions {
                let mut definition = PermissionDefinition::new(permission.default.into());
                for (child, value) in &permission.children {
                    definition
                        .children
                        .insert(child.to_ascii_lowercase(), *value);
                }
                registry.register(name.clone(), definition);
            }
        }
    }

    pub fn enable_plugins(
        &mut self,
        order: PluginLoadOrder,
        command_system: &mut ServerCommandSystem,
    ) {
        for index in 0..self.plugins.len() {
            if self.plugins[index].manifest.load != order {
                continue;
            }
            if !matches!(
                self.plugins[index].status,
                PluginStatus::Discovered | PluginStatus::Disabled
            ) {
                continue;
            }

            if let Err(error) = self.enable_plugin(index, command_system) {
                let plugin_name = self.plugins[index].manifest.name.clone();
                self.plugins[index].status = PluginStatus::Failed(error.clone());
                self.plugins[index].runtime = None;
                warn!("Failed to enable plugin {}: {}", plugin_name, error);
            }
        }
    }

    pub fn disable_all(&mut self, command_system: &mut ServerCommandSystem) {
        for index in (0..self.plugins.len()).rev() {
            self.disable_plugin(index, command_system);
        }
    }

    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins
            .iter()
            .filter(|plugin| matches!(plugin.status, PluginStatus::Enabled))
            .map(|plugin| plugin.manifest.name.clone())
            .collect()
    }

    pub fn plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }

    pub fn execute_command(
        &mut self,
        plugin_name: &str,
        command_name: &str,
        invocation: &CommandInvocation,
        runtime: &mut dyn ServerCommandRuntime,
    ) -> Result<(), String> {
        let command_name = normalize_command_label(command_name);
        let entry = self
            .plugins
            .iter_mut()
            .find(|plugin| {
                plugin.manifest.name.eq_ignore_ascii_case(plugin_name)
                    && matches!(plugin.status, PluginStatus::Enabled)
            })
            .ok_or_else(|| format!("Plugin {} is not enabled.", plugin_name))?;

        let Some(plugin_runtime) = entry.runtime.as_mut() else {
            return Err(format!("Plugin {} runtime is unavailable.", plugin_name));
        };

        let handler = {
            let host = plugin_runtime
                .lua
                .app_data_ref::<LuaHostData>()
                .ok_or_else(|| format!("Plugin {} host data is unavailable.", plugin_name))?;
            let Some(key) = host.command_handlers.get(&command_name) else {
                return Err(format!(
                    "Plugin {} has no registered handler for /{}.",
                    plugin_name, command_name
                ));
            };
            plugin_runtime
                .lua
                .registry_value::<Function>(key)
                .map_err(|error| {
                    format!(
                        "Plugin {} could not restore handler for /{}: {}",
                        plugin_name, command_name, error
                    )
                })?
        };

        let response = {
            let sender = create_sender_table(&plugin_runtime.lua, runtime).map_err(|error| {
                format!(
                    "Plugin {} failed to build sender context for /{}: {}",
                    plugin_name, command_name, error
                )
            })?;
            let args = plugin_runtime
                .lua
                .create_sequence_from(invocation.args.iter().map(String::as_str))
                .map_err(|error| {
                    format!(
                        "Plugin {} failed to build args table for /{}: {}",
                        plugin_name, command_name, error
                    )
                })?;
            let raw_args = invocation.raw_args.clone();
            let label = invocation.label.clone();
            let result = handler
                .call::<LuaValue>((sender, args, raw_args, label))
                .map_err(|error| {
                    format!(
                        "Plugin {} failed while handling /{}: {}",
                        plugin_name, command_name, error
                    )
                })?;
            lua_response_to_string(result).map_err(|error| {
                format!(
                    "Plugin {} returned an unsupported value for /{}: {}",
                    plugin_name, command_name, error
                )
            })?
        };

        Self::flush_actions(entry, Some(runtime));
        if let Some(message) = response {
            runtime.send_feedback(&message);
        }
        Ok(())
    }

    fn enable_plugin(
        &mut self,
        index: usize,
        command_system: &mut ServerCommandSystem,
    ) -> Result<(), String> {
        let entry = &mut self.plugins[index];
        let script_path =
            resolve_main_script(&entry.root_dir, &entry.manifest.main).ok_or_else(|| {
                format!(
                    "Could not resolve main script '{}' for plugin {}",
                    entry.manifest.main, entry.manifest.name
                )
            })?;

        let script = fs::read_to_string(&script_path).map_err(|error| {
            format!(
                "Failed to read plugin script at {}: {}",
                script_path.display(),
                error
            )
        })?;

        let runtime = PluginRuntime::new(build_lua_runtime(entry)?);

        runtime
            .lua
            .load(&script)
            .set_name(script_path.to_string_lossy().as_ref())
            .exec()
            .map_err(|error| {
                format!(
                    "Failed to execute Lua script for plugin {}: {}",
                    entry.manifest.name, error
                )
            })?;

        entry.runtime = Some(runtime);
        self.call_hook(index, "on_load")?;
        self.call_hook(index, "on_enable")?;
        self.register_plugin_commands(index, command_system)?;
        self.plugins[index].status = PluginStatus::Enabled;
        info!(
            "Enabled plugin {} v{}",
            self.plugins[index].manifest.name, self.plugins[index].manifest.version
        );
        Ok(())
    }

    fn disable_plugin(&mut self, index: usize, command_system: &mut ServerCommandSystem) {
        if !matches!(self.plugins[index].status, PluginStatus::Enabled) {
            return;
        }

        if let Err(error) = self.call_hook(index, "on_disable") {
            warn!(
                "Plugin {} failed during on_disable: {}",
                self.plugins[index].manifest.name, error
            );
        }

        let plugin_name = self.plugins[index].manifest.name.clone();
        command_system.map.unregister_owner(&plugin_name);
        self.plugins[index].runtime = None;
        self.plugins[index].status = PluginStatus::Disabled;
        info!("Disabled plugin {}", plugin_name);
    }

    fn call_hook(&mut self, index: usize, hook_name: &str) -> Result<(), String> {
        let entry = &mut self.plugins[index];
        let Some(runtime) = entry.runtime.as_mut() else {
            return Err(format!(
                "Plugin {} runtime is unavailable.",
                entry.manifest.name
            ));
        };

        let globals = runtime.lua.globals();
        let hook = globals.get::<LuaValue>(hook_name).map_err(|error| {
            format!(
                "Plugin {} could not access {}: {}",
                entry.manifest.name, hook_name, error
            )
        })?;
        if let LuaValue::Function(function) = hook {
            function.call::<()>(()).map_err(|error| {
                format!(
                    "Plugin {} failed in {}: {}",
                    entry.manifest.name, hook_name, error
                )
            })?;
        }

        Self::flush_actions(entry, None);
        Ok(())
    }

    fn register_plugin_commands(
        &mut self,
        index: usize,
        command_system: &mut ServerCommandSystem,
    ) -> Result<(), String> {
        let entry = &mut self.plugins[index];
        let Some(runtime) = entry.runtime.as_ref() else {
            return Err(format!(
                "Plugin {} runtime is unavailable.",
                entry.manifest.name
            ));
        };

        let handler_names = runtime
            .lua
            .app_data_ref::<LuaHostData>()
            .map(|host| host.command_handlers.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        let mut registered: Vec<String> = Vec::new();
        for name in handler_names {
            let Some(command_manifest) = entry.manifest.commands.get(&name) else {
                warn!(
                    "Plugin {} registered /{} in Lua but did not declare it in plugin.yml",
                    entry.manifest.name, name
                );
                continue;
            };

            let mut definition = CommandDefinition::new(&name, &command_manifest.description);
            definition.usage = if command_manifest.usage.is_empty() {
                format!("/{name} [args]")
            } else {
                command_manifest.usage.clone()
            };
            definition.aliases = command_manifest.aliases.clone();
            definition.permission_message = command_manifest.permission_message.clone();
            definition.owner = Some(entry.manifest.name.clone());
            if let Some(permission) = command_manifest.permission.as_ref() {
                definition.permissions.push(permission.to_ascii_lowercase());
                if command_system.permissions.definition(permission).is_none() {
                    command_system.permissions.register(
                        permission.clone(),
                        PermissionDefinition::new(PermissionDefault::Op),
                    );
                    warn!(
                        "Plugin {} uses undeclared permission {}; defaulting it to op",
                        entry.manifest.name, permission
                    );
                }
            }
            definition.overloads.push(CommandOverload {
                parameters: vec![CommandParameter {
                    name: "args".to_string(),
                    param_type: ParamType::RawText,
                    optional: true,
                }],
            });

            let plugin_name = entry.manifest.name.clone();
            let command_name = name.clone();
            if let Err(error) =
                command_system
                    .map
                    .register(definition, move |runtime, invocation| {
                        runtime.execute_plugin_command(&plugin_name, &command_name, invocation)
                    })
            {
                for added in registered {
                    command_system.map.unregister(&added);
                }
                return Err(format!(
                    "Plugin {} failed to register /{}: {}",
                    entry.manifest.name, name, error
                ));
            }
            registered.push(name);
        }

        for command_name in entry.manifest.commands.keys() {
            if !registered
                .iter()
                .any(|registered_name| registered_name == command_name)
            {
                warn!(
                    "Plugin {} declares /{} in plugin.yml but did not register a Lua handler",
                    entry.manifest.name, command_name
                );
            }
        }

        Ok(())
    }

    fn flush_actions(entry: &mut LoadedPlugin, mut runtime: Option<&mut dyn ServerCommandRuntime>) {
        let Some(plugin_runtime) = entry.runtime.as_mut() else {
            return;
        };

        let actions = plugin_runtime
            .lua
            .app_data_mut::<LuaHostData>()
            .map(|mut host| std::mem::take(&mut host.actions))
            .unwrap_or_default();

        for action in actions {
            match action {
                PluginAction::Log { level, message } => match level {
                    PluginLogLevel::Info => info!("{message}"),
                    PluginLogLevel::Warn => warn!("{message}"),
                    PluginLogLevel::Error => error!("{message}"),
                },
                PluginAction::Broadcast { message } => {
                    if let Some(runtime) = runtime.as_deref_mut() {
                        let source = entry
                            .manifest
                            .prefix
                            .as_deref()
                            .unwrap_or(entry.manifest.name.as_str());
                        runtime.broadcast_chat(source, &message);
                    } else {
                        warn!(
                            "Plugin {} attempted to broadcast outside a command runtime; ignoring message",
                            entry.manifest.name
                        );
                    }
                }
                PluginAction::Schedule {
                    delay_ticks,
                    callback_key,
                } => {
                    let fire_at = plugin_runtime.tick_counter.saturating_add(delay_ticks);
                    plugin_runtime.scheduled_tasks.push(ScheduledTask {
                        fire_at_tick: fire_at,
                        callback_key,
                    });
                }
                PluginAction::RegisterEvent {
                    event_name,
                    callback_key,
                } => {
                    if let Some(old) = plugin_runtime
                        .event_handlers
                        .insert(event_name, callback_key)
                    {
                        let _ = plugin_runtime.lua.remove_registry_value(old);
                    }
                }
            }
        }
    }

    /// Tick scheduler — appelé chaque server tick. Fire les callbacks Lua dont
    /// `fire_at_tick` est atteint.
    pub fn tick_scheduler(&mut self) {
        for entry in &mut self.plugins {
            let Some(rt) = entry.runtime.as_mut() else {
                continue;
            };
            rt.tick_counter = rt.tick_counter.saturating_add(1);
            let now = rt.tick_counter;
            let pending = std::mem::take(&mut rt.scheduled_tasks);
            for task in pending {
                if task.fire_at_tick <= now {
                    if let Ok(handler) = rt.lua.registry_value::<Function>(&task.callback_key) {
                        if let Err(e) = handler.call::<()>(()) {
                            warn!("Lua scheduled task error: {e}");
                        }
                    }
                    let _ = rt.lua.remove_registry_value(task.callback_key);
                } else {
                    rt.scheduled_tasks.push(task);
                }
            }
        }
    }
}

#[derive(Debug)]
struct ScheduledTask {
    fire_at_tick: u64,
    callback_key: mlua::RegistryKey,
}

fn manifest_path_for_dir(dir: &Path) -> Option<PathBuf> {
    MANIFEST_FILE_NAMES
        .iter()
        .map(|file_name| dir.join(file_name))
        .find(|path| path.is_file())
}

fn load_plugin_candidate(
    manifest_path: &Path,
    root_dir: &Path,
    data_root: &Path,
) -> Result<LoadedPlugin, String> {
    let raw = fs::read_to_string(manifest_path).map_err(|error| {
        format!(
            "Failed to read plugin manifest at {}: {}",
            manifest_path.display(),
            error
        )
    })?;
    let manifest = serde_yaml::from_str::<RawPluginManifest>(&raw)
        .map_err(|error| {
            format!(
                "Invalid plugin manifest at {}: {}",
                manifest_path.display(),
                error
            )
        })?
        .into_manifest()?;

    let data_dir = data_root.join(sanitize_plugin_dir_name(&manifest.name));
    fs::create_dir_all(&data_dir).map_err(|error| {
        format!(
            "Failed to create plugin data directory for {} at {}: {}",
            manifest.name,
            data_dir.display(),
            error
        )
    })?;

    Ok(LoadedPlugin {
        manifest,
        root_dir: root_dir.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
        data_dir,
        status: PluginStatus::Discovered,
        runtime: None,
    })
}

fn build_lua_runtime(plugin: &LoadedPlugin) -> Result<Lua, String> {
    let lua = Lua::new();
    let globals = lua.globals();
    globals
        .set("os", LuaValue::Nil)
        .and_then(|_| globals.set("io", LuaValue::Nil))
        .and_then(|_| globals.set("debug", LuaValue::Nil))
        .and_then(|_| globals.set("loadfile", LuaValue::Nil))
        .and_then(|_| globals.set("dofile", LuaValue::Nil))
        .map_err(|error| {
            format!(
                "Failed to set Lua sandbox for plugin {}: {}",
                plugin.manifest.name, error
            )
        })?;

    lua.set_app_data(LuaHostData {
        plugin_name: plugin.manifest.name.clone(),
        plugin_prefix: plugin
            .manifest
            .prefix
            .clone()
            .unwrap_or_else(|| plugin.manifest.name.clone()),
        root_dir: plugin.root_dir.clone(),
        data_dir: plugin.data_dir.clone(),
        manifest_commands: plugin
            .manifest
            .commands
            .keys()
            .map(|name| normalize_command_label(name))
            .collect(),
        command_handlers: HashMap::new(),
        actions: Vec::new(),
    });

    install_lua_api(&lua).map_err(|error| {
        format!(
            "Failed to install Lua API for plugin {}: {}",
            plugin.manifest.name, error
        )
    })?;

    Ok(lua)
}

fn install_lua_api(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();

    globals.set(
        "register_command",
        lua.create_function(|lua, (name, handler): (String, Function)| {
            let normalized = normalize_command_label(&name);
            let key = lua.create_registry_value(handler)?;
            let mut host = lua
                .app_data_mut::<LuaHostData>()
                .ok_or_else(|| mlua::Error::external("Lua host data is unavailable"))?;
            if !host.manifest_commands.contains(&normalized) {
                return Err(mlua::Error::external(format!(
                    "Command {} must be declared in plugin.yml before it can be registered",
                    normalized
                )));
            }
            if let Some(old) = host.command_handlers.insert(normalized, key) {
                lua.remove_registry_value(old)?;
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "log",
        lua.create_function(|lua, message: String| {
            push_action(
                lua,
                PluginAction::Log {
                    level: PluginLogLevel::Info,
                    message: prefixed_message(lua, &message),
                },
            );
            Ok(())
        })?,
    )?;
    globals.set(
        "warn",
        lua.create_function(|lua, message: String| {
            push_action(
                lua,
                PluginAction::Log {
                    level: PluginLogLevel::Warn,
                    message: prefixed_message(lua, &message),
                },
            );
            Ok(())
        })?,
    )?;
    globals.set(
        "error_log",
        lua.create_function(|lua, message: String| {
            push_action(
                lua,
                PluginAction::Log {
                    level: PluginLogLevel::Error,
                    message: prefixed_message(lua, &message),
                },
            );
            Ok(())
        })?,
    )?;
    globals.set(
        "broadcast",
        lua.create_function(|lua, message: String| {
            push_action(lua, PluginAction::Broadcast { message });
            Ok(())
        })?,
    )?;

    // schedule.after(delay_ticks, callback) — fire callback dans N server ticks.
    globals.set(
        "schedule_after",
        lua.create_function(|lua, (delay_ticks, handler): (u64, Function)| {
            let key = lua.create_registry_value(handler)?;
            push_action(
                lua,
                PluginAction::Schedule {
                    delay_ticks,
                    callback_key: key,
                },
            );
            Ok(())
        })?,
    )?;

    // register_event(event_name, callback) — handler appelé quand le serveur fire
    // un event de ce nom (ex: "PlayerJoin", "BlockBreak").
    globals.set(
        "register_event",
        lua.create_function(|lua, (event_name, handler): (String, Function)| {
            let key = lua.create_registry_value(handler)?;
            push_action(
                lua,
                PluginAction::RegisterEvent {
                    event_name,
                    callback_key: key,
                },
            );
            Ok(())
        })?,
    )?;
    globals.set(
        "get_plugin_name",
        lua.create_function(|lua, ()| {
            let name = lua
                .app_data_ref::<LuaHostData>()
                .map(|host| host.plugin_name.clone())
                .unwrap_or_default();
            Ok(name)
        })?,
    )?;
    globals.set(
        "get_data_folder",
        lua.create_function(|lua, ()| {
            let path = lua
                .app_data_ref::<LuaHostData>()
                .map(|host| host.data_dir.display().to_string())
                .unwrap_or_default();
            Ok(path)
        })?,
    )?;
    globals.set(
        "get_config_path",
        lua.create_function(|lua, ()| {
            let path = lua
                .app_data_ref::<LuaHostData>()
                .map(|host| host.data_dir.join("config.yml").display().to_string())
                .unwrap_or_default();
            Ok(path)
        })?,
    )?;
    globals.set(
        "save_default_config",
        lua.create_function(|lua, ()| {
            let Some(host) = lua.app_data_ref::<LuaHostData>() else {
                return Ok(false);
            };
            let source = host.root_dir.join("config.yml");
            let target = host.data_dir.join("config.yml");
            if target.exists() || !source.is_file() {
                return Ok(false);
            }
            fs::copy(&source, &target).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;
    globals.set(
        "load_config",
        lua.create_function(|lua, ()| {
            let Some(host) = lua.app_data_ref::<LuaHostData>() else {
                return Ok(LuaValue::Nil);
            };
            let config_path = host.data_dir.join("config.yml");
            if !config_path.is_file() {
                return Ok(LuaValue::Nil);
            }
            let raw = fs::read_to_string(config_path).map_err(mlua::Error::external)?;
            let value: serde_yaml::Value =
                serde_yaml::from_str(&raw).map_err(mlua::Error::external)?;
            yaml_to_lua(lua, &value)
        })?,
    )?;

    Ok(())
}

fn push_action(lua: &Lua, action: PluginAction) {
    if let Some(mut host) = lua.app_data_mut::<LuaHostData>() {
        host.actions.push(action);
    }
}

fn prefixed_message(lua: &Lua, message: &str) -> String {
    lua.app_data_ref::<LuaHostData>()
        .map(|host| format!("[{}] {}", host.plugin_prefix, message))
        .unwrap_or_else(|| message.to_string())
}

fn create_sender_table(lua: &Lua, runtime: &dyn ServerCommandRuntime) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("name", runtime.sender_name())?;
    table.set("is_player", runtime.sender_is_player())?;
    table.set("is_op", runtime.sender_is_op())?;
    Ok(table)
}

fn lua_response_to_string(value: LuaValue) -> Result<Option<String>, String> {
    match value {
        LuaValue::Nil => Ok(None),
        LuaValue::String(value) => value
            .to_str()
            .map(|text| Some(text.to_string()))
            .map_err(|error| error.to_string()),
        LuaValue::Boolean(value) => Ok(Some(value.to_string())),
        LuaValue::Integer(value) => Ok(Some(value.to_string())),
        LuaValue::Number(value) => Ok(Some(value.to_string())),
        other => Err(format!("unsupported return type {}", other.type_name())),
    }
}

fn yaml_to_lua(lua: &Lua, value: &serde_yaml::Value) -> mlua::Result<LuaValue> {
    Ok(match value {
        serde_yaml::Value::Null => LuaValue::Nil,
        serde_yaml::Value::Bool(value) => LuaValue::Boolean(*value),
        serde_yaml::Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                LuaValue::Integer(value)
            } else if let Some(value) = number.as_u64() {
                LuaValue::Integer(value as i64)
            } else if let Some(value) = number.as_f64() {
                LuaValue::Number(value)
            } else {
                LuaValue::Nil
            }
        }
        serde_yaml::Value::String(value) => LuaValue::String(lua.create_string(value)?),
        serde_yaml::Value::Sequence(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.set(index + 1, yaml_to_lua(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
        serde_yaml::Value::Mapping(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                let key = match key {
                    serde_yaml::Value::String(value) => value.clone(),
                    other => serde_yaml::to_string(other)
                        .unwrap_or_else(|_| "key".to_string())
                        .trim()
                        .to_string(),
                };
                table.set(key, yaml_to_lua(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_lua(lua, &tagged.value)?,
    })
}

fn resolve_main_script(root_dir: &Path, main: &str) -> Option<PathBuf> {
    let trimmed = main.trim();
    if trimmed.is_empty() {
        return None;
    }

    let direct = root_dir.join(trimmed);
    if direct.is_file() {
        return Some(direct);
    }

    let slashified = trimmed.replace('\\', "/");
    let slash_path = root_dir.join(&slashified);
    if slash_path.is_file() {
        return Some(slash_path);
    }

    if Path::new(&slashified).extension().is_none() {
        let with_extension = root_dir.join(format!("{slashified}.lua"));
        if with_extension.is_file() {
            return Some(with_extension);
        }
    }

    let dotted = trimmed.replace('\\', ".").replace('/', ".");
    let dotted_path = root_dir
        .join(dotted.replace('.', "/"))
        .with_extension("lua");
    if dotted_path.is_file() {
        return Some(dotted_path);
    }

    None
}

fn order_plugins(mut plugins: Vec<LoadedPlugin>) -> Vec<LoadedPlugin> {
    let lookup = plugins
        .iter()
        .map(|plugin| {
            (
                plugin.manifest.name.to_ascii_lowercase(),
                plugin.manifest.clone(),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut indegree = HashMap::<String, usize>::new();
    let mut edges = HashMap::<String, Vec<String>>::new();

    for plugin in &plugins {
        indegree.insert(plugin.manifest.name.to_ascii_lowercase(), 0);
    }

    for plugin in &plugins {
        let current = plugin.manifest.name.to_ascii_lowercase();
        for dependency in &plugin.manifest.depend {
            add_dependency_edge(&mut edges, &mut indegree, dependency, &current, &lookup);
        }
        for dependency in &plugin.manifest.softdepend {
            add_dependency_edge(&mut edges, &mut indegree, dependency, &current, &lookup);
        }
        for target in &plugin.manifest.loadbefore {
            add_dependency_edge(&mut edges, &mut indegree, &current, target, &lookup);
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(name, degree)| (*degree == 0).then_some(name.clone()))
        .collect::<Vec<_>>();
    sort_ready(&mut ready, &lookup);

    let mut ordered_names = Vec::with_capacity(plugins.len());
    while let Some(current) = ready.first().cloned() {
        ready.remove(0);
        ordered_names.push(current.clone());

        if let Some(targets) = edges.remove(&current) {
            for target in targets {
                if let Some(degree) = indegree.get_mut(&target) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        ready.push(target);
                    }
                }
            }
            sort_ready(&mut ready, &lookup);
        }
    }

    if ordered_names.len() != plugins.len() {
        let mut remaining = plugins
            .iter()
            .filter_map(|plugin| {
                let key = plugin.manifest.name.to_ascii_lowercase();
                (!ordered_names.iter().any(|name| name == &key)).then_some(plugin)
            })
            .collect::<Vec<_>>();
        remaining.sort_by(|left, right| {
            load_order_rank(left.manifest.load)
                .cmp(&load_order_rank(right.manifest.load))
                .then_with(|| left.manifest.name.cmp(&right.manifest.name))
        });
        warn!(
            "Plugin dependency cycle detected; falling back to deterministic order for: {}",
            remaining
                .iter()
                .map(|plugin| plugin.manifest.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        ordered_names.extend(
            remaining
                .iter()
                .map(|plugin| plugin.manifest.name.to_ascii_lowercase()),
        );
    }

    let mut by_name = plugins
        .drain(..)
        .map(|plugin| (plugin.manifest.name.to_ascii_lowercase(), plugin))
        .collect::<HashMap<_, _>>();
    ordered_names
        .into_iter()
        .filter_map(|name| by_name.remove(&name))
        .collect()
}

fn add_dependency_edge(
    edges: &mut HashMap<String, Vec<String>>,
    indegree: &mut HashMap<String, usize>,
    from: &str,
    to: &str,
    lookup: &HashMap<String, PluginManifest>,
) {
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    if from == to || !lookup.contains_key(&from) || !lookup.contains_key(&to) {
        return;
    }
    let targets = edges.entry(from).or_default();
    if targets.iter().any(|existing| existing == &to) {
        return;
    }
    targets.push(to.clone());
    if let Some(degree) = indegree.get_mut(&to) {
        *degree += 1;
    }
}

fn sort_ready(ready: &mut Vec<String>, lookup: &HashMap<String, PluginManifest>) {
    ready.sort_by(|left, right| {
        let left_rank = lookup
            .get(left)
            .map(|plugin| load_order_rank(plugin.load))
            .unwrap_or(1);
        let right_rank = lookup
            .get(right)
            .map(|plugin| load_order_rank(plugin.load))
            .unwrap_or(1);
        left_rank.cmp(&right_rank).then_with(|| left.cmp(right))
    });
    ready.dedup();
}

fn load_order_rank(order: PluginLoadOrder) -> u8 {
    match order {
        PluginLoadOrder::Startup => 0,
        PluginLoadOrder::PostWorld => 1,
    }
}

fn sanitize_plugin_dir_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn normalize_name_list(values: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    for value in values {
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if !normalized
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&value))
        {
            normalized.push(value);
        }
    }
    normalized
}

fn normalize_command_label(label: &str) -> String {
    label.trim().trim_start_matches('/').to_ascii_lowercase()
}

fn deserialize_aliases<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_list(deserializer)
}

fn deserialize_string_or_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrList {
        One(String),
        Many(Vec<String>),
    }

    match StringOrList::deserialize(deserializer)? {
        StringOrList::One(value) => Ok(vec![value]),
        StringOrList::Many(values) => Ok(values),
    }
}

fn deserialize_string_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringListValue {
        One(String),
        Many(Vec<String>),
    }

    let Some(value) = Option::<StringListValue>::deserialize(deserializer)? else {
        return Ok(Vec::new());
    };

    Ok(match value {
        StringListValue::One(value) => {
            if value.trim().is_empty() {
                Vec::new()
            } else {
                vec![value]
            }
        }
        StringListValue::Many(values) => values,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::SocketAddr;

    use mc_rs_command::{CommandDispatchError, CommandSender, PermissionState, SoftEnumSource};
    use uuid::Uuid;

    use super::*;
    use crate::commands::{build_command_system, ServerCommandRuntime, TitlePacketAction};

    struct PluginTestRuntime {
        feedback: Vec<String>,
        broadcasts: Vec<String>,
        plugin_names: Vec<String>,
    }

    impl PluginTestRuntime {
        fn new(plugin_names: Vec<String>) -> Self {
            Self {
                feedback: Vec::new(),
                broadcasts: Vec::new(),
                plugin_names,
            }
        }
    }

    impl CommandSender for PluginTestRuntime {
        fn sender_name(&self) -> &str {
            "Console"
        }

        fn sender_is_player(&self) -> bool {
            false
        }

        fn sender_position(&self) -> [f32; 3] {
            [0.0, 0.0, 0.0]
        }

        fn sender_entity_id(&self) -> Option<u64> {
            None
        }

        fn sender_is_op(&self) -> bool {
            true
        }

        fn sender_has_permission(&self, _permission: &str) -> bool {
            true
        }
    }

    impl SoftEnumSource for PluginTestRuntime {
        fn soft_enum_values(&self, _name: &str) -> Vec<String> {
            Vec::new()
        }
    }

    impl ServerCommandRuntime for PluginTestRuntime {
        fn sender_addr(&self) -> Option<SocketAddr> {
            None
        }

        fn send_feedback(&mut self, message: &str) {
            self.feedback.push(message.to_string());
        }

        fn send_message(&mut self, _addr: SocketAddr, _message: &str) {}

        fn broadcast_chat(&mut self, source: &str, message: &str) {
            self.broadcasts.push(format!("{source}:{message}"));
        }

        fn broadcast_action(&mut self, _source: &str, _message: &str) {}

        fn open_sender_menu(&mut self) {}

        fn show_sender_biome(&mut self) {}

        fn selector_entities(&self) -> Vec<mc_rs_command::SelectorEntity> {
            Vec::new()
        }

        fn random_index(&mut self, _upper: usize) -> usize {
            0
        }

        fn player_addr_by_entity(&self, _entity_id: u64) -> Option<SocketAddr> {
            None
        }

        fn teleport_player(&mut self, _addr: SocketAddr, _position: [f32; 3]) {}

        fn set_player_gamemode(&mut self, _addr: SocketAddr, _mode: i32) {}

        fn player_position(&self, _addr: SocketAddr) -> Option<[f32; 3]> {
            None
        }

        fn player_name(&self, _addr: SocketAddr) -> Option<String> {
            None
        }

        fn player_gamemode(&self, _addr: SocketAddr) -> Option<i32> {
            None
        }

        fn clear_inventory(&mut self, _addr: SocketAddr) {}

        fn give_item(
            &mut self,
            _addr: SocketAddr,
            _item: mc_rs_proto::packets::player::ItemStack,
        ) -> Result<(), String> {
            Ok(())
        }

        fn spawn_mob(&mut self, _mob_name: &str, _position: [f32; 3]) -> Result<u64, String> {
            Err("not supported".to_string())
        }

        fn kill_player(&mut self, _addr: SocketAddr) {}

        fn remove_entity(&mut self, _entity_id: u64) -> Result<(), String> {
            Err("not supported".to_string())
        }

        fn set_time(&mut self, _time: i32) {}

        fn current_time(&self) -> i32 {
            0
        }

        fn set_weather(&mut self, _rain: bool, _thunder: bool) {}

        fn add_player_xp(&mut self, _addr: SocketAddr, _amount: i32) -> Result<i32, String> {
            Ok(0)
        }

        fn apply_player_effect(
            &mut self,
            _addr: SocketAddr,
            _effect_id: i32,
            _duration_ticks: i32,
            _amplifier: u8,
        ) -> Result<(), String> {
            Ok(())
        }
        fn apply_held_enchant(
            &mut self,
            _addr: SocketAddr,
            _enchant_id: u8,
            _level: u8,
        ) -> Result<(), String> {
            Ok(())
        }
        fn spawn_particle(&mut self, _position: [f32; 3], _particle_name: &str) {}
        fn boss_show(&mut self, _title: &str, _health_percent: f32) {}
        fn boss_hide(&mut self) {}
        fn boss_set_title(&mut self, _title: &str) {}
        fn boss_set_health(&mut self, _health_percent: f32) {}
        fn scoreboard_set(&mut self, _objective: &str, _player: &str, _score: i32) {}

        fn set_difficulty(&mut self, _difficulty: i32) {}

        fn current_difficulty(&self) -> i32 {
            0
        }

        fn set_default_gamemode(&mut self, _gamemode: i32) {}

        fn current_default_gamemode(&self) -> i32 {
            0
        }

        fn stop_server(&mut self) {}

        fn save_world(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn set_auto_save(&mut self, _enabled: bool) {}

        fn auto_save_enabled(&self) -> bool {
            true
        }

        fn kick(&mut self, _addr: SocketAddr, _reason: &str) {}

        fn transfer(&mut self, _addr: SocketAddr, _host: &str, _port: u16) {}

        fn set_player_spawn(&mut self, _addr: SocketAddr, _pos: [f32; 3]) -> Result<(), String> {
            Ok(())
        }

        fn player_spawn(&self, _addr: SocketAddr) -> Option<[f32; 3]> {
            None
        }

        fn set_world_spawn(&mut self, _pos: [f32; 3]) {}

        fn world_spawn(&self) -> [f32; 3] {
            [0.0, 0.0, 0.0]
        }

        fn op(&mut self, _name: &str) {}

        fn deop(&mut self, _name: &str) {}

        fn list_ops(&self) -> Vec<String> {
            Vec::new()
        }

        fn set_whitelist_enabled(&mut self, _enabled: bool) {}

        fn whitelist_enabled(&self) -> bool {
            false
        }

        fn whitelist_entries(&self) -> Vec<String> {
            Vec::new()
        }

        fn whitelist_add(&mut self, _name: &str) {}

        fn whitelist_remove(&mut self, _name: &str) {}

        fn ban_name(&mut self, _name: &str) {}

        fn pardon_name(&mut self, _name: &str) {}

        fn banned_names(&self) -> Vec<String> {
            Vec::new()
        }

        fn ban_ip(&mut self, _ip: &str) {}

        fn pardon_ip(&mut self, _ip: &str) {}

        fn banned_ips(&self) -> Vec<String> {
            Vec::new()
        }

        fn player_ip(&self, _addr: SocketAddr) -> Option<String> {
            None
        }

        fn send_title(&mut self, _addr: SocketAddr, _action: TitlePacketAction) {}

        fn sync_available_commands_for_all(&mut self) {}

        fn server_motd(&self) -> &str {
            "Test"
        }

        fn world_name(&self) -> &str {
            "world"
        }

        fn world_seed(&self) -> u64 {
            0
        }

        fn resolve_block_name(&self, _name: &str) -> Option<u32> {
            None
        }

        fn set_world_block(&mut self, _x: i32, _y: i32, _z: i32, _block_id: u32) -> bool {
            false
        }

        fn world_block_at(&self, _x: i32, _y: i32, _z: i32) -> u32 {
            0
        }

        fn gamerule_list(&self) -> Vec<(String, crate::game_rules::GameRuleValue)> {
            Vec::new()
        }

        fn gamerule_get(&self, _name: &str) -> Option<crate::game_rules::GameRuleValue> {
            None
        }

        fn gamerule_set(
            &mut self,
            _name: &str,
            _value: crate::game_rules::GameRuleValue,
        ) -> Result<(), String> {
            Ok(())
        }

        fn tellraw_send(&mut self, _addr: SocketAddr, _payload: &[u8]) {}

        fn play_sound(
            &mut self,
            _targets: &[SocketAddr],
            _sound: &str,
            _position: [f32; 3],
            _volume: f32,
            _pitch: f32,
        ) {
        }

        fn stop_sound(&mut self, _targets: &[SocketAddr], _sound: Option<&str>) {}

        fn replace_player_slot(
            &mut self,
            _addr: SocketAddr,
            _inv_key: crate::inventory_manager::InvKey,
            _slot_index: usize,
            _item: mc_rs_proto::packets::player::ItemStack,
        ) -> Result<(), String> {
            Ok(())
        }

        fn player_tag_add(&mut self, _addr: SocketAddr, _tag: &str) -> bool {
            false
        }

        fn player_tag_remove(&mut self, _addr: SocketAddr, _tag: &str) -> bool {
            false
        }

        fn player_tag_list(&self, _addr: SocketAddr) -> Vec<String> {
            Vec::new()
        }

        fn online_players(&self) -> usize {
            0
        }

        fn max_players(&self) -> u32 {
            0
        }

        fn execute_plugin_command(
            &mut self,
            _plugin_name: &str,
            _command_name: &str,
            _invocation: &CommandInvocation,
        ) -> Result<(), CommandDispatchError> {
            Err(CommandDispatchError::Message(
                "not wired in this test runtime".to_string(),
            ))
        }

        fn plugin_names(&self) -> Vec<String> {
            self.plugin_names.clone()
        }

        fn visible_command_names(&self) -> Vec<String> {
            Vec::new()
        }
    }

    #[test]
    fn loads_and_orders_plugin_manifests() {
        let root = make_temp_dir();
        let plugins_dir = root.join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        write_plugin(
            &plugins_dir,
            "CoreLib",
            r#"
name: CoreLib
version: 1.0.0
main: core.Main
api: 1.0.0
load: STARTUP
"#,
            None,
            None,
        );
        write_plugin(
            &plugins_dir,
            "ChatTools",
            r#"
name: ChatTools
version: 1.0.0
main: chat.Main
api: 1.0.0
depend: [CoreLib]
"#,
            None,
            None,
        );
        write_plugin(
            &plugins_dir,
            "Welcome",
            r#"
name: Welcome
version: 1.0.0
main: welcome.Main
api: 1.0.0
loadbefore: [ChatTools]
"#,
            None,
            None,
        );

        let manager = PluginManager::load_from_dir(&plugins_dir);
        assert_eq!(
            manager
                .plugins()
                .iter()
                .map(|plugin| plugin.manifest.name.clone())
                .collect::<Vec<_>>(),
            vec![
                "CoreLib".to_string(),
                "Welcome".to_string(),
                "ChatTools".to_string()
            ]
        );
    }

    #[test]
    fn registers_permissions_from_manifest() {
        let root = make_temp_dir();
        let plugins_dir = root.join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        write_plugin(
            &plugins_dir,
            "Perms",
            r#"
name: Perms
version: 1.0.0
main: main.lua
api: 1.0.0
permissions:
  perms.admin:
    default: op
    children:
      perms.command.use: true
  perms.command.use:
    default: false
"#,
            Some(""),
            None,
        );

        let manager = PluginManager::load_from_dir(&plugins_dir);
        let mut registry = PermissionRegistry::new();
        manager.register_permissions(&mut registry);

        let state = PermissionState {
            explicit: HashMap::new(),
            is_op: true,
        };
        assert!(registry.has_permission(&state, "perms.admin"));
        assert!(registry.has_permission(&state, "perms.command.use"));
    }

    #[test]
    fn enables_lua_plugin_and_registers_commands() {
        let root = make_temp_dir();
        let plugins_dir = root.join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        write_plugin(
            &plugins_dir,
            "ExampleLua",
            r#"
name: ExampleLua
version: 1.0.0
main: main.lua
api: 1.0.0
commands:
  hello:
    description: "Say hello"
    usage: "/hello [name]"
    aliases: [hi]
    permission: "examplelua.command.hello"
permissions:
  examplelua.command.hello:
    default: true
"#,
            Some(
                r#"
save_default_config()

local config = load_config() or {}
local prefix = config.prefix or "[ExampleLua]"

function on_load()
  log("loaded")
end

function on_enable()
  log("enabled")
end

function on_disable()
  log("disabled")
end

register_command("hello", function(sender, args)
  local target = args[1] or sender.name
  broadcast("Lua says hi to " .. target)
  return prefix .. " Hello, " .. target .. "!"
end)
"#,
            ),
            Some("prefix: \"[Cfg]\""),
        );

        let mut manager = PluginManager::load_from_dir(&plugins_dir);
        let mut system = build_command_system();
        manager.register_permissions(&mut system.permissions);
        manager.enable_plugins(PluginLoadOrder::PostWorld, &mut system);

        assert_eq!(manager.plugin_names(), vec!["ExampleLua".to_string()]);
        assert!(system.map.definition("hello").is_some());
        assert!(system.map.definition("hi").is_some());

        let invocation = CommandInvocation {
            original: "/hello Karim".to_string(),
            label: "hello".to_string(),
            command_name: "hello".to_string(),
            args: vec!["Karim".to_string()],
            raw_args: "Karim".to_string(),
        };

        let mut runtime = PluginTestRuntime::new(manager.plugin_names());
        manager
            .execute_command("ExampleLua", "hello", &invocation, &mut runtime)
            .unwrap();

        assert_eq!(runtime.feedback, vec!["[Cfg] Hello, Karim!".to_string()]);
        assert_eq!(
            runtime.broadcasts,
            vec!["ExampleLua:Lua says hi to Karim".to_string()]
        );
    }

    #[test]
    fn disable_all_unregisters_plugin_commands() {
        let root = make_temp_dir();
        let plugins_dir = root.join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        write_plugin(
            &plugins_dir,
            "DisableMe",
            r#"
name: DisableMe
version: 1.0.0
main: main.lua
api: 1.0.0
commands:
  ping:
    description: "Ping"
"#,
            Some(
                r#"
register_command("ping", function()
  return "pong"
end)
"#,
            ),
            None,
        );

        let mut manager = PluginManager::load_from_dir(&plugins_dir);
        let mut system = build_command_system();
        manager.enable_plugins(PluginLoadOrder::PostWorld, &mut system);
        assert!(system.map.definition("ping").is_some());

        manager.disable_all(&mut system);
        assert!(system.map.definition("ping").is_none());
        assert!(manager.plugin_names().is_empty());
    }

    fn write_plugin(
        plugins_dir: &Path,
        dir_name: &str,
        manifest: &str,
        main_lua: Option<&str>,
        config_yml: Option<&str>,
    ) {
        let plugin_dir = plugins_dir.join(dir_name);
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("plugin.yml"), manifest.trim()).unwrap();
        if let Some(main_lua) = main_lua {
            fs::write(plugin_dir.join("main.lua"), main_lua.trim()).unwrap();
        }
        if let Some(config_yml) = config_yml {
            fs::write(plugin_dir.join("config.yml"), config_yml.trim()).unwrap();
        }
    }

    fn make_temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("mc-rs-plugin-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
