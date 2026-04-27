use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use mc_rs_command::{
    resolve_target_token_with_index, CommandDefinition, CommandDispatchError, CommandInvocation,
    CommandOverload, CommandParameter, CommandSender, ParamType, PermissionDefault,
    PermissionDefinition, PermissionRegistry, PermissionState, RegistrationError, SelectorEntity,
    SelectorError, SoftEnumSource, VisibleCommand, VisibleCommandOverload, VisibleCommandParameter,
    VisibleParamType,
};
use mc_rs_proto::packets::login::{Disconnect, DisconnectReason};
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::{
    ItemStack, PlayerList, PlayerListAdd, SetTitle, Text, Transfer,
};
use mc_rs_proto::packets::world::{
    AvailableCommands, CmdEntry, CmdOverload, CmdParam, CmdParamType, SetDifficulty,
    SetSpawnPosition,
};
use mc_rs_raknet::protocol::datagram::Reliability;
use mc_rs_raknet::{RakNetPeer, RakNetServer};
use tracing::info;

use crate::connection::Connection;
use crate::inventory;
use crate::item_entities::{ItemEntityManager, PendingItemEntitySpawn};
use crate::mob_entities::{MobEntityManager, MobKind};
use crate::player_data;
use crate::player_registry::PlayerRegistry;
use crate::plugin::PluginManager;
use crate::server_state::{normalize_name, ServerState};
use crate::world::biome;
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;
use crate::world::terrain_generator;
use crate::world::tick::{encode_set_time, WorldState};

pub const SERVER_VERSION: &str = "mc-rs 0.1.0";

#[derive(Debug, Clone)]
pub enum TitlePacketAction {
    Clear,
    Reset,
    Title(String),
    Subtitle(String),
    Actionbar(String),
    Times {
        fade_in: i32,
        stay: i32,
        fade_out: i32,
    },
}

pub trait ServerCommandRuntime: CommandSender + SoftEnumSource {
    fn sender_addr(&self) -> Option<SocketAddr>;
    fn send_feedback(&mut self, message: &str);
    fn send_message(&mut self, addr: SocketAddr, message: &str);
    fn broadcast_chat(&mut self, source: &str, message: &str);
    fn broadcast_action(&mut self, source: &str, message: &str);
    fn open_sender_menu(&mut self);
    fn show_sender_biome(&mut self);
    fn selector_entities(&self) -> Vec<SelectorEntity>;
    fn random_index(&mut self, upper: usize) -> usize;
    fn player_addr_by_entity(&self, entity_id: u64) -> Option<SocketAddr>;
    fn teleport_player(&mut self, addr: SocketAddr, position: [f32; 3]);
    fn set_player_gamemode(&mut self, addr: SocketAddr, mode: i32);
    fn player_position(&self, addr: SocketAddr) -> Option<[f32; 3]>;
    fn player_name(&self, addr: SocketAddr) -> Option<String>;
    fn player_gamemode(&self, addr: SocketAddr) -> Option<i32>;
    fn clear_inventory(&mut self, addr: SocketAddr);
    fn give_item(&mut self, addr: SocketAddr, item: ItemStack) -> Result<(), String>;
    fn spawn_mob(&mut self, mob_name: &str, position: [f32; 3]) -> Result<u64, String>;
    fn kill_player(&mut self, addr: SocketAddr);
    fn remove_entity(&mut self, entity_id: u64) -> Result<(), String>;
    fn set_time(&mut self, time: i32);
    fn current_time(&self) -> i32;
    fn set_weather(&mut self, rain: bool, thunder: bool);
    /// Ajoute `amount` XP (positif) ou retire (négatif) au joueur. Retourne
    /// le nouveau level après application.
    fn add_player_xp(&mut self, addr: SocketAddr, amount: i32) -> Result<i32, String>;
    /// Applique un effet potion sur le joueur. `effect_id` = PMMP MobEffectIds,
    /// `duration_ticks` (20/s), `amplifier` (0 = I, 1 = II, ...).
    fn apply_player_effect(
        &mut self,
        addr: SocketAddr,
        effect_id: i32,
        duration_ticks: i32,
        amplifier: u8,
    ) -> Result<(), String>;
    /// Spawn une particule à la position donnée pour tous les joueurs.
    fn spawn_particle(&mut self, position: [f32; 3], particle_name: &str);
    /// Affiche un boss bar serveur-wide avec titre + health (0..1).
    fn boss_show(&mut self, title: &str, health_percent: f32);
    fn boss_hide(&mut self);
    fn boss_set_title(&mut self, title: &str);
    fn boss_set_health(&mut self, health_percent: f32);
    /// Set un score sur un objectif (créé si nécessaire), display = sidebar.
    fn scoreboard_set(&mut self, objective: &str, player: &str, score: i32);
    /// Ajoute un enchantement au held item du joueur (NBT `ench` list).
    fn apply_held_enchant(
        &mut self,
        addr: SocketAddr,
        enchant_id: u8,
        level: u8,
    ) -> Result<(), String>;
    fn set_difficulty(&mut self, difficulty: i32);
    fn current_difficulty(&self) -> i32;
    fn set_default_gamemode(&mut self, gamemode: i32);
    fn current_default_gamemode(&self) -> i32;
    fn stop_server(&mut self);
    fn save_world(&mut self) -> Result<(), String>;
    fn set_auto_save(&mut self, enabled: bool);
    fn auto_save_enabled(&self) -> bool;
    fn kick(&mut self, addr: SocketAddr, reason: &str);
    fn transfer(&mut self, addr: SocketAddr, host: &str, port: u16);
    fn set_player_spawn(&mut self, addr: SocketAddr, pos: [f32; 3]) -> Result<(), String>;
    fn player_spawn(&self, addr: SocketAddr) -> Option<[f32; 3]>;
    fn set_world_spawn(&mut self, pos: [f32; 3]);
    fn world_spawn(&self) -> [f32; 3];
    fn op(&mut self, name: &str);
    fn deop(&mut self, name: &str);
    fn list_ops(&self) -> Vec<String>;
    fn set_whitelist_enabled(&mut self, enabled: bool);
    fn whitelist_enabled(&self) -> bool;
    fn whitelist_entries(&self) -> Vec<String>;
    fn whitelist_add(&mut self, name: &str);
    fn whitelist_remove(&mut self, name: &str);
    fn ban_name(&mut self, name: &str);
    fn pardon_name(&mut self, name: &str);
    fn banned_names(&self) -> Vec<String>;
    fn ban_ip(&mut self, ip: &str);
    fn pardon_ip(&mut self, ip: &str);
    fn banned_ips(&self) -> Vec<String>;
    fn player_ip(&self, addr: SocketAddr) -> Option<String>;
    fn send_title(&mut self, addr: SocketAddr, action: TitlePacketAction);
    fn sync_available_commands_for_all(&mut self);
    fn server_motd(&self) -> &str;
    fn world_name(&self) -> &str;
    fn world_seed(&self) -> u64;
    fn online_players(&self) -> usize;
    fn max_players(&self) -> u32;
    fn execute_plugin_command(
        &mut self,
        plugin_name: &str,
        command_name: &str,
        invocation: &CommandInvocation,
    ) -> Result<(), CommandDispatchError>;
    fn plugin_names(&self) -> Vec<String>;
    fn visible_command_names(&self) -> Vec<String>;
}

type ServerCommandHandler = dyn for<'runtime, 'invocation> Fn(
        &'runtime mut (dyn ServerCommandRuntime + 'runtime),
        &'invocation CommandInvocation,
    ) -> Result<(), CommandDispatchError>
    + Send
    + Sync;

struct ServerCommandEntry {
    definition: CommandDefinition,
    handler: Box<ServerCommandHandler>,
}

pub struct ServerCommandMap {
    commands: Vec<ServerCommandEntry>,
    name_to_index: HashMap<String, usize>,
}

impl Default for ServerCommandMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCommandMap {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            name_to_index: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        mut definition: CommandDefinition,
        handler: impl for<'runtime, 'invocation> Fn(
                &'runtime mut (dyn ServerCommandRuntime + 'runtime),
                &'invocation CommandInvocation,
            ) -> Result<(), CommandDispatchError>
            + Send
            + Sync
            + 'static,
    ) -> Result<(), RegistrationError> {
        let name = normalize_command_label(&definition.name);
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
            .map(|alias| normalize_command_label(&alias))
            .collect();
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

        self.commands.push(ServerCommandEntry {
            definition,
            handler: Box::new(handler),
        });
        Ok(())
    }

    pub fn definitions(&self) -> impl Iterator<Item = &CommandDefinition> {
        self.commands.iter().map(|entry| &entry.definition)
    }

    pub fn definition(&self, name: &str) -> Option<&CommandDefinition> {
        let index = self
            .name_to_index
            .get(&normalize_command_label(name))
            .copied()?;
        self.commands.get(index).map(|entry| &entry.definition)
    }

    pub fn unregister(&mut self, name: &str) -> Option<CommandDefinition> {
        let canonical = normalize_command_label(name);
        let index = *self.name_to_index.get(&canonical)?;
        let removed = self.commands.remove(index);
        self.rebuild_name_index();
        Some(removed.definition)
    }

    pub fn unregister_owner(&mut self, owner: &str) -> Vec<CommandDefinition> {
        let owner = owner.to_ascii_lowercase();
        let mut removed = Vec::new();
        self.commands.retain(|entry| {
            let should_remove = entry
                .definition
                .owner
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(&owner))
                .unwrap_or(false);
            if should_remove {
                removed.push(entry.definition.clone());
            }
            !should_remove
        });
        if !removed.is_empty() {
            self.rebuild_name_index();
        }
        removed
    }

    pub fn dispatch(
        &self,
        runtime: &mut (dyn ServerCommandRuntime + '_),
        command_line: &str,
    ) -> Result<(), CommandDispatchError> {
        let parsed =
            mc_rs_command::parse_command_line(command_line).map_err(CommandDispatchError::Parse)?;
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

pub struct ServerCommandSystem {
    pub permissions: PermissionRegistry,
    pub map: ServerCommandMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Player(SocketAddr),
    Console,
}

pub struct ExecutionContext<'a> {
    pub source: CommandSource,
    pub command_system: &'a ServerCommandSystem,
    pub connections: &'a mut HashMap<SocketAddr, Connection>,
    pub peers: &'a mut HashMap<SocketAddr, RakNetPeer>,
    pub raknet: &'a mut RakNetServer,
    pub registry: &'a mut PlayerRegistry,
    pub item_entities: &'a mut ItemEntityManager,
    pub mob_entities: &'a mut MobEntityManager,
    pub world_state: &'a mut WorldState,
    pub server_state: &'a mut ServerState,
    pub plugin_manager: &'a Arc<Mutex<PluginManager>>,
    pub chunk_cache: &'a Arc<Mutex<ChunkCache>>,
    pub should_stop: &'a mut bool,
}

fn register_command<H>(
    permissions: &mut PermissionRegistry,
    map: &mut ServerCommandMap,
    definition: CommandDefinition,
    default_permission: PermissionDefault,
    handler: H,
) where
    H: for<'runtime, 'invocation> Fn(
            &'runtime mut (dyn ServerCommandRuntime + 'runtime),
            &'invocation CommandInvocation,
        ) -> Result<(), CommandDispatchError>
        + Send
        + Sync
        + 'static,
{
    if let Some(permission) = definition.permissions.first() {
        permissions.register(
            permission.clone(),
            PermissionDefinition::new(default_permission),
        );
    }
    map.register(definition, handler).unwrap();
}

fn normalize_command_label(label: &str) -> String {
    label.trim().trim_start_matches('/').to_ascii_lowercase()
}

pub fn dispatch_command_line(
    source: CommandSource,
    line: &str,
    command_system: &ServerCommandSystem,
    connections: &mut HashMap<SocketAddr, Connection>,
    peers: &mut HashMap<SocketAddr, RakNetPeer>,
    raknet: &mut RakNetServer,
    registry: &mut PlayerRegistry,
    item_entities: &mut ItemEntityManager,
    mob_entities: &mut MobEntityManager,
    world_state: &mut WorldState,
    server_state: &mut ServerState,
    plugin_manager: &Arc<Mutex<PluginManager>>,
    chunk_cache: &Arc<Mutex<ChunkCache>>,
    should_stop: &mut bool,
) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    let mut runtime = ExecutionContext::new(
        source,
        command_system,
        connections,
        peers,
        raknet,
        registry,
        item_entities,
        mob_entities,
        world_state,
        server_state,
        plugin_manager,
        chunk_cache,
        should_stop,
    );
    if let Err(error) = command_system.map.dispatch(&mut runtime, trimmed) {
        runtime.send_feedback(&error.to_string());
    }
}

impl ExecutionContext<'_> {
    pub fn new<'a>(
        source: CommandSource,
        command_system: &'a ServerCommandSystem,
        connections: &'a mut HashMap<SocketAddr, Connection>,
        peers: &'a mut HashMap<SocketAddr, RakNetPeer>,
        raknet: &'a mut RakNetServer,
        registry: &'a mut PlayerRegistry,
        item_entities: &'a mut ItemEntityManager,
        mob_entities: &'a mut MobEntityManager,
        world_state: &'a mut WorldState,
        server_state: &'a mut ServerState,
        plugin_manager: &'a Arc<Mutex<PluginManager>>,
        chunk_cache: &'a Arc<Mutex<ChunkCache>>,
        should_stop: &'a mut bool,
    ) -> ExecutionContext<'a> {
        ExecutionContext {
            source,
            command_system,
            connections,
            peers,
            raknet,
            registry,
            item_entities,
            mob_entities,
            world_state,
            server_state,
            plugin_manager,
            chunk_cache,
            should_stop,
        }
    }

    fn source_addr(&self) -> Option<SocketAddr> {
        match self.source {
            CommandSource::Player(addr) => Some(addr),
            CommandSource::Console => None,
        }
    }

    fn online_player_names(&self) -> Vec<String> {
        let mut values = self
            .connections
            .values()
            .filter(|connection| connection.is_in_game())
            .filter_map(|connection| connection.display_name.clone())
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        values
    }

    fn permission_state(&self) -> PermissionState {
        let is_op = match self.source {
            CommandSource::Player(addr) => self
                .connections
                .get(&addr)
                .map(|connection| connection.is_op)
                .unwrap_or(false),
            CommandSource::Console => true,
        };
        PermissionState {
            explicit: HashMap::new(),
            is_op,
        }
    }

    fn send_prepared(&mut self, addr: SocketAddr, packet: Vec<u8>) {
        if let Some(connection) = self.connections.get_mut(&addr) {
            let prepared = connection.prepare_for_send(packet);
            self.raknet
                .send_to_session(&addr, prepared, Reliability::ReliableOrdered, true);
        }
    }

    fn send_compressed(&mut self, addr: SocketAddr, packet_id: u32, payload: &[u8]) {
        if let Some(connection) = self.connections.get_mut(&addr) {
            let packet = connection.encode_compressed_packet(packet_id, payload);
            let prepared = connection.prepare_for_send(packet);
            self.raknet
                .send_to_session(&addr, prepared, Reliability::ReliableOrdered, true);
        }
    }

    fn broadcast_compressed(&mut self, packet_id: u32, payload: &[u8]) {
        let addrs = self
            .connections
            .iter()
            .filter_map(|(addr, connection)| connection.is_in_game().then_some(*addr))
            .collect::<Vec<_>>();

        for addr in addrs {
            self.send_compressed(addr, packet_id, payload);
        }
    }

    fn spawn_world_item_entity(&mut self, spawn: PendingItemEntitySpawn) {
        let entity = self.item_entities.spawn(spawn);
        self.broadcast_compressed(packet_id::ADD_ITEM_ACTOR, &entity.add_actor_packet());
    }

    fn update_connection_permissions(&mut self, name: &str) {
        let normalized = normalize_name(name);
        for connection in self.connections.values_mut() {
            let should_be_op = connection
                .display_name
                .as_deref()
                .is_some_and(|display_name| normalize_name(display_name) == normalized)
                && self.server_state.is_op(name);
            if should_be_op {
                connection.is_op = true;
            } else if connection
                .display_name
                .as_deref()
                .is_some_and(|display_name| normalize_name(display_name) == normalized)
            {
                connection.is_op = false;
            }
        }
    }

    fn visible_commands_for_addr(&self, addr: SocketAddr) -> Option<Vec<VisibleCommand>> {
        let viewer = ViewerContext {
            addr,
            permissions: &self.command_system.permissions,
            connections: &self.connections,
        };
        Some(
            self.command_system
                .map
                .definitions()
                .filter(|definition| {
                    definition.permissions.is_empty()
                        || definition
                            .permissions
                            .iter()
                            .any(|permission| viewer.sender_has_permission(permission))
                })
                .map(|definition| VisibleCommand {
                    name: definition.name.clone(),
                    description: definition.description.clone(),
                    aliases: definition.aliases.clone(),
                    overloads: definition
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
                                        ParamType::Json => VisibleParamType::Basic(70),
                                        ParamType::Command => VisibleParamType::Basic(67),
                                        ParamType::HardEnum { name, values } => {
                                            VisibleParamType::HardEnum {
                                                name: name.clone(),
                                                values: values.clone(),
                                            }
                                        }
                                        ParamType::SoftEnum { name } => {
                                            VisibleParamType::SoftEnum {
                                                name: name.clone(),
                                                values: viewer.soft_enum_values(name),
                                            }
                                        }
                                    },
                                    optional: parameter.optional,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        )
    }

    fn sync_available_commands_for_addr(&mut self, addr: SocketAddr) {
        let Some(visible) = self.visible_commands_for_addr(addr) else {
            return;
        };
        let mut entries: Vec<CmdEntry> = Vec::new();
        for command in &visible {
            let overloads = command
                .overloads
                .iter()
                .map(|overload| CmdOverload {
                    params: overload
                        .parameters
                        .iter()
                        .map(|parameter| CmdParam {
                            name: parameter.name.clone(),
                            param_type: match &parameter.param_type {
                                VisibleParamType::Basic(type_id) => CmdParamType::Basic(*type_id),
                                VisibleParamType::HardEnum { name, values } => {
                                    CmdParamType::HardEnum {
                                        name: name.clone(),
                                        values: values.clone(),
                                    }
                                }
                                VisibleParamType::SoftEnum { name, values } => {
                                    CmdParamType::SoftEnum {
                                        name: name.clone(),
                                        values: values.clone(),
                                    }
                                }
                            },
                            optional: parameter.optional,
                        })
                        .collect(),
                })
                .collect();
            entries.push(CmdEntry {
                name: command.name.clone(),
                description: command.description.clone(),
                aliases: command.aliases.clone(),
                overloads,
            });
        }
        let packet = AvailableCommands::encode_rich(&entries);
        self.send_compressed(addr, packet_id::AVAILABLE_COMMANDS, &packet);
    }
}

struct ViewerContext<'a> {
    addr: SocketAddr,
    permissions: &'a PermissionRegistry,
    connections: &'a HashMap<SocketAddr, Connection>,
}

impl CommandSender for ViewerContext<'_> {
    fn sender_name(&self) -> &str {
        self.connections
            .get(&self.addr)
            .and_then(|connection| connection.display_name.as_deref())
            .unwrap_or("Player")
    }

    fn sender_is_player(&self) -> bool {
        true
    }

    fn sender_position(&self) -> [f32; 3] {
        self.connections
            .get(&self.addr)
            .map(|connection| connection.position)
            .unwrap_or([0.0, 0.0, 0.0])
    }

    fn sender_entity_id(&self) -> Option<u64> {
        self.connections
            .get(&self.addr)
            .map(|connection| connection.entity_runtime_id)
    }

    fn sender_is_op(&self) -> bool {
        self.connections
            .get(&self.addr)
            .map(|connection| connection.is_op)
            .unwrap_or(false)
    }

    fn sender_has_permission(&self, permission: &str) -> bool {
        self.permissions.has_permission(
            &PermissionState {
                explicit: HashMap::new(),
                is_op: self.sender_is_op(),
            },
            permission,
        )
    }
}

impl SoftEnumSource for ViewerContext<'_> {
    fn soft_enum_values(&self, name: &str) -> Vec<String> {
        if name.eq_ignore_ascii_case("online_players") {
            let mut values = self
                .connections
                .values()
                .filter(|connection| connection.is_in_game())
                .filter_map(|connection| connection.display_name.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            values
        } else {
            Vec::new()
        }
    }
}

impl CommandSender for ExecutionContext<'_> {
    fn sender_name(&self) -> &str {
        match self.source {
            CommandSource::Player(addr) => self
                .connections
                .get(&addr)
                .and_then(|connection| connection.display_name.as_deref())
                .unwrap_or("Player"),
            CommandSource::Console => "Console",
        }
    }

    fn sender_is_player(&self) -> bool {
        matches!(self.source, CommandSource::Player(_))
    }

    fn sender_position(&self) -> [f32; 3] {
        self.source_addr()
            .and_then(|addr| {
                self.connections
                    .get(&addr)
                    .map(|connection| connection.position)
            })
            .unwrap_or([0.0, 0.0, 0.0])
    }

    fn sender_entity_id(&self) -> Option<u64> {
        self.source_addr().and_then(|addr| {
            self.connections
                .get(&addr)
                .map(|connection| connection.entity_runtime_id)
        })
    }

    fn sender_is_op(&self) -> bool {
        match self.source {
            CommandSource::Player(addr) => self
                .connections
                .get(&addr)
                .map(|connection| connection.is_op)
                .unwrap_or(false),
            CommandSource::Console => true,
        }
    }

    fn sender_has_permission(&self, permission: &str) -> bool {
        if matches!(self.source, CommandSource::Console) {
            return true;
        }
        self.command_system
            .permissions
            .has_permission(&self.permission_state(), permission)
    }
}

impl SoftEnumSource for ExecutionContext<'_> {
    fn soft_enum_values(&self, name: &str) -> Vec<String> {
        if name.eq_ignore_ascii_case("online_players") {
            self.online_player_names()
        } else {
            Vec::new()
        }
    }
}

impl ExecutionContext<'_> {
    fn save_player_state(&self, addr: SocketAddr) {
        if let Some(connection) = self.connections.get(&addr) {
            if let Some(ref xuid) = connection.xuid {
                let save = player_data::PlayerSaveData::from_runtime(
                    connection.position,
                    [connection.yaw, connection.pitch],
                    connection.gamemode,
                    20.0,
                    20.0,
                    connection.spawn_position,
                    &connection.inventory,
                );
                let _ = player_data::save_player(xuid, &save);
            }
        }
    }

    fn remove_player_from_world(
        &mut self,
        addr: SocketAddr,
        send_disconnect: Option<(DisconnectReason, String)>,
    ) {
        if let Some((reason, message)) = send_disconnect {
            if self.connections.contains_key(&addr) {
                let disconnect = Disconnect {
                    reason,
                    message: Some(message),
                }
                .encode();
                self.send_compressed(addr, packet_id::DISCONNECT, &disconnect);
            }
        }

        self.save_player_state(addr);

        if let Some(player_info) = self.registry.remove(&addr) {
            let remove_entity = mc_rs_proto::packets::player::RemoveEntity {
                entity_unique_id: player_info.entity_id,
            }
            .encode();

            let player_list_remove = PlayerList {
                action: 1,
                entries: vec![PlayerListAdd {
                    uuid: player_info.uuid,
                    entity_id: player_info.entity_id,
                    username: String::new(),
                    xuid: String::new(),
                    platform_chat_id: String::new(),
                    build_platform: 0,
                    is_teacher: false,
                    is_host: false,
                    is_subclient: false,
                }],
            }
            .encode();

            let other_addrs = self
                .connections
                .iter()
                .filter_map(|(other_addr, connection)| {
                    (*other_addr != addr && connection.is_in_game()).then_some(*other_addr)
                })
                .collect::<Vec<_>>();
            for other_addr in other_addrs {
                self.send_compressed(other_addr, packet_id::REMOVE_ACTOR, &remove_entity);
                self.send_compressed(other_addr, packet_id::PLAYER_LIST, &player_list_remove);
            }
        }

        self.connections.remove(&addr);
        self.peers.remove(&addr);
    }
}

impl ServerCommandRuntime for ExecutionContext<'_> {
    fn sender_addr(&self) -> Option<SocketAddr> {
        self.source_addr()
    }

    fn send_feedback(&mut self, message: &str) {
        if let Some(addr) = self.source_addr() {
            self.send_message(addr, message);
        } else {
            info!("[CONSOLE] {message}");
        }
    }

    fn send_message(&mut self, addr: SocketAddr, message: &str) {
        let packet = self
            .connections
            .get(&addr)
            .map(|connection| connection.encode_system_message(message))
            .unwrap_or_else(|| Text::system(message));
        self.send_prepared(addr, packet);
    }

    fn broadcast_chat(&mut self, source: &str, message: &str) {
        let packet = Text::chat(source, message, "");
        self.broadcast_compressed(packet_id::TEXT, &packet);
    }

    fn broadcast_action(&mut self, source: &str, message: &str) {
        let formatted = format!("* {} {}", source, message);
        let packet = Text::system(&formatted);
        self.broadcast_compressed(packet_id::TEXT, &packet);
    }

    fn open_sender_menu(&mut self) {
        // Hub menu système (compass) retiré. Stub silencieux.
        self.send_feedback("Hub menu désactivé.");
    }

    fn show_sender_biome(&mut self) {
        let Some(position) = self.source_addr().and_then(|addr| {
            self.connections
                .get(&addr)
                .map(|connection| connection.position)
        }) else {
            self.send_feedback("Console must target a player to inspect a biome.");
            return;
        };
        let world_x = position[0].floor() as i32;
        let world_z = position[2].floor() as i32;
        let debug =
            terrain_generator::get_biome_debug_info(world_x, world_z, self.server_state.world_seed);
        let biome_def = biome::get_biome(debug.biome_id);
        self.send_feedback(&format!(
            "Biome: {} (id={}) | temp={:.3} rain={:.3} | surface_y={} | terrain={:.0}..{:.0} | chunk=({}, {})",
            biome::biome_name(debug.biome_id),
            debug.biome_id,
            debug.temperature,
            debug.rainfall,
            debug.surface_y,
            biome_def.min_elevation,
            biome_def.max_elevation,
            world_x.div_euclid(16),
            world_z.div_euclid(16),
        ));
    }

    fn selector_entities(&self) -> Vec<SelectorEntity> {
        let mut entities = self
            .registry
            .players
            .values()
            .map(|player| SelectorEntity {
                id: player.entity_id as u64,
                name: Some(player.name.clone()),
                entity_type: "player".to_string(),
                position: player.position,
                gamemode: Some(player.gamemode),
            })
            .collect::<Vec<_>>();
        entities.extend(self.item_entities.all().map(|entity| SelectorEntity {
            id: entity.entity_runtime_id,
            name: None,
            entity_type: "item".to_string(),
            position: entity.position,
            gamemode: None,
        }));
        entities.extend(self.mob_entities.all().map(|entity| SelectorEntity {
            id: entity.base.entity_runtime_id,
            name: Some(entity.base.display_name.clone()),
            entity_type: entity.base.selector_type.clone(),
            position: entity.base.position,
            gamemode: None,
        }));
        entities
    }

    fn random_index(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            rand::random::<usize>() % upper
        }
    }

    fn player_addr_by_entity(&self, entity_id: u64) -> Option<SocketAddr> {
        self.registry
            .players
            .values()
            .find(|player| player.entity_id as u64 == entity_id)
            .map(|player| player.addr)
    }

    fn teleport_player(&mut self, addr: SocketAddr, position: [f32; 3]) {
        if let Some(connection) = self.connections.get_mut(&addr) {
            let packets = connection.teleport_to(position);
            self.registry.update_position(
                &addr,
                position,
                connection.pitch,
                connection.yaw,
                connection.head_yaw,
            );
            for packet in packets {
                self.send_prepared(addr, packet);
            }
        }
    }

    fn set_player_gamemode(&mut self, addr: SocketAddr, mode: i32) {
        if let Some(connection) = self.connections.get_mut(&addr) {
            let packets = connection.apply_gamemode_packets(mode);
            if let Some(player) = self.registry.players.get_mut(&addr) {
                player.gamemode = mode;
            }
            for packet in packets {
                self.send_prepared(addr, packet);
            }
        }
    }

    fn player_position(&self, addr: SocketAddr) -> Option<[f32; 3]> {
        self.connections
            .get(&addr)
            .map(|connection| connection.position)
    }

    fn player_name(&self, addr: SocketAddr) -> Option<String> {
        self.connections
            .get(&addr)
            .and_then(|connection| connection.display_name.clone())
    }

    fn player_gamemode(&self, addr: SocketAddr) -> Option<i32> {
        self.connections
            .get(&addr)
            .map(|connection| connection.gamemode)
    }

    fn clear_inventory(&mut self, addr: SocketAddr) {
        if let Some(connection) = self.connections.get_mut(&addr) {
            connection.inventory.clear();
            for packet in connection.prepared_inventory_sync_packets() {
                self.raknet
                    .send_to_session(&addr, packet, Reliability::ReliableOrdered, true);
            }
        }
    }

    fn give_item(&mut self, addr: SocketAddr, item: ItemStack) -> Result<(), String> {
        if let Some(connection) = self.connections.get_mut(&addr) {
            connection
                .inventory
                .add_item(item)
                .ok_or_else(|| "Inventory is full.".to_string())?;
            for packet in connection.prepared_inventory_sync_packets() {
                self.raknet
                    .send_to_session(&addr, packet, Reliability::ReliableOrdered, true);
            }
            Ok(())
        } else {
            Err("Player is offline.".to_string())
        }
    }

    fn spawn_mob(&mut self, mob_name: &str, position: [f32; 3]) -> Result<u64, String> {
        let kind =
            MobKind::parse(mob_name).ok_or_else(|| format!("Unknown mob type: {mob_name}"))?;
        let entity = self.mob_entities.spawn(kind, position);
        self.broadcast_compressed(packet_id::ADD_ACTOR, &entity.add_actor_packet());
        Ok(entity.base.entity_runtime_id)
    }

    fn kill_player(&mut self, addr: SocketAddr) {
        if let Some(connection) = self.connections.get_mut(&addr) {
            connection.position = connection.spawn_position;
            let packets = connection.teleport_to(connection.spawn_position);
            for packet in packets {
                self.send_prepared(addr, packet);
            }
            self.send_message(addr, "You died!");
        }
    }

    fn remove_entity(&mut self, entity_id: u64) -> Result<(), String> {
        if let Some(entity) = self.item_entities.remove(entity_id) {
            let remove_packet = entity.remove_packet();
            self.broadcast_compressed(packet_id::REMOVE_ACTOR, &remove_packet);
            return Ok(());
        }
        if let Some(entity) = self.mob_entities.remove(entity_id) {
            let position = entity.base.position;
            let drops = entity.kind.default_loot();
            let remove_packet = entity.remove_packet();
            self.broadcast_compressed(packet_id::REMOVE_ACTOR, &remove_packet);
            for drop in drops {
                self.spawn_world_item_entity(PendingItemEntitySpawn::stationary(drop, position));
            }
            return Ok(());
        }
        Err("Entity could not be removed.".to_string())
    }

    fn set_time(&mut self, time: i32) {
        self.world_state.set_time(time);
        let packet = encode_set_time(time);
        self.broadcast_compressed(packet_id::SET_TIME, &packet);
    }

    fn current_time(&self) -> i32 {
        self.world_state.time
    }

    fn set_weather(&mut self, rain: bool, thunder: bool) {
        self.world_state.set_weather(rain, thunder);
    }

    fn add_player_xp(&mut self, addr: SocketAddr, amount: i32) -> Result<i32, String> {
        let Some(connection) = self.connections.get_mut(&addr) else {
            return Err("Player not connected".into());
        };
        let (level, _progress) = if amount >= 0 {
            crate::attribute::ExperienceManager::add_xp(&mut connection.attributes, amount)
        } else {
            crate::attribute::ExperienceManager::remove_xp(&mut connection.attributes, -amount)
        };
        Ok(level)
    }

    fn spawn_particle(&mut self, position: [f32; 3], particle_name: &str) {
        let bytes = crate::visuals::SpawnParticleEffect::at(position, particle_name);
        self.broadcast_compressed(packet_id::SPAWN_PARTICLE_EFFECT, &bytes);
    }

    fn boss_show(&mut self, title: &str, health_percent: f32) {
        // Server boss = id -1 (PMMP convention pour boss "fictif" non lié à entity).
        let bytes = crate::visuals::boss_show(-1, title, health_percent.clamp(0.0, 1.0), 5);
        self.broadcast_compressed(packet_id::BOSS_EVENT, &bytes);
    }
    fn boss_hide(&mut self) {
        let bytes = crate::visuals::boss_hide(-1);
        self.broadcast_compressed(packet_id::BOSS_EVENT, &bytes);
    }
    fn boss_set_title(&mut self, title: &str) {
        let bytes = crate::visuals::boss_update_title(-1, title);
        self.broadcast_compressed(packet_id::BOSS_EVENT, &bytes);
    }
    fn boss_set_health(&mut self, health_percent: f32) {
        let bytes = crate::visuals::boss_update_health(-1, health_percent.clamp(0.0, 1.0));
        self.broadcast_compressed(packet_id::BOSS_EVENT, &bytes);
    }
    fn scoreboard_set(&mut self, objective: &str, player: &str, score: i32) {
        // Stockage in-memory via le ScoreboardManager partagé.
        let mut mgr = self.server_state.scoreboards.lock().unwrap();
        let obj = mgr
            .objectives
            .entry(objective.to_string())
            .or_insert_with(|| crate::scoreboard::Objective::new(objective, objective));
        obj.set_score(player, score);
        // Sync vers les clients via SetScore + SetDisplayObjective : à faire
        // dans une phase ultérieure (PMMP NetworkSession::syncWorld).
    }

    fn apply_held_enchant(
        &mut self,
        addr: SocketAddr,
        enchant_id: u8,
        level: u8,
    ) -> Result<(), String> {
        let Some(connection) = self.connections.get_mut(&addr) else {
            return Err("Player not connected".into());
        };
        let slot = connection.inventory.held_slot as usize;
        let held = &mut connection.inventory.slots[slot];
        if held.item.is_air() {
            return Err("Held item is empty".into());
        }
        held.item.extra_data =
            crate::enchantments::build_extra_data_with_enchant(enchant_id, level);
        let cur = held.item.clone();
        connection.inventory_manager.set_slot(
            &mut connection.inventory,
            crate::inventory_manager::InvKey::Main,
            slot,
            cur,
        );
        let sync_pkts: Vec<Vec<u8>> = connection
            .tick_inventory_flush()
            .into_iter()
            .map(|p| connection.prepare_for_send(p))
            .collect();
        for packet in sync_pkts {
            self.send_prepared(addr, packet);
        }
        Ok(())
    }

    fn apply_player_effect(
        &mut self,
        addr: SocketAddr,
        effect_id: i32,
        duration_ticks: i32,
        amplifier: u8,
    ) -> Result<(), String> {
        let Some(connection) = self.connections.get_mut(&addr) else {
            return Err("Player not connected".into());
        };
        // Send Bedrock MobEffectPacket (event=ADD).
        let event_id = if duration_ticks <= 0 {
            mc_rs_proto::packets::world::MobEffect::EVENT_REMOVE
        } else {
            mc_rs_proto::packets::world::MobEffect::EVENT_ADD
        };
        let pkt = mc_rs_proto::packets::world::MobEffect {
            actor_runtime_id: connection.entity_runtime_id,
            event_id,
            effect_id,
            amplifier: amplifier as i32,
            particles: true,
            duration_ticks,
            tick: 0,
            ambient: false,
        };
        let bytes = connection
            .encode_compressed_packet(packet_id::MOB_EFFECT, &pkt.encode());
        let prepared = connection.prepare_for_send(bytes);
        self.send_prepared(addr, prepared);
        tracing::info!(
            "/effect applied: addr={addr} effect_id={effect_id} duration={duration_ticks} amplifier={amplifier}"
        );
        Ok(())
    }

    fn set_difficulty(&mut self, difficulty: i32) {
        self.server_state.persistent.difficulty = Some(difficulty);
        for connection in self.connections.values_mut() {
            connection.current_difficulty = difficulty;
        }
        let packet = SetDifficulty {
            difficulty: difficulty as u32,
        }
        .encode();
        self.broadcast_compressed(packet_id::SET_DIFFICULTY, &packet);
        let _ = self.server_state.save();
    }

    fn current_difficulty(&self) -> i32 {
        self.server_state.effective_difficulty(0)
    }

    fn set_default_gamemode(&mut self, gamemode: i32) {
        self.server_state.persistent.default_gamemode = Some(gamemode);
        for connection in self.connections.values_mut() {
            connection.world_gamemode = gamemode;
        }
        let _ = self.server_state.save();
    }

    fn current_default_gamemode(&self) -> i32 {
        self.server_state.effective_default_gamemode(0)
    }

    fn stop_server(&mut self) {
        *self.should_stop = true;
    }

    fn save_world(&mut self) -> Result<(), String> {
        self.chunk_cache
            .lock()
            .map_err(|_| "Chunk cache is poisoned.".to_string())?
            .save_dirty();
        let _ = self.server_state.save();
        // level.dat snapshot — sauve uniquement les champs accessibles ici.
        // Le path complet n'est pas dispo dans ExecutionContext, donc autosave
        // de level.dat se fait dans main.rs (qui a world_dir + config).
        Ok(())
    }

    fn set_auto_save(&mut self, enabled: bool) {
        self.server_state.auto_save_enabled = enabled;
    }

    fn auto_save_enabled(&self) -> bool {
        self.server_state.auto_save_enabled
    }

    fn kick(&mut self, addr: SocketAddr, reason: &str) {
        self.remove_player_from_world(addr, Some((DisconnectReason::Kicked, reason.to_string())));
        self.sync_available_commands_for_all();
    }

    fn transfer(&mut self, addr: SocketAddr, host: &str, port: u16) {
        let packet = Transfer {
            address: host.to_string(),
            port,
            reload_world: false,
        }
        .encode();
        self.send_compressed(addr, packet_id::TRANSFER, &packet);
        self.remove_player_from_world(addr, None);
        self.sync_available_commands_for_all();
    }

    fn set_player_spawn(&mut self, addr: SocketAddr, pos: [f32; 3]) -> Result<(), String> {
        if let Some(connection) = self.connections.get_mut(&addr) {
            connection.spawn_position = pos;
            let block_pos = [
                pos[0].floor() as i32,
                pos[1].floor() as i32,
                pos[2].floor() as i32,
            ];
            let packet = SetSpawnPosition {
                spawn_type: 0,
                position: block_pos,
                dimension: 0,
                spawn_position: block_pos,
            }
            .encode();
            self.send_compressed(addr, packet_id::SET_SPAWN_POSITION, &packet);
            Ok(())
        } else {
            Err("Player is offline.".to_string())
        }
    }

    fn player_spawn(&self, addr: SocketAddr) -> Option<[f32; 3]> {
        self.connections
            .get(&addr)
            .map(|connection| connection.spawn_position)
    }

    fn set_world_spawn(&mut self, pos: [f32; 3]) {
        self.server_state.persistent.world_spawn = Some(pos);
        let block_pos = [
            pos[0].floor() as i32,
            pos[1].floor() as i32,
            pos[2].floor() as i32,
        ];
        let packet = SetSpawnPosition {
            spawn_type: 1,
            position: block_pos,
            dimension: 0,
            spawn_position: block_pos,
        }
        .encode();
        self.broadcast_compressed(packet_id::SET_SPAWN_POSITION, &packet);
        let _ = self.server_state.save();
    }

    fn world_spawn(&self) -> [f32; 3] {
        self.server_state
            .persistent
            .world_spawn
            .unwrap_or([0.5, 64.0, 0.5])
    }

    fn op(&mut self, name: &str) {
        self.server_state.set_op(name, true);
        self.update_connection_permissions(name);
        let _ = self.server_state.save();
        self.sync_available_commands_for_all();
    }

    fn deop(&mut self, name: &str) {
        self.server_state.set_op(name, false);
        self.update_connection_permissions(name);
        let _ = self.server_state.save();
        self.sync_available_commands_for_all();
    }

    fn list_ops(&self) -> Vec<String> {
        self.server_state.persistent.ops.iter().cloned().collect()
    }

    fn set_whitelist_enabled(&mut self, enabled: bool) {
        self.server_state.persistent.whitelist_enabled = enabled;
        let _ = self.server_state.save();
    }

    fn whitelist_enabled(&self) -> bool {
        self.server_state.persistent.whitelist_enabled
    }

    fn whitelist_entries(&self) -> Vec<String> {
        self.server_state
            .persistent
            .whitelist
            .iter()
            .cloned()
            .collect()
    }

    fn whitelist_add(&mut self, name: &str) {
        self.server_state.set_whitelist_entry(name, true);
        let _ = self.server_state.save();
    }

    fn whitelist_remove(&mut self, name: &str) {
        self.server_state.set_whitelist_entry(name, false);
        let _ = self.server_state.save();
    }

    fn ban_name(&mut self, name: &str) {
        self.server_state.set_name_ban(name, true);
        if let Some(addr) = self.connections.iter().find_map(|(addr, connection)| {
            connection
                .display_name
                .as_deref()
                .is_some_and(|display_name| normalize_name(display_name) == normalize_name(name))
                .then_some(*addr)
        }) {
            self.kick(addr, "You have been banned from this server.");
        }
        let _ = self.server_state.save();
    }

    fn pardon_name(&mut self, name: &str) {
        self.server_state.set_name_ban(name, false);
        let _ = self.server_state.save();
    }

    fn banned_names(&self) -> Vec<String> {
        self.server_state
            .persistent
            .banned_names
            .iter()
            .cloned()
            .collect()
    }

    fn ban_ip(&mut self, ip: &str) {
        self.server_state.set_ip_ban(ip, true);
        let _ = self.server_state.save();
    }

    fn pardon_ip(&mut self, ip: &str) {
        self.server_state.set_ip_ban(ip, false);
        let _ = self.server_state.save();
    }

    fn banned_ips(&self) -> Vec<String> {
        self.server_state
            .persistent
            .banned_ips
            .iter()
            .cloned()
            .collect()
    }

    fn player_ip(&self, addr: SocketAddr) -> Option<String> {
        self.connections
            .contains_key(&addr)
            .then_some(addr.ip().to_string())
    }

    fn send_title(&mut self, addr: SocketAddr, action: TitlePacketAction) {
        let packet = match action {
            TitlePacketAction::Clear => SetTitle::simple(SetTitle::TYPE_CLEAR, ""),
            TitlePacketAction::Reset => SetTitle::simple(SetTitle::TYPE_RESET, ""),
            TitlePacketAction::Title(text) => SetTitle::simple(SetTitle::TYPE_TITLE, text),
            TitlePacketAction::Subtitle(text) => SetTitle::simple(SetTitle::TYPE_SUBTITLE, text),
            TitlePacketAction::Actionbar(text) => SetTitle::simple(SetTitle::TYPE_ACTIONBAR, text),
            TitlePacketAction::Times {
                fade_in,
                stay,
                fade_out,
            } => SetTitle::times(fade_in, stay, fade_out),
        }
        .encode();
        self.send_compressed(addr, packet_id::SET_TITLE, &packet);
    }

    fn sync_available_commands_for_all(&mut self) {
        let addrs = self
            .connections
            .iter()
            .filter_map(|(addr, connection)| connection.is_in_game().then_some(*addr))
            .collect::<Vec<_>>();
        for addr in addrs {
            self.sync_available_commands_for_addr(addr);
        }
    }

    fn server_motd(&self) -> &str {
        &self.server_state.server_motd
    }

    fn world_name(&self) -> &str {
        &self.server_state.world_name
    }

    fn world_seed(&self) -> u64 {
        self.server_state.world_seed
    }

    fn online_players(&self) -> usize {
        self.registry.count()
    }

    fn max_players(&self) -> u32 {
        self.server_state.max_players
    }

    fn execute_plugin_command(
        &mut self,
        plugin_name: &str,
        command_name: &str,
        invocation: &CommandInvocation,
    ) -> Result<(), CommandDispatchError> {
        let plugin_manager = Arc::clone(self.plugin_manager);
        let mut manager = plugin_manager.lock().map_err(|_| {
            CommandDispatchError::Message("Plugin manager lock is poisoned.".to_string())
        })?;
        manager
            .execute_command(plugin_name, command_name, invocation, self)
            .map_err(CommandDispatchError::Message)
    }

    fn plugin_names(&self) -> Vec<String> {
        self.plugin_manager
            .lock()
            .map(|manager| manager.plugin_names())
            .unwrap_or_default()
    }

    fn visible_command_names(&self) -> Vec<String> {
        let mut commands = self
            .command_system
            .map
            .definitions()
            .filter(|definition| {
                definition.permissions.is_empty()
                    || definition
                        .permissions
                        .iter()
                        .any(|permission| self.sender_has_permission(permission))
            })
            .map(|definition| definition.name.clone())
            .collect::<Vec<_>>();
        commands.sort();
        commands
    }
}

fn usage<T>(message: &str) -> Result<T, CommandDispatchError> {
    Err(CommandDispatchError::Usage(message.to_string()))
}

fn message<T>(message: impl Into<String>) -> Result<T, CommandDispatchError> {
    Err(CommandDispatchError::Message(message.into()))
}

fn parse_gamemode(token: &str) -> Option<i32> {
    match token.to_ascii_lowercase().as_str() {
        "0" | "survival" | "s" => Some(0),
        "1" | "creative" | "c" => Some(1),
        "2" | "adventure" | "a" => Some(2),
        "3" | "spectator" | "sp" => Some(3),
        _ => None,
    }
}

fn parse_difficulty(token: &str) -> Option<i32> {
    match token.to_ascii_lowercase().as_str() {
        "0" | "peaceful" | "p" => Some(0),
        "1" | "easy" | "e" => Some(1),
        "2" | "normal" | "n" => Some(2),
        "3" | "hard" | "h" => Some(3),
        _ => None,
    }
}

fn parse_time_value(token: &str) -> Option<i32> {
    match token.to_ascii_lowercase().as_str() {
        "day" | "sunrise" => Some(0),
        "noon" | "midday" => Some(6000),
        "sunset" | "dusk" => Some(12000),
        "night" | "midnight" => Some(18000),
        _ => token.parse::<i32>().ok(),
    }
}

fn parse_coord(token: &str, current: f32) -> Option<f32> {
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

fn resolve_player_targets(
    runtime: &mut dyn ServerCommandRuntime,
    token: Option<&str>,
    allow_multiple: bool,
) -> Result<Vec<SocketAddr>, CommandDispatchError> {
    let Some(token) = token else {
        return runtime.sender_addr().map(|addr| vec![addr]).ok_or_else(|| {
            CommandDispatchError::Message("This command requires an in-game sender.".to_string())
        });
    };

    let candidates = runtime.selector_entities();
    let random_index = runtime.random_index(candidates.len().max(1));
    let resolved = resolve_target_token_with_index(token, runtime, &candidates, random_index)
        .map_err(|error: SelectorError| CommandDispatchError::Message(error.to_string()))?;

    let players = resolved
        .into_iter()
        .filter_map(|entity| runtime.player_addr_by_entity(entity.id))
        .collect::<Vec<_>>();

    if players.is_empty() {
        return Err(CommandDispatchError::Message(
            "No player targets matched.".to_string(),
        ));
    }
    if !allow_multiple && players.len() != 1 {
        return Err(CommandDispatchError::Message(
            "This command requires exactly one player target.".to_string(),
        ));
    }
    Ok(players)
}

fn resolve_entity_targets(
    runtime: &mut dyn ServerCommandRuntime,
    token: &str,
) -> Result<Vec<(u64, Option<SocketAddr>)>, CommandDispatchError> {
    let candidates = runtime.selector_entities();
    let random_index = runtime.random_index(candidates.len().max(1));
    let resolved = resolve_target_token_with_index(token, runtime, &candidates, random_index)
        .map_err(|error: SelectorError| CommandDispatchError::Message(error.to_string()))?;

    Ok(resolved
        .into_iter()
        .map(|entity| {
            let player_addr = runtime.player_addr_by_entity(entity.id);
            (entity.id, player_addr)
        })
        .collect())
}

fn parse_item_stack(token: &str, count: u16) -> Result<ItemStack, CommandDispatchError> {
    if let Ok(item_id) = token.parse::<i32>() {
        let normalized_id = crate::item_registry::network_id_from_legacy(item_id)
            .or_else(|| crate::item_registry::is_known_network_id(item_id).then_some(item_id))
            .unwrap_or(item_id);
        let block_runtime_id = inventory::item_to_block(normalized_id)
            .map(|runtime_id| runtime_id as i32)
            .unwrap_or(0);
        return Ok(ItemStack::new(normalized_id, count, block_runtime_id));
    }

    let normalized = token.replace(' ', "_").to_ascii_lowercase();
    let item_name = if normalized.contains(':') {
        normalized
    } else {
        format!("minecraft:{normalized}")
    };
    let Some(item_id) = crate::item_registry::network_id(&item_name) else {
        return Err(CommandDispatchError::Message(format!(
            "Unknown item: {token}"
        )));
    };

    let runtime_id = BLOCKS.get(&item_name);
    let block_runtime_id = if runtime_id != BLOCKS.air {
        runtime_id as i32
    } else {
        0
    };

    Ok(ItemStack::new(item_id, count, block_runtime_id))
}

fn send_title_to_targets(
    runtime: &mut dyn ServerCommandRuntime,
    targets: &[SocketAddr],
    action: TitlePacketAction,
) {
    for target in targets {
        runtime.send_title(*target, action.clone());
    }
}

fn param(name: &str, param_type: ParamType, optional: bool) -> CommandParameter {
    CommandParameter {
        name: name.into(),
        param_type,
        optional,
    }
}

fn soft_player_param(name: &str, optional: bool) -> CommandParameter {
    param(
        name,
        ParamType::SoftEnum {
            name: "online_players".into(),
        },
        optional,
    )
}

fn hard_enum_param(
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

fn parse_position_triplet(
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

fn parse_position_triplet_for_source(
    runtime: &dyn ServerCommandRuntime,
    player_origin: Option<[f32; 3]>,
    x: &str,
    y: &str,
    z: &str,
) -> Result<[f32; 3], CommandDispatchError> {
    if runtime.sender_is_player() {
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

pub fn build_command_system() -> ServerCommandSystem {
    let mut permissions = PermissionRegistry::new();
    let mut map = ServerCommandMap::new();

    let mut help = CommandDefinition::new("help", "Show available commands");
    help.usage = "/help".into();
    help.permissions = vec!["server.command.help".into()];
    register_command(
        &mut permissions,
        &mut map,
        help,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            let commands = runtime.visible_command_names();
            runtime.send_feedback(&format!(
                "Available commands ({}): {}",
                commands.len(),
                commands.join(", ")
            ));
            Ok(())
        },
    );

    let mut version = CommandDefinition::new("version", "Show server version");
    version.usage = "/version".into();
    version.permissions = vec!["server.command.version".into()];
    register_command(
        &mut permissions,
        &mut map,
        version,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!(
                "{} | world={} | seed={}",
                SERVER_VERSION,
                runtime.world_name(),
                runtime.world_seed()
            ));
            Ok(())
        },
    );

    let mut plugins = CommandDefinition::new("plugins", "List loaded plugins");
    plugins.usage = "/plugins".into();
    plugins.permissions = vec!["server.command.plugins".into()];
    register_command(
        &mut permissions,
        &mut map,
        plugins,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            let plugin_names = runtime.plugin_names();
            if plugin_names.is_empty() {
                runtime.send_feedback("Plugins: none loaded");
            } else {
                runtime.send_feedback(&format!(
                    "Plugins ({}): {}",
                    plugin_names.len(),
                    plugin_names.join(", ")
                ));
            }
            Ok(())
        },
    );

    let mut status = CommandDefinition::new("status", "Show basic server status");
    status.usage = "/status".into();
    status.permissions = vec!["server.command.status".into()];
    register_command(
        &mut permissions,
        &mut map,
        status,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!(
                "{} | players={}/{} | difficulty={} | defaultGamemode={} | autoSave={}",
                runtime.server_motd(),
                runtime.online_players(),
                runtime.max_players(),
                runtime.current_difficulty(),
                runtime.current_default_gamemode(),
                runtime.auto_save_enabled()
            ));
            Ok(())
        },
    );

    let mut stop = CommandDefinition::new("stop", "Stop the server");
    stop.usage = "/stop".into();
    stop.permissions = vec!["server.command.stop".into()];
    register_command(
        &mut permissions,
        &mut map,
        stop,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback("Server shutting down...");
            runtime.stop_server();
            Ok(())
        },
    );

    let mut save = CommandDefinition::new("save", "Save the world immediately");
    save.usage = "/save".into();
    save.permissions = vec!["server.command.save".into()];
    register_command(
        &mut permissions,
        &mut map,
        save,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime
                .save_world()
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback("World and server state saved.");
            Ok(())
        },
    );

    let mut save_on = CommandDefinition::new("save-on", "Enable auto-save");
    save_on.usage = "/save-on".into();
    save_on.permissions = vec!["server.command.save".into()];
    register_command(
        &mut permissions,
        &mut map,
        save_on,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.set_auto_save(true);
            runtime.send_feedback("Auto-save enabled.");
            Ok(())
        },
    );

    let mut save_off = CommandDefinition::new("save-off", "Disable auto-save");
    save_off.usage = "/save-off".into();
    save_off.permissions = vec!["server.command.save".into()];
    register_command(
        &mut permissions,
        &mut map,
        save_off,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.set_auto_save(false);
            runtime.send_feedback("Auto-save disabled.");
            Ok(())
        },
    );

    let mut gc = CommandDefinition::new("gc", "Explain Rust memory management");
    gc.usage = "/gc".into();
    gc.permissions = vec!["server.command.gc".into()];
    register_command(
        &mut permissions,
        &mut map,
        gc,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback("Rust has no manual GC cycle to trigger. Memory is reclaimed automatically when values are dropped.");
            Ok(())
        },
    );

    let mut dumpmemory = CommandDefinition::new("dumpmemory", "Show lightweight memory/debug info");
    dumpmemory.usage = "/dumpmemory".into();
    dumpmemory.permissions = vec!["server.command.dumpmemory".into()];
    register_command(
        &mut permissions,
        &mut map,
        dumpmemory,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!(
                "Debug snapshot: players={} itemEntities={} world={}",
                runtime.online_players(),
                runtime
                    .selector_entities()
                    .into_iter()
                    .filter(|entity| entity.entity_type == "item")
                    .count(),
                runtime.world_name()
            ));
            Ok(())
        },
    );

    let mut timings = CommandDefinition::new("timings", "Show lightweight timings status");
    timings.usage = "/timings".into();
    timings.permissions = vec!["server.command.timings".into()];
    register_command(
        &mut permissions,
        &mut map,
        timings,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback("Timings/profiling is not wired yet. Use tracing logs for now.");
            Ok(())
        },
    );

    let mut list = CommandDefinition::new("list", "List online players");
    list.usage = "/list".into();
    list.permissions = vec!["server.command.list".into()];
    register_command(
        &mut permissions,
        &mut map,
        list,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            let players = runtime.soft_enum_values("online_players");
            runtime.send_feedback(&format!(
                "Online players ({}): {}",
                players.len(),
                players.join(", ")
            ));
            Ok(())
        },
    );

    let mut say = CommandDefinition::new("say", "Broadcast a server message");
    say.usage = "/say <message>".into();
    say.permissions = vec!["server.command.say".into()];
    say.overloads.push(CommandOverload {
        parameters: vec![CommandParameter {
            name: "message".into(),
            param_type: ParamType::Message,
            optional: false,
        }],
    });
    register_command(
        &mut permissions,
        &mut map,
        say,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() {
                return usage("Usage: /say <message>");
            }
            runtime.broadcast_chat("Server", &invocation.raw_args);
            Ok(())
        },
    );

    let mut me = CommandDefinition::new("me", "Broadcast an emote");
    me.usage = "/me <action>".into();
    me.permissions = vec!["server.command.me".into()];
    me.overloads.push(CommandOverload {
        parameters: vec![CommandParameter {
            name: "action".into(),
            param_type: ParamType::Message,
            optional: false,
        }],
    });
    register_command(
        &mut permissions,
        &mut map,
        me,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() {
                return usage("Usage: /me <action>");
            }
            let sender_name = runtime.sender_name().to_string();
            runtime.broadcast_action(&sender_name, &invocation.raw_args);
            Ok(())
        },
    );

    let mut tell = CommandDefinition::new("tell", "Send a private message");
    tell.aliases = vec!["msg".into(), "w".into()];
    tell.usage = "/tell <target> <message>".into();
    tell.permissions = vec!["server.command.tell".into()];
    tell.overloads.push(CommandOverload {
        parameters: vec![
            CommandParameter {
                name: "target".into(),
                param_type: ParamType::SoftEnum {
                    name: "online_players".into(),
                },
                optional: false,
            },
            CommandParameter {
                name: "message".into(),
                param_type: ParamType::Message,
                optional: false,
            },
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        tell,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /tell <target> <message>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let message_text = invocation.tail(1);
            for target in &targets {
                runtime.send_message(
                    *target,
                    &format!("[{} -> you] {}", runtime.sender_name(), message_text),
                );
            }
            runtime.send_feedback(&format!(
                "[you -> {}] {}",
                invocation.arg(0).unwrap_or("?"),
                message_text
            ));
            Ok(())
        },
    );

    let mut kick = CommandDefinition::new("kick", "Kick one or more players");
    kick.usage = "/kick <target> [reason]".into();
    kick.permissions = vec!["server.command.kick".into()];
    kick.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("reason", ParamType::Message, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        kick,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() {
                return usage("Usage: /kick <target> [reason]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let reason = if invocation.args.len() > 1 {
                invocation.tail(1)
            } else {
                "Kicked from the server.".to_string()
            };
            let count = targets.len();
            for target in targets {
                runtime.kick(target, &reason);
            }
            runtime.send_feedback(&format!("Kicked {count} player(s)."));
            Ok(())
        },
    );

    let mut op = CommandDefinition::new("op", "Grant operator status");
    op.usage = "/op <player>".into();
    op.permissions = vec!["server.command.op".into()];
    op.overloads.push(CommandOverload {
        parameters: vec![param("player", ParamType::String, false)],
    });
    register_command(
        &mut permissions,
        &mut map,
        op,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /op <player>");
            };
            runtime.op(name);
            runtime.send_feedback(&format!("{name} is now an operator."));
            Ok(())
        },
    );

    let mut deop = CommandDefinition::new("deop", "Revoke operator status");
    deop.usage = "/deop <player>".into();
    deop.permissions = vec!["server.command.deop".into()];
    deop.overloads.push(CommandOverload {
        parameters: vec![param("player", ParamType::String, false)],
    });
    register_command(
        &mut permissions,
        &mut map,
        deop,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /deop <player>");
            };
            runtime.deop(name);
            runtime.send_feedback(&format!("{name} is no longer an operator."));
            Ok(())
        },
    );

    let mut whitelist = CommandDefinition::new("whitelist", "Manage the server whitelist");
    whitelist.usage = "/whitelist <on|off|list|add|remove> [player]".into();
    whitelist.permissions = vec!["server.command.whitelist".into()];
    whitelist.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "action",
            "whitelist_action",
            &["on", "off", "list", "add", "remove"],
            false,
        )],
    });
    whitelist.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "whitelist_mutation", &["add", "remove"], false),
            param("player", ParamType::String, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        whitelist,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /whitelist <on|off|list|add|remove> [player]");
            };
            match action.to_ascii_lowercase().as_str() {
                "on" => {
                    runtime.set_whitelist_enabled(true);
                    runtime.send_feedback("Whitelist enabled.");
                }
                "off" => {
                    runtime.set_whitelist_enabled(false);
                    runtime.send_feedback("Whitelist disabled.");
                }
                "list" => {
                    let entries = runtime.whitelist_entries();
                    runtime.send_feedback(&format!(
                        "Whitelist {} ({}): {}",
                        if runtime.whitelist_enabled() {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        entries.len(),
                        if entries.is_empty() {
                            "empty".to_string()
                        } else {
                            entries.join(", ")
                        }
                    ));
                }
                "add" => {
                    let Some(name) = invocation.arg(1) else {
                        return usage("Usage: /whitelist add <player>");
                    };
                    runtime.whitelist_add(name);
                    runtime.send_feedback(&format!("Added {name} to the whitelist."));
                }
                "remove" => {
                    let Some(name) = invocation.arg(1) else {
                        return usage("Usage: /whitelist remove <player>");
                    };
                    runtime.whitelist_remove(name);
                    runtime.send_feedback(&format!("Removed {name} from the whitelist."));
                }
                _ => return usage("Usage: /whitelist <on|off|list|add|remove> [player]"),
            }
            Ok(())
        },
    );

    let mut ban = CommandDefinition::new("ban", "Ban a player name");
    ban.usage = "/ban <player> [reason]".into();
    ban.permissions = vec!["server.command.ban".into()];
    ban.overloads.push(CommandOverload {
        parameters: vec![
            param("player", ParamType::Target, false),
            param("reason", ParamType::Message, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        ban,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_token) = invocation.arg(0) else {
                return usage("Usage: /ban <player> [reason]");
            };
            let mut banned_names = Vec::new();
            if let Ok(targets) = resolve_player_targets(runtime, Some(target_token), true) {
                for target in targets {
                    if let Some(name) = runtime.player_name(target) {
                        runtime.ban_name(&name);
                        banned_names.push(name);
                    }
                }
            } else {
                runtime.ban_name(target_token);
                banned_names.push(target_token.to_string());
            }
            runtime.send_feedback(&format!("Banned: {}", banned_names.join(", ")));
            Ok(())
        },
    );

    let mut ban_ip = CommandDefinition::new("ban-ip", "Ban an IP address or online player IP");
    ban_ip.usage = "/ban-ip <ip|player>".into();
    ban_ip.permissions = vec!["server.command.banip".into()];
    ban_ip.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::String, false)],
    });
    register_command(
        &mut permissions,
        &mut map,
        ban_ip,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_token) = invocation.arg(0) else {
                return usage("Usage: /ban-ip <ip|player>");
            };
            if let Ok(targets) = resolve_player_targets(runtime, Some(target_token), false) {
                let target = targets[0];
                let ip = runtime.player_ip(target).ok_or_else(|| {
                    CommandDispatchError::Message("Player IP is unavailable.".to_string())
                })?;
                runtime.ban_ip(&ip);
                runtime.kick(target, "Your IP has been banned from this server.");
                runtime.send_feedback(&format!("Banned IP {ip}."));
            } else {
                runtime.ban_ip(target_token);
                runtime.send_feedback(&format!("Banned IP {target_token}."));
            }
            Ok(())
        },
    );

    let mut banlist = CommandDefinition::new("banlist", "Show current bans");
    banlist.usage = "/banlist [players|ips|all]".into();
    banlist.permissions = vec!["server.command.banlist".into()];
    banlist.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "type",
            "banlist_type",
            &["players", "ips", "all"],
            true,
        )],
    });
    register_command(
        &mut permissions,
        &mut map,
        banlist,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let selection = invocation.arg(0).unwrap_or("all").to_ascii_lowercase();
            let names = runtime.banned_names();
            let ips = runtime.banned_ips();
            match selection.as_str() {
                "players" => runtime.send_feedback(&format!(
                    "Banned players ({}): {}",
                    names.len(),
                    if names.is_empty() {
                        "none".to_string()
                    } else {
                        names.join(", ")
                    }
                )),
                "ips" => runtime.send_feedback(&format!(
                    "Banned IPs ({}): {}",
                    ips.len(),
                    if ips.is_empty() {
                        "none".to_string()
                    } else {
                        ips.join(", ")
                    }
                )),
                "all" => runtime.send_feedback(&format!(
                    "Banned players ({}): {} | Banned IPs ({}): {}",
                    names.len(),
                    if names.is_empty() {
                        "none".to_string()
                    } else {
                        names.join(", ")
                    },
                    ips.len(),
                    if ips.is_empty() {
                        "none".to_string()
                    } else {
                        ips.join(", ")
                    }
                )),
                _ => return usage("Usage: /banlist [players|ips|all]"),
            }
            Ok(())
        },
    );

    let mut pardon = CommandDefinition::new("pardon", "Remove a player ban");
    pardon.usage = "/pardon <player>".into();
    pardon.permissions = vec!["server.command.pardon".into()];
    pardon.overloads.push(CommandOverload {
        parameters: vec![param("player", ParamType::String, false)],
    });
    register_command(
        &mut permissions,
        &mut map,
        pardon,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /pardon <player>");
            };
            runtime.pardon_name(name);
            runtime.send_feedback(&format!("Removed ban for {name}."));
            Ok(())
        },
    );

    let mut pardon_ip = CommandDefinition::new("pardon-ip", "Remove an IP ban");
    pardon_ip.usage = "/pardon-ip <ip>".into();
    pardon_ip.permissions = vec!["server.command.pardonip".into()];
    pardon_ip.overloads.push(CommandOverload {
        parameters: vec![param("ip", ParamType::String, false)],
    });
    register_command(
        &mut permissions,
        &mut map,
        pardon_ip,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(ip) = invocation.arg(0) else {
                return usage("Usage: /pardon-ip <ip>");
            };
            runtime.pardon_ip(ip);
            runtime.send_feedback(&format!("Removed IP ban for {ip}."));
            Ok(())
        },
    );

    let mut gamemode = CommandDefinition::new("gamemode", "Change a player's gamemode");
    gamemode.usage = "/gamemode <survival|creative|adventure|spectator> [player]".into();
    gamemode.permissions = vec!["server.command.gamemode".into()];
    gamemode.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param(
                "gamemode",
                "gamemode_values",
                &[
                    "survival",
                    "creative",
                    "adventure",
                    "spectator",
                    "s",
                    "c",
                    "a",
                    "sp",
                    "0",
                    "1",
                    "2",
                    "3",
                ],
                false,
            ),
            param("player", ParamType::Target, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        gamemode,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(mode_token) = invocation.arg(0) else {
                return usage("Usage: /gamemode <survival|creative|adventure|spectator> [player]");
            };
            let mode = parse_gamemode(mode_token).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown gamemode: {mode_token}"))
            })?;
            if invocation.arg(1).is_none() && !runtime.sender_is_player() {
                return message(
                    "Console must specify a player target. Usage: /gamemode <mode> <player>",
                );
            }
            let targets = resolve_player_targets(runtime, invocation.arg(1), true)?;
            let count = targets.len();
            for target in targets {
                runtime.set_player_gamemode(target, mode);
            }
            runtime.send_feedback(&format!("Updated gamemode for {count} player(s)."));
            Ok(())
        },
    );

    let mut teleport = CommandDefinition::new("tp", "Teleport players");
    teleport.aliases = vec!["teleport".into()];
    teleport.usage = "/tp [target] <destination|x y z>".into();
    teleport.permissions = vec!["server.command.tp".into()];
    teleport.overloads.push(CommandOverload {
        parameters: vec![param("destination", ParamType::Target, false)],
    });
    teleport.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("destination", ParamType::Target, false),
        ],
    });
    teleport.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    teleport.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        teleport,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            match invocation.args.len() {
                1 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify a player target. Usage: /tp <target> <destination>",
                        );
                    }
                    let sender = runtime.sender_addr().ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?;
                    let destination = resolve_player_targets(runtime, invocation.arg(0), false)?[0];
                    let position = runtime.player_position(destination).ok_or_else(|| {
                        CommandDispatchError::Message(
                            "Destination player is unavailable.".to_string(),
                        )
                    })?;
                    runtime.teleport_player(sender, position);
                    runtime.send_feedback("Teleported.");
                }
                2 => {
                    let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
                    let destination = resolve_player_targets(runtime, invocation.arg(1), false)?[0];
                    let position = runtime.player_position(destination).ok_or_else(|| {
                        CommandDispatchError::Message(
                            "Destination player is unavailable.".to_string(),
                        )
                    })?;
                    let count = targets.len();
                    for target in targets {
                        runtime.teleport_player(target, position);
                    }
                    runtime.send_feedback(&format!("Teleported {count} player(s)."));
                }
                3 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify a player target. Usage: /tp <target> <x> <y> <z>",
                        );
                    }
                    let sender = runtime.sender_addr().ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?;
                    let origin = runtime.player_position(sender).ok_or_else(|| {
                        CommandDispatchError::Message("Sender position is unavailable.".to_string())
                    })?;
                    let position = parse_position_triplet_for_source(
                        runtime,
                        Some(origin),
                        invocation.arg(0).unwrap_or(""),
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                    )?;
                    runtime.teleport_player(sender, position);
                    runtime.send_feedback("Teleported.");
                }
                4 => {
                    let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
                    let origin = runtime
                        .sender_addr()
                        .and_then(|addr| runtime.player_position(addr));
                    let position = parse_position_triplet_for_source(
                        runtime,
                        origin,
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                        invocation.arg(3).unwrap_or(""),
                    )?;
                    let count = targets.len();
                    for target in targets {
                        runtime.teleport_player(target, position);
                    }
                    runtime.send_feedback(&format!("Teleported {count} player(s)."));
                }
                _ => return usage("Usage: /tp [target] <destination|x y z>"),
            }
            Ok(())
        },
    );

    let mut kill = CommandDefinition::new("kill", "Kill players or remove entities");
    kill.usage = "/kill [target]".into();
    kill.permissions = vec!["server.command.kill".into()];
    kill.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::Target, true)],
    });
    register_command(
        &mut permissions,
        &mut map,
        kill,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                if !runtime.sender_is_player() {
                    return message("Console must specify a target. Usage: /kill <target>");
                }
                let sender = runtime.sender_addr().ok_or_else(|| {
                    CommandDispatchError::Message(
                        "This command requires an in-game sender.".to_string(),
                    )
                })?;
                runtime.kill_player(sender);
                runtime.send_feedback("You died.");
                return Ok(());
            };
            let targets = resolve_entity_targets(runtime, token)?;
            let count = targets.len();
            for (entity_id, player_addr) in targets {
                if let Some(addr) = player_addr {
                    runtime.kill_player(addr);
                } else {
                    runtime
                        .remove_entity(entity_id)
                        .map_err(CommandDispatchError::Message)?;
                }
            }
            runtime.send_feedback(&format!("Killed or removed {count} target(s)."));
            Ok(())
        },
    );

    let mut clear = CommandDefinition::new("clear", "Clear player inventories");
    clear.usage = "/clear [target]".into();
    clear.permissions = vec!["server.command.clear".into()];
    clear.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::Target, true)],
    });
    register_command(
        &mut permissions,
        &mut map,
        clear,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.arg(0).is_none() && !runtime.sender_is_player() {
                return message("Console must specify a player target. Usage: /clear <target>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let count = targets.len();
            for target in targets {
                runtime.clear_inventory(target);
            }
            runtime.send_feedback(&format!("Cleared inventory for {count} player(s)."));
            Ok(())
        },
    );

    let mut give = CommandDefinition::new("give", "Give items to players");
    give.usage = "/give <target> <item> [count]".into();
    give.permissions = vec!["server.command.give".into()];
    give.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("item", ParamType::String, false),
            param("count", ParamType::Int, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        give,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /give <target> <item> [count]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            let count = if let Some(count_token) = invocation.arg(2) {
                count_token.parse::<u16>().map_err(|_| {
                    CommandDispatchError::Message(format!("Invalid count: {count_token}"))
                })?
            } else {
                1
            };
            let item = parse_item_stack(invocation.arg(1).unwrap_or(""), count)?;
            let count_targets = targets.len();
            for target in targets {
                runtime
                    .give_item(target, item.clone())
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!("Gave item to {count_targets} player(s)."));
            Ok(())
        },
    );

    let mut summon = CommandDefinition::new("summon", "Summon a basic mob entity");
    summon.usage = "/summon <entity> [x y z]".into();
    summon.permissions = vec!["server.command.summon".into()];
    summon.overloads.push(CommandOverload {
        parameters: vec![param("entity", ParamType::String, false)],
    });
    summon.overloads.push(CommandOverload {
        parameters: vec![
            param("entity", ParamType::String, false),
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        summon,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(entity_name) = invocation.arg(0) else {
                return usage("Usage: /summon <entity> [x y z]");
            };

            let sender = runtime.sender_addr();
            let sender_pos = sender.and_then(|addr| runtime.player_position(addr));
            let position = match invocation.args.len() {
                1 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify absolute coordinates. Usage: /summon <entity> <x> <y> <z>",
                        );
                    }
                    let mut pos = sender_pos.ok_or_else(|| {
                        CommandDispatchError::Message("Sender position is unavailable.".to_string())
                    })?;
                    pos[1] += 1.0;
                    pos
                }
                4 => parse_position_triplet_for_source(
                    runtime,
                    sender_pos,
                    invocation.arg(1).unwrap_or(""),
                    invocation.arg(2).unwrap_or(""),
                    invocation.arg(3).unwrap_or(""),
                )?,
                _ => return usage("Usage: /summon <entity> [x y z]"),
            };

            let entity_id = runtime
                .spawn_mob(entity_name, position)
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback(&format!(
                "Summoned {entity_name} at {:.1} {:.1} {:.1} (entity_id={entity_id}).",
                position[0], position[1], position[2]
            ));
            Ok(())
        },
    );

    let mut spawnpoint = CommandDefinition::new("spawnpoint", "Set a player's respawn point");
    spawnpoint.usage = "/spawnpoint [player] [x y z]".into();
    spawnpoint.permissions = vec!["server.command.spawnpoint".into()];
    spawnpoint.overloads.push(CommandOverload::default());
    spawnpoint.overloads.push(CommandOverload {
        parameters: vec![param("player", ParamType::Target, false)],
    });
    spawnpoint.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    spawnpoint.overloads.push(CommandOverload {
        parameters: vec![
            param("player", ParamType::Target, false),
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        spawnpoint,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if !runtime.sender_is_player() && matches!(invocation.args.len(), 0 | 1 | 3) {
                return message(
                    "Console must specify a player target and absolute coordinates when needed. Usage: /spawnpoint <player> [x y z]",
                );
            }
            let sender = runtime.sender_addr();
            let sender_pos = sender.and_then(|addr| runtime.player_position(addr));

            let (targets, position) = match invocation.args.len() {
                0 => (
                    vec![sender.ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?],
                    sender_pos.ok_or_else(|| {
                        CommandDispatchError::Message("Sender position is unavailable.".to_string())
                    })?,
                ),
                1 => {
                    if invocation.arg(0).unwrap_or("").starts_with('~')
                        || invocation.arg(0).unwrap_or("").parse::<f32>().is_ok()
                    {
                        return usage("Usage: /spawnpoint [player] [x y z]");
                    }
                    (
                        resolve_player_targets(runtime, invocation.arg(0), true)?,
                        sender_pos.ok_or_else(|| {
                            CommandDispatchError::Message(
                                "Sender position is unavailable.".to_string(),
                            )
                        })?,
                    )
                }
                3 => (
                    vec![sender.ok_or_else(|| {
                        CommandDispatchError::Message(
                            "This command requires an in-game sender.".to_string(),
                        )
                    })?],
                    parse_position_triplet_for_source(
                        runtime,
                        sender_pos,
                        invocation.arg(0).unwrap_or(""),
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                    )?,
                ),
                4 => (
                    resolve_player_targets(runtime, invocation.arg(0), true)?,
                    parse_position_triplet_for_source(
                        runtime,
                        sender_pos,
                        invocation.arg(1).unwrap_or(""),
                        invocation.arg(2).unwrap_or(""),
                        invocation.arg(3).unwrap_or(""),
                    )?,
                ),
                _ => return usage("Usage: /spawnpoint [player] [x y z]"),
            };

            let count = targets.len();
            for target in targets {
                runtime
                    .set_player_spawn(target, position)
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!("Updated spawnpoint for {count} player(s)."));
            Ok(())
        },
    );

    let mut setworldspawn = CommandDefinition::new("setworldspawn", "Set the world spawn");
    setworldspawn.usage = "/setworldspawn [x y z]".into();
    setworldspawn.permissions = vec!["server.command.setworldspawn".into()];
    setworldspawn.overloads.push(CommandOverload::default());
    setworldspawn.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        setworldspawn,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.is_empty() && !runtime.sender_is_player() {
                return message(
                    "Console must specify coordinates. Usage: /setworldspawn <x> <y> <z>",
                );
            }
            let sender = runtime.sender_addr();
            let sender_pos = sender.and_then(|addr| runtime.player_position(addr));
            let position = match invocation.args.len() {
                0 => sender_pos.ok_or_else(|| {
                    CommandDispatchError::Message(
                        "This command requires an in-game sender.".to_string(),
                    )
                })?,
                3 => parse_position_triplet_for_source(
                    runtime,
                    sender_pos,
                    invocation.arg(0).unwrap_or(""),
                    invocation.arg(1).unwrap_or(""),
                    invocation.arg(2).unwrap_or(""),
                )?,
                _ => return usage("Usage: /setworldspawn [x y z]"),
            };
            runtime.set_world_spawn(position);
            runtime.send_feedback("World spawn updated.");
            Ok(())
        },
    );

    let mut time = CommandDefinition::new("time", "Control world time");
    time.usage = "/time <set|add|query> [value]".into();
    time.permissions = vec!["server.command.time".into()];
    time.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "time_action", &["set", "add", "query"], false),
            param("value", ParamType::String, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        time,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /time <set|add|query> [value]");
            };
            match action.to_ascii_lowercase().as_str() {
                "set" => {
                    let Some(value_token) = invocation.arg(1) else {
                        return usage("Usage: /time set <value>");
                    };
                    let value = parse_time_value(value_token).ok_or_else(|| {
                        CommandDispatchError::Message(format!("Invalid time value: {value_token}"))
                    })?;
                    runtime.set_time(value);
                    runtime.send_feedback(&format!("Set time to {value}."));
                }
                "add" => {
                    let Some(value_token) = invocation.arg(1) else {
                        return usage("Usage: /time add <ticks>");
                    };
                    let delta = value_token.parse::<i32>().map_err(|_| {
                        CommandDispatchError::Message(format!("Invalid tick amount: {value_token}"))
                    })?;
                    let new_time = runtime.current_time().saturating_add(delta);
                    runtime.set_time(new_time);
                    runtime.send_feedback(&format!("Advanced time to {new_time}."));
                }
                "query" => {
                    runtime.send_feedback(&format!("Current time: {}", runtime.current_time()));
                }
                _ => return usage("Usage: /time <set|add|query> [value]"),
            }
            Ok(())
        },
    );

    let mut difficulty = CommandDefinition::new("difficulty", "Show or change difficulty");
    difficulty.usage = "/difficulty [peaceful|easy|normal|hard]".into();
    difficulty.permissions = vec!["server.command.difficulty".into()];
    difficulty.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "difficulty",
            "difficulty_values",
            &[
                "peaceful", "easy", "normal", "hard", "p", "e", "n", "h", "0", "1", "2", "3",
            ],
            true,
        )],
    });
    register_command(
        &mut permissions,
        &mut map,
        difficulty,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                runtime.send_feedback(&format!(
                    "Current difficulty: {}",
                    runtime.current_difficulty()
                ));
                return Ok(());
            };
            let difficulty = parse_difficulty(token).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown difficulty: {token}"))
            })?;
            runtime.set_difficulty(difficulty);
            runtime.send_feedback(&format!("Difficulty set to {difficulty}."));
            Ok(())
        },
    );

    let mut default_gamemode = CommandDefinition::new(
        "defaultgamemode",
        "Show or change the default world gamemode",
    );
    default_gamemode.usage = "/defaultgamemode [mode]".into();
    default_gamemode.permissions = vec!["server.command.defaultgamemode".into()];
    default_gamemode.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "gamemode",
            "default_gamemode_values",
            &[
                "survival",
                "creative",
                "adventure",
                "spectator",
                "s",
                "c",
                "a",
                "sp",
                "0",
                "1",
                "2",
                "3",
            ],
            true,
        )],
    });
    register_command(
        &mut permissions,
        &mut map,
        default_gamemode,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                runtime.send_feedback(&format!(
                    "Current default gamemode: {}",
                    runtime.current_default_gamemode()
                ));
                return Ok(());
            };
            let gamemode = parse_gamemode(token).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown gamemode: {token}"))
            })?;
            runtime.set_default_gamemode(gamemode);
            runtime.send_feedback(&format!("Default gamemode set to {gamemode}."));
            Ok(())
        },
    );

    let mut seed = CommandDefinition::new("seed", "Show the world seed");
    seed.usage = "/seed".into();
    seed.permissions = vec!["server.command.seed".into()];
    register_command(
        &mut permissions,
        &mut map,
        seed,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.send_feedback(&format!("World seed: {}", runtime.world_seed()));
            Ok(())
        },
    );

    // ── /weather <clear|rain|thunder> ──
    let mut weather = CommandDefinition::new("weather", "Control world weather");
    weather.usage = "/weather <clear|rain|thunder>".into();
    weather.permissions = vec!["server.command.weather".into()];
    weather.overloads.push(CommandOverload {
        parameters: vec![hard_enum_param(
            "state",
            "weather_state",
            &["clear", "rain", "thunder"],
            false,
        )],
    });
    register_command(
        &mut permissions,
        &mut map,
        weather,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(state) = invocation.arg(0) else {
                return usage("Usage: /weather <clear|rain|thunder>");
            };
            match state.to_ascii_lowercase().as_str() {
                "clear" => {
                    runtime.set_weather(false, false);
                    runtime.send_feedback("Weather set to clear.");
                }
                "rain" => {
                    runtime.set_weather(true, false);
                    runtime.send_feedback("Weather set to rain.");
                }
                "thunder" => {
                    runtime.set_weather(true, true);
                    runtime.send_feedback("Weather set to thunder.");
                }
                _ => return usage("Usage: /weather <clear|rain|thunder>"),
            }
            Ok(())
        },
    );

    // ── /xp <add|set|query> [amount] [target] ──
    let mut xp = CommandDefinition::new("xp", "Manage player experience");
    xp.usage = "/xp <add|set|query> [amount] [target]".into();
    xp.permissions = vec!["server.command.xp".into()];
    xp.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "xp_action", &["add", "set", "query"], false),
            param("amount", ParamType::Int, true),
            param("target", ParamType::Target, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        xp,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /xp <add|set|query> [amount] [target]");
            };

            // Cible par défaut = self si possible.
            let addr = match invocation.arg(2) {
                Some(target_name) => {
                    let entity_id = runtime
                        .selector_entities()
                        .into_iter()
                        .find(|e| {
                            e.name
                                .as_deref()
                                .map(|n| n.eq_ignore_ascii_case(target_name))
                                .unwrap_or(false)
                        })
                        .map(|e| e.id);
                    entity_id.and_then(|id| runtime.player_addr_by_entity(id))
                }
                None => runtime.sender_addr(),
            }
            .ok_or_else(|| {
                CommandDispatchError::Message(
                    "No target player (specify a name or run as a player)".into(),
                )
            })?;

            match action.to_ascii_lowercase().as_str() {
                "add" => {
                    let Some(amount_tok) = invocation.arg(1) else {
                        return usage("Usage: /xp add <amount> [target]");
                    };
                    let amount: i32 = amount_tok.parse().map_err(|_| {
                        CommandDispatchError::Message(format!(
                            "Invalid amount: {amount_tok}"
                        ))
                    })?;
                    let new_level = runtime
                        .add_player_xp(addr, amount)
                        .map_err(CommandDispatchError::Message)?;
                    runtime.send_feedback(&format!(
                        "Added {amount} XP (level now {new_level})"
                    ));
                }
                "set" => {
                    // set = query current then add diff (pour récupérer diff
                    // il faudrait query_xp ; on simplifie en retirant tout puis
                    // réajoutant).
                    let Some(amount_tok) = invocation.arg(1) else {
                        return usage("Usage: /xp set <amount> [target]");
                    };
                    let amount: i32 = amount_tok.parse().map_err(|_| {
                        CommandDispatchError::Message(format!(
                            "Invalid amount: {amount_tok}"
                        ))
                    })?;
                    // Clear total puis ajouter — simple et correct pour une
                    // 1ère version.
                    let _ = runtime.add_player_xp(addr, i32::MIN / 2);
                    let level = runtime
                        .add_player_xp(addr, amount)
                        .map_err(CommandDispatchError::Message)?;
                    runtime.send_feedback(&format!("Set XP to {amount} (level {level})"));
                }
                "query" => {
                    let level = runtime
                        .add_player_xp(addr, 0)
                        .map_err(CommandDispatchError::Message)?;
                    runtime.send_feedback(&format!("Level: {level}"));
                }
                _ => return usage("Usage: /xp <add|set|query> [amount] [target]"),
            }
            Ok(())
        },
    );

    // ── /effect <target> <effect_id> [duration] [amplifier] ──
    let mut effect = CommandDefinition::new("effect", "Apply a potion effect");
    effect.usage = "/effect <target> <effect_id> [duration] [amplifier]".into();
    effect.permissions = vec!["server.command.effect".into()];
    effect.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("effect_id", ParamType::Int, false),
            param("duration", ParamType::Int, true),
            param("amplifier", ParamType::Int, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        effect,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_name) = invocation.arg(0) else {
                return usage("Usage: /effect <target> <effect_name|id> [duration] [amplifier]");
            };
            let Some(effect_tok) = invocation.arg(1) else {
                return usage("Usage: /effect <target> <effect_name|id> [duration] [amplifier]");
            };
            // Accepte "minecraft:speed", "speed" ou un id numérique.
            let kind = crate::effects::EffectKind::from_name_or_id(effect_tok).ok_or_else(|| {
                CommandDispatchError::Message(format!("Unknown effect: {effect_tok}"))
            })?;
            let effect_id: i32 = kind.id() as i32;
            let duration: i32 = invocation.arg(2).and_then(|s| s.parse().ok()).unwrap_or(600);
            let amplifier: u8 = invocation.arg(3).and_then(|s| s.parse().ok()).unwrap_or(0);

            let entity_id = runtime
                .selector_entities()
                .into_iter()
                .find(|e| {
                    e.name
                        .as_deref()
                        .map(|n| n.eq_ignore_ascii_case(target_name))
                        .unwrap_or(false)
                })
                .map(|e| e.id);
            let addr = entity_id
                .and_then(|id| runtime.player_addr_by_entity(id))
                .ok_or_else(|| {
                    CommandDispatchError::Message(format!("Player not found: {target_name}"))
                })?;

            runtime
                .apply_player_effect(addr, effect_id, duration, amplifier)
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback(&format!(
                "Applied effect {effect_id} (duration={duration}, amplifier={amplifier})"
            ));
            Ok(())
        },
    );

    // ── /enchant <target> <enchant_name|id> [level] ──
    let mut enchant = CommandDefinition::new("enchant", "Add enchantment to held item");
    enchant.usage = "/enchant <target> <enchant_name|id> [level]".into();
    enchant.permissions = vec!["server.command.enchant".into()];
    enchant.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("enchant", ParamType::String, false),
            param("level", ParamType::Int, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        enchant,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_name) = invocation.arg(0) else {
                return usage("Usage: /enchant <target> <enchant_name|id> [level]");
            };
            let Some(ench_tok) = invocation.arg(1) else {
                return usage("Usage: /enchant <target> <enchant_name|id> [level]");
            };
            let kind =
                crate::enchantments::EnchantmentKind::from_name_or_id(ench_tok).ok_or_else(|| {
                    CommandDispatchError::Message(format!("Unknown enchantment: {ench_tok}"))
                })?;
            let level: u8 = invocation.arg(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let max = kind.max_level();
            let level = level.min(max).max(1);

            let entity_id = runtime
                .selector_entities()
                .into_iter()
                .find(|e| {
                    e.name
                        .as_deref()
                        .map(|n| n.eq_ignore_ascii_case(target_name))
                        .unwrap_or(false)
                })
                .map(|e| e.id);
            let addr = entity_id
                .and_then(|id| runtime.player_addr_by_entity(id))
                .ok_or_else(|| {
                    CommandDispatchError::Message(format!("Player not found: {target_name}"))
                })?;

            runtime
                .apply_held_enchant(addr, kind.id(), level)
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback(&format!(
                "Enchanted held item with {ench_tok} {level} (max {max})"
            ));
            Ok(())
        },
    );

    // ── /boss <show|hide|title|health> [args] ──
    let mut boss = CommandDefinition::new("boss", "Manage a boss bar (server-wide)");
    boss.usage = "/boss <show|hide|title|health> [args]".into();
    boss.permissions = vec!["server.command.boss".into()];
    boss.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param(
                "action",
                "boss_action",
                &["show", "hide", "title", "health"],
                false,
            ),
            param("value", ParamType::Message, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        boss,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /boss <show|hide|title|health> [args]");
            };
            match action {
                "show" => {
                    let title = invocation.arg(1).unwrap_or("Boss");
                    runtime.boss_show(title, 1.0);
                    runtime.send_feedback(&format!("Boss bar shown: {title}"));
                }
                "hide" => {
                    runtime.boss_hide();
                    runtime.send_feedback("Boss bar hidden");
                }
                "title" => {
                    let t = invocation.arg(1).unwrap_or("");
                    runtime.boss_set_title(t);
                    runtime.send_feedback(&format!("Boss title: {t}"));
                }
                "health" => {
                    let p: f32 = invocation
                        .arg(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                    runtime.boss_set_health(p.clamp(0.0, 1.0));
                    runtime.send_feedback(&format!("Boss health: {:.0}%", p * 100.0));
                }
                _ => return usage("Usage: /boss <show|hide|title|health> [args]"),
            }
            Ok(())
        },
    );

    // ── /scoreboard <objective> <player> <score> ──
    let mut sb = CommandDefinition::new(
        "scoreboard",
        "Set a player score on a sidebar objective",
    );
    sb.usage = "/scoreboard <objective> <player> <score>".into();
    sb.permissions = vec!["server.command.scoreboard".into()];
    sb.overloads.push(CommandOverload {
        parameters: vec![
            param("objective", ParamType::String, false),
            param("player", ParamType::String, false),
            param("score", ParamType::Int, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        sb,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(obj) = invocation.arg(0) else {
                return usage("Usage: /scoreboard <objective> <player> <score>");
            };
            let Some(player) = invocation.arg(1) else {
                return usage("Usage: /scoreboard <objective> <player> <score>");
            };
            let score: i32 = invocation.arg(2).and_then(|s| s.parse().ok()).ok_or_else(
                || CommandDispatchError::Message("score must be an integer".into()),
            )?;
            runtime.scoreboard_set(obj, player, score);
            runtime.send_feedback(&format!("Scoreboard {obj}: {player} = {score}"));
            Ok(())
        },
    );

    // ── /particle <name> [x] [y] [z] ──
    let mut particle = CommandDefinition::new("particle", "Spawn a particle effect");
    particle.usage = "/particle <name> [x] [y] [z]".into();
    particle.permissions = vec!["server.command.particle".into()];
    particle.overloads.push(CommandOverload {
        parameters: vec![
            param("name", ParamType::String, false),
            param("x", ParamType::Float, true),
            param("y", ParamType::Float, true),
            param("z", ParamType::Float, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        particle,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name_tok) = invocation.arg(0) else {
                return usage("Usage: /particle <name> [x] [y] [z]");
            };
            // Si pas de coords explicites, prend la position du sender.
            let sender_pos = runtime.sender_position();
            let x: f32 = invocation
                .arg(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(sender_pos[0]);
            let y: f32 = invocation
                .arg(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(sender_pos[1]);
            let z: f32 = invocation
                .arg(3)
                .and_then(|s| s.parse().ok())
                .unwrap_or(sender_pos[2]);
            let pname = if name_tok.contains(':') {
                name_tok.to_string()
            } else {
                format!("minecraft:{name_tok}")
            };
            runtime.spawn_particle([x, y, z], &pname);
            runtime.send_feedback(&format!("Spawned particle {pname} at ({x:.1},{y:.1},{z:.1})"));
            Ok(())
        },
    );

    let mut title = CommandDefinition::new("title", "Send Bedrock title packets");
    title.usage = "/title <target> <clear|reset|title|subtitle|actionbar|times> [...]".into();
    title.permissions = vec!["server.command.title".into()];
    title.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            hard_enum_param(
                "action",
                "title_action",
                &["clear", "reset", "title", "subtitle", "actionbar", "times"],
                false,
            ),
            param("value", ParamType::Message, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        title,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage(
                    "Usage: /title <target> <clear|reset|title|subtitle|actionbar|times> [...]",
                );
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            match invocation
                .arg(1)
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "clear" => {
                    send_title_to_targets(runtime, &targets, TitlePacketAction::Clear);
                    runtime.send_feedback("Cleared titles.");
                }
                "reset" => {
                    send_title_to_targets(runtime, &targets, TitlePacketAction::Reset);
                    runtime.send_feedback("Reset titles.");
                }
                "title" => {
                    if invocation.args.len() < 3 {
                        return usage("Usage: /title <target> title <text>");
                    }
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Title(invocation.tail(2)),
                    );
                    runtime.send_feedback("Sent title.");
                }
                "subtitle" => {
                    if invocation.args.len() < 3 {
                        return usage("Usage: /title <target> subtitle <text>");
                    }
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Subtitle(invocation.tail(2)),
                    );
                    runtime.send_feedback("Sent subtitle.");
                }
                "actionbar" => {
                    if invocation.args.len() < 3 {
                        return usage("Usage: /title <target> actionbar <text>");
                    }
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Actionbar(invocation.tail(2)),
                    );
                    runtime.send_feedback("Sent actionbar.");
                }
                "times" => {
                    if invocation.args.len() != 5 {
                        return usage("Usage: /title <target> times <fadeIn> <stay> <fadeOut>");
                    }
                    let fade_in = invocation
                        .arg(2)
                        .unwrap_or("")
                        .parse::<i32>()
                        .map_err(|_| {
                            CommandDispatchError::Message("Invalid fadeIn value.".to_string())
                        })?;
                    let stay = invocation
                        .arg(3)
                        .unwrap_or("")
                        .parse::<i32>()
                        .map_err(|_| {
                            CommandDispatchError::Message("Invalid stay value.".to_string())
                        })?;
                    let fade_out =
                        invocation
                            .arg(4)
                            .unwrap_or("")
                            .parse::<i32>()
                            .map_err(|_| {
                                CommandDispatchError::Message("Invalid fadeOut value.".to_string())
                            })?;
                    send_title_to_targets(
                        runtime,
                        &targets,
                        TitlePacketAction::Times {
                            fade_in,
                            stay,
                            fade_out,
                        },
                    );
                    runtime.send_feedback("Updated title timings.");
                }
                _ => {
                    return usage(
                        "Usage: /title <target> <clear|reset|title|subtitle|actionbar|times> [...]",
                    )
                }
            }
            Ok(())
        },
    );

    let mut transfer =
        CommandDefinition::new("transferserver", "Transfer players to another server");
    transfer.usage = "/transferserver [target] <host> <port>".into();
    transfer.permissions = vec!["server.command.transferserver".into()];
    transfer.overloads.push(CommandOverload {
        parameters: vec![
            param("host", ParamType::String, false),
            param("port", ParamType::Int, false),
        ],
    });
    transfer.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("host", ParamType::String, false),
            param("port", ParamType::Int, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        transfer,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let (targets, host_token, port_token) = match invocation.args.len() {
                2 => {
                    if !runtime.sender_is_player() {
                        return message(
                            "Console must specify a player target. Usage: /transferserver <target> <host> <port>",
                        );
                    }
                    (
                        resolve_player_targets(runtime, None, true)?,
                        invocation.arg(0).unwrap_or(""),
                        invocation.arg(1).unwrap_or(""),
                    )
                }
                3 => (
                    resolve_player_targets(runtime, invocation.arg(0), true)?,
                    invocation.arg(1).unwrap_or(""),
                    invocation.arg(2).unwrap_or(""),
                ),
                _ => return usage("Usage: /transferserver [target] <host> <port>"),
            };
            let port = port_token.parse::<u16>().map_err(|_| {
                CommandDispatchError::Message(format!("Invalid port: {port_token}"))
            })?;
            let count = targets.len();
            for target in targets {
                runtime.transfer(target, host_token, port);
            }
            runtime.send_feedback(&format!(
                "Transferred {count} player(s) to {host_token}:{port}."
            ));
            Ok(())
        },
    );

    ServerCommandSystem { permissions, map }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    #[derive(Clone)]
    struct TestPlayer {
        addr: SocketAddr,
        name: String,
        entity_id: u64,
        position: [f32; 3],
        gamemode: i32,
        spawn_position: [f32; 3],
        inventory: Vec<ItemStack>,
        messages: Vec<String>,
        kicked_reason: Option<String>,
        transfer_target: Option<(String, u16)>,
        titles: Vec<TitlePacketAction>,
    }

    #[derive(Clone)]
    struct TestMob {
        entity_id: u64,
        name: String,
        entity_type: String,
        position: [f32; 3],
    }

    struct TestRuntime {
        visible_commands: Vec<String>,
        feedback: Vec<String>,
        broadcasts: Vec<String>,
        action_broadcasts: Vec<String>,
        players: HashMap<SocketAddr, TestPlayer>,
        mobs: HashMap<u64, TestMob>,
        next_entity_id: u64,
        removed_entities: Vec<u64>,
        should_stop: bool,
        time: i32,
        difficulty: i32,
        default_gamemode: i32,
        auto_save_enabled: bool,
        ops: BTreeSet<String>,
        whitelist_enabled: bool,
        whitelist: BTreeSet<String>,
        banned_names: BTreeSet<String>,
        banned_ips: BTreeSet<String>,
        world_spawn: [f32; 3],
    }

    impl TestRuntime {
        fn new(system: &ServerCommandSystem) -> Self {
            let steve = TestPlayer {
                addr: addr(19132),
                name: "Steve".to_string(),
                entity_id: 1,
                position: [0.0, 64.0, 0.0],
                gamemode: 0,
                spawn_position: [0.0, 64.0, 0.0],
                inventory: Vec::new(),
                messages: Vec::new(),
                kicked_reason: None,
                transfer_target: None,
                titles: Vec::new(),
            };
            let alex = TestPlayer {
                addr: addr(19133),
                name: "Alex".to_string(),
                entity_id: 2,
                position: [10.0, 70.0, 10.0],
                gamemode: 0,
                spawn_position: [10.0, 70.0, 10.0],
                inventory: Vec::new(),
                messages: Vec::new(),
                kicked_reason: None,
                transfer_target: None,
                titles: Vec::new(),
            };
            let players = [steve, alex]
                .into_iter()
                .map(|player| (player.addr, player))
                .collect::<HashMap<_, _>>();

            Self {
                visible_commands: system
                    .map
                    .definitions()
                    .map(|definition| definition.name.clone())
                    .collect(),
                feedback: Vec::new(),
                broadcasts: Vec::new(),
                action_broadcasts: Vec::new(),
                players,
                mobs: HashMap::new(),
                next_entity_id: 100,
                removed_entities: Vec::new(),
                should_stop: false,
                time: 0,
                difficulty: 2,
                default_gamemode: 0,
                auto_save_enabled: true,
                ops: BTreeSet::new(),
                whitelist_enabled: false,
                whitelist: BTreeSet::new(),
                banned_names: BTreeSet::new(),
                banned_ips: BTreeSet::new(),
                world_spawn: [0.5, 64.0, 0.5],
            }
        }

        fn player(&self, name: &str) -> &TestPlayer {
            self.players
                .values()
                .find(|player| player.name.eq_ignore_ascii_case(name))
                .expect("player exists")
        }

        fn player_mut(&mut self, addr: SocketAddr) -> &mut TestPlayer {
            self.players.get_mut(&addr).expect("player exists")
        }
    }

    impl CommandSender for TestRuntime {
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

    impl SoftEnumSource for TestRuntime {
        fn soft_enum_values(&self, name: &str) -> Vec<String> {
            if name.eq_ignore_ascii_case("online_players") {
                let mut values = self
                    .players
                    .values()
                    .map(|player| player.name.clone())
                    .collect::<Vec<_>>();
                values.sort();
                values
            } else {
                Vec::new()
            }
        }
    }

    impl ServerCommandRuntime for TestRuntime {
        fn sender_addr(&self) -> Option<SocketAddr> {
            None
        }

        fn send_feedback(&mut self, message: &str) {
            self.feedback.push(message.to_string());
        }

        fn send_message(&mut self, addr: SocketAddr, message: &str) {
            self.player_mut(addr).messages.push(message.to_string());
        }

        fn broadcast_chat(&mut self, source: &str, message: &str) {
            self.broadcasts.push(format!("{source}:{message}"));
        }

        fn broadcast_action(&mut self, source: &str, message: &str) {
            self.action_broadcasts.push(format!("{source}:{message}"));
        }

        fn open_sender_menu(&mut self) {
            self.feedback
                .push("Console cannot open the in-game menu.".to_string());
        }

        fn show_sender_biome(&mut self) {
            self.feedback
                .push("Console must target a player to inspect a biome.".to_string());
        }

        fn selector_entities(&self) -> Vec<SelectorEntity> {
            let mut entities = self
                .players
                .values()
                .map(|player| SelectorEntity {
                    id: player.entity_id,
                    name: Some(player.name.clone()),
                    entity_type: "player".to_string(),
                    position: player.position,
                    gamemode: Some(player.gamemode),
                })
                .collect::<Vec<_>>();
            entities.extend(self.mobs.values().map(|mob| SelectorEntity {
                id: mob.entity_id,
                name: Some(mob.name.clone()),
                entity_type: mob.entity_type.clone(),
                position: mob.position,
                gamemode: None,
            }));
            entities
        }

        fn random_index(&mut self, _upper: usize) -> usize {
            0
        }

        fn player_addr_by_entity(&self, entity_id: u64) -> Option<SocketAddr> {
            self.players
                .values()
                .find(|player| player.entity_id == entity_id)
                .map(|player| player.addr)
        }

        fn teleport_player(&mut self, addr: SocketAddr, position: [f32; 3]) {
            self.player_mut(addr).position = position;
        }

        fn set_player_gamemode(&mut self, addr: SocketAddr, mode: i32) {
            self.player_mut(addr).gamemode = mode;
        }

        fn player_position(&self, addr: SocketAddr) -> Option<[f32; 3]> {
            self.players.get(&addr).map(|player| player.position)
        }

        fn player_name(&self, addr: SocketAddr) -> Option<String> {
            self.players.get(&addr).map(|player| player.name.clone())
        }

        fn player_gamemode(&self, addr: SocketAddr) -> Option<i32> {
            self.players.get(&addr).map(|player| player.gamemode)
        }

        fn clear_inventory(&mut self, addr: SocketAddr) {
            self.player_mut(addr).inventory.clear();
        }

        fn give_item(&mut self, addr: SocketAddr, item: ItemStack) -> Result<(), String> {
            self.player_mut(addr).inventory.push(item);
            Ok(())
        }

        fn spawn_mob(&mut self, mob_name: &str, position: [f32; 3]) -> Result<u64, String> {
            let kind =
                MobKind::parse(mob_name).ok_or_else(|| format!("Unknown mob type: {mob_name}"))?;
            let entity_id = self.next_entity_id;
            self.next_entity_id += 1;
            self.mobs.insert(
                entity_id,
                TestMob {
                    entity_id,
                    name: kind.display_name().to_string(),
                    entity_type: kind.actor_type().to_string(),
                    position,
                },
            );
            Ok(entity_id)
        }

        fn kill_player(&mut self, addr: SocketAddr) {
            let player = self.player_mut(addr);
            player.position = player.spawn_position;
            player.messages.push("You died!".to_string());
        }

        fn remove_entity(&mut self, entity_id: u64) -> Result<(), String> {
            if self.mobs.remove(&entity_id).is_some() {
                self.removed_entities.push(entity_id);
                Ok(())
            } else {
                Err("Entity could not be removed.".to_string())
            }
        }

        fn set_time(&mut self, time: i32) {
            self.time = time;
        }

        fn current_time(&self) -> i32 {
            self.time
        }

        fn set_weather(&mut self, _rain: bool, _thunder: bool) {}
        fn add_player_xp(
            &mut self,
            _addr: SocketAddr,
            _amount: i32,
        ) -> Result<i32, String> {
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

        fn set_difficulty(&mut self, difficulty: i32) {
            self.difficulty = difficulty;
        }

        fn current_difficulty(&self) -> i32 {
            self.difficulty
        }

        fn set_default_gamemode(&mut self, gamemode: i32) {
            self.default_gamemode = gamemode;
        }

        fn current_default_gamemode(&self) -> i32 {
            self.default_gamemode
        }

        fn stop_server(&mut self) {
            self.should_stop = true;
        }

        fn save_world(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn set_auto_save(&mut self, enabled: bool) {
            self.auto_save_enabled = enabled;
        }

        fn auto_save_enabled(&self) -> bool {
            self.auto_save_enabled
        }

        fn kick(&mut self, addr: SocketAddr, reason: &str) {
            self.player_mut(addr).kicked_reason = Some(reason.to_string());
        }

        fn transfer(&mut self, addr: SocketAddr, host: &str, port: u16) {
            self.player_mut(addr).transfer_target = Some((host.to_string(), port));
        }

        fn set_player_spawn(&mut self, addr: SocketAddr, pos: [f32; 3]) -> Result<(), String> {
            self.player_mut(addr).spawn_position = pos;
            Ok(())
        }

        fn player_spawn(&self, addr: SocketAddr) -> Option<[f32; 3]> {
            self.players.get(&addr).map(|player| player.spawn_position)
        }

        fn set_world_spawn(&mut self, pos: [f32; 3]) {
            self.world_spawn = pos;
        }

        fn world_spawn(&self) -> [f32; 3] {
            self.world_spawn
        }

        fn op(&mut self, name: &str) {
            self.ops.insert(normalize_name(name));
        }

        fn deop(&mut self, name: &str) {
            self.ops.remove(&normalize_name(name));
        }

        fn list_ops(&self) -> Vec<String> {
            self.ops.iter().cloned().collect()
        }

        fn set_whitelist_enabled(&mut self, enabled: bool) {
            self.whitelist_enabled = enabled;
        }

        fn whitelist_enabled(&self) -> bool {
            self.whitelist_enabled
        }

        fn whitelist_entries(&self) -> Vec<String> {
            self.whitelist.iter().cloned().collect()
        }

        fn whitelist_add(&mut self, name: &str) {
            self.whitelist.insert(normalize_name(name));
        }

        fn whitelist_remove(&mut self, name: &str) {
            self.whitelist.remove(&normalize_name(name));
        }

        fn ban_name(&mut self, name: &str) {
            self.banned_names.insert(normalize_name(name));
        }

        fn pardon_name(&mut self, name: &str) {
            self.banned_names.remove(&normalize_name(name));
        }

        fn banned_names(&self) -> Vec<String> {
            self.banned_names.iter().cloned().collect()
        }

        fn ban_ip(&mut self, ip: &str) {
            self.banned_ips.insert(ip.to_string());
        }

        fn pardon_ip(&mut self, ip: &str) {
            self.banned_ips.remove(ip);
        }

        fn banned_ips(&self) -> Vec<String> {
            self.banned_ips.iter().cloned().collect()
        }

        fn player_ip(&self, addr: SocketAddr) -> Option<String> {
            self.players
                .contains_key(&addr)
                .then_some(addr.ip().to_string())
        }

        fn send_title(&mut self, addr: SocketAddr, action: TitlePacketAction) {
            self.player_mut(addr).titles.push(action);
        }

        fn sync_available_commands_for_all(&mut self) {}

        fn server_motd(&self) -> &str {
            "Test Server"
        }

        fn world_name(&self) -> &str {
            "test-world"
        }

        fn world_seed(&self) -> u64 {
            42
        }

        fn online_players(&self) -> usize {
            self.players.len()
        }

        fn max_players(&self) -> u32 {
            20
        }

        fn execute_plugin_command(
            &mut self,
            _plugin_name: &str,
            _command_name: &str,
            _invocation: &CommandInvocation,
        ) -> Result<(), CommandDispatchError> {
            Err(CommandDispatchError::Message(
                "Plugin commands are not wired in this test runtime.".to_string(),
            ))
        }

        fn plugin_names(&self) -> Vec<String> {
            Vec::new()
        }

        fn visible_command_names(&self) -> Vec<String> {
            let mut commands = self.visible_commands.clone();
            commands.sort();
            commands
        }
    }

    fn addr(port: u16) -> SocketAddr {
        format!("127.0.0.1:{port}").parse().unwrap()
    }

    fn dispatch_ok(system: &ServerCommandSystem, runtime: &mut TestRuntime, command: &str) {
        system.map.dispatch(runtime, command).unwrap();
    }

    fn dispatch_err(
        system: &ServerCommandSystem,
        runtime: &mut TestRuntime,
        command: &str,
    ) -> String {
        system
            .map
            .dispatch(runtime, command)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn console_help_lists_all_visible_commands() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "help");

        let feedback = runtime.feedback.last().cloned().unwrap_or_default();
        assert!(feedback.contains("gamemode"));
        assert!(feedback.contains("transferserver"));
    }

    #[test]
    fn console_admin_commands_work() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "status");
        dispatch_ok(&system, &mut runtime, "say Server maintenance soon");
        dispatch_ok(&system, &mut runtime, "time set day");
        dispatch_ok(&system, &mut runtime, "stop");

        assert!(runtime
            .feedback
            .iter()
            .any(|line| line.contains("players=")));
        assert_eq!(runtime.broadcasts, vec!["Server:Server maintenance soon"]);
        assert_eq!(runtime.time, 0);
        assert!(runtime.should_stop);
    }

    #[test]
    fn console_targeted_player_commands_work() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "gamemode creative Steve");
        dispatch_ok(&system, &mut runtime, "tp Steve Alex");
        dispatch_ok(&system, &mut runtime, "give Steve stone 64");
        dispatch_ok(&system, &mut runtime, "kick Steve");

        let steve = runtime.player("Steve");
        let alex = runtime.player("Alex");
        assert_eq!(steve.gamemode, 1);
        assert_eq!(steve.position, alex.position);
        assert_eq!(steve.inventory.len(), 1);
        assert_eq!(steve.inventory[0].count, 64);
        assert_eq!(
            steve.kicked_reason.as_deref(),
            Some("Kicked from the server.")
        );
    }

    #[test]
    fn console_rejects_sender_dependent_forms_cleanly() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        let cases = [
            ("gamemode creative", "Console must specify a player target"),
            ("tp 0 64 0", "Console must specify a player target"),
            ("kill", "Console must specify a target"),
            ("clear", "Console must specify a player target"),
            ("spawnpoint", "Console must specify a player target"),
            ("setworldspawn", "Console must specify coordinates"),
            (
                "transferserver example.com 19132",
                "Console must specify a player target",
            ),
        ];

        for (command, expected) in cases {
            let error = dispatch_err(&system, &mut runtime, command);
            assert!(
                error.contains(expected),
                "expected `{expected}` in `{error}` for command `{command}`"
            );
        }
    }

    #[test]
    fn kill_removes_summoned_mob_with_short_type_selector() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "summon zombie 0 64 0");
        assert_eq!(runtime.mobs.len(), 1);

        dispatch_ok(&system, &mut runtime, "kill @e[type=zombie]");
        assert!(
            runtime.mobs.is_empty(),
            "expected summoned mob to be removed"
        );
        assert_eq!(runtime.removed_entities.len(), 1);
    }

    #[test]
    fn kill_removes_summoned_mob_with_namespaced_type_selector() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "summon zombie 0 64 0");
        assert_eq!(runtime.mobs.len(), 1);

        dispatch_ok(&system, &mut runtime, "kill @e[type=minecraft:zombie]");
        assert!(
            runtime.mobs.is_empty(),
            "expected summoned mob to be removed"
        );
        assert_eq!(runtime.removed_entities.len(), 1);
    }
}
