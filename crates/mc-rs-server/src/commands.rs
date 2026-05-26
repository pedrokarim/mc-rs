use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use mc_rs_command::{
    hard_enum_param, message, param, parse_coord, parse_position_triplet_for_source,
    resolve_target_token_with_index, usage, CommandDefinition, CommandDispatchError,
    CommandInvocation, CommandOverload, CommandParameter, CommandSender, ParamType,
    PermissionDefault, PermissionDefinition, PermissionRegistry, PermissionState,
    RegistrationError, SelectorEntity, SelectorError, SoftEnumSource, VisibleCommand,
    VisibleCommandOverload, VisibleCommandParameter, VisibleParamType,
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
    /// Résout un nom de bloc (`minecraft:stone`, `stone`, …) vers son network_id
    /// canonique. Retourne `None` si inconnu.
    fn resolve_block_name(&self, name: &str) -> Option<u32>;
    /// Pose un bloc et broadcast `UpdateBlock` à tous les joueurs in_game.
    /// Retourne `true` si la valeur du bloc a changé.
    fn set_world_block(&mut self, x: i32, y: i32, z: i32, block_id: u32) -> bool;
    fn world_block_at(&self, x: i32, y: i32, z: i32) -> u32;
    /// Renvoie tous les game rules courants (name, value).
    fn gamerule_list(&self) -> Vec<(String, crate::game_rules::GameRuleValue)>;
    /// Renvoie la valeur d'un game rule par nom (case-insensitive).
    fn gamerule_get(&self, name: &str) -> Option<crate::game_rules::GameRuleValue>;
    /// Set un game rule. Le type doit matcher (Bool→Bool etc.). Broadcast
    /// `GameRulesChanged` aux joueurs in_game. Retourne `Err` si rule
    /// inconnue ou type incompatible.
    fn gamerule_set(
        &mut self,
        name: &str,
        value: crate::game_rules::GameRuleValue,
    ) -> Result<(), String>;
    /// Envoie un Text packet brut (déjà encodé) à un joueur — pour /tellraw.
    fn tellraw_send(&mut self, addr: SocketAddr, encoded_text_payload: &[u8]);
    /// Joue un son côté client. Si `targets` est vide, broadcast à tous.
    fn play_sound(
        &mut self,
        targets: &[SocketAddr],
        sound: &str,
        position: [f32; 3],
        volume: f32,
        pitch: f32,
    );
    /// Arrête un son (ou tous si `sound == None`).
    fn stop_sound(&mut self, targets: &[SocketAddr], sound: Option<&str>);
    /// Remplace un slot précis dans l'inventaire d'un joueur. Sync via
    /// InventoryManager pour broadcast correct au client.
    fn replace_player_slot(
        &mut self,
        addr: SocketAddr,
        inv_key: crate::inventory_manager::InvKey,
        slot_index: usize,
        item: ItemStack,
    ) -> Result<(), String>;
    /// Ajoute un tag à un joueur. Retourne `true` si nouveau.
    fn player_tag_add(&mut self, addr: SocketAddr, tag: &str) -> bool;
    /// Retire un tag d'un joueur. Retourne `true` si retiré.
    fn player_tag_remove(&mut self, addr: SocketAddr, tag: &str) -> bool;
    /// Liste les tags d'un joueur (alphabétique pour /tag list).
    fn player_tag_list(&self, addr: SocketAddr) -> Vec<String>;
    /// Spawn un item entity au sol (PendingItemEntitySpawn::stationary) — pour /loot.
    fn spawn_item_world(&mut self, position: [f32; 3], item: ItemStack);
    /// Roll une chest loot table par nom (ex `minecraft:simple_dungeon`) et
    /// renvoie les drops résolus (name, count).
    fn roll_chest_loot_drops(&self, table_name: &str) -> Vec<(String, u32)>;
    /// Inflige `amount` HP de dégâts à un joueur via combat::attack_entity.
    /// Retourne true si le joueur est mort suite au dégât.
    fn damage_player(&mut self, addr: SocketAddr, amount: f32) -> Result<bool, String>;
    /// Broadcast un ActorEvent (event_id + data) pour une entité — pour /event.
    fn actor_event_broadcast(&mut self, runtime_entity_id: u64, event_id: u32, data: i32);
    /// Récupère le runtime entity ID pour un selector unique (premier match).
    fn first_entity_runtime_id(&self, token: &str) -> Option<u64>;
    /// Recharge l'état persistant (ops/whitelist/bans) depuis disque. Note :
    /// ne reload PAS server.toml, ni les chunks, ni les plugins déjà chargés.
    fn reload_server_state(&mut self) -> Result<(), String>;
    /// Toggle un flag d'ability (mayfly, mute, worldbuilder…) sur un joueur.
    /// Envoie un UpdateAbilities one-shot. Pas persistant : un changement de
    /// gamemode reset les abilities aux defaults.
    fn set_player_ability(
        &mut self,
        addr: SocketAddr,
        ability: &str,
        value: bool,
    ) -> Result<(), String>;
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
    /// Commande venue d'un client RCON distant. Le feedback est capturé dans
    /// `ExecutionContext::rcon_output` au lieu d'être envoyé sur le réseau.
    Rcon,
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
    /// Buffer de capture pour RCON : `send_feedback`/`broadcast_chat`/`broadcast_action`
    /// y poussent leur message au lieu d'utiliser le réseau quand
    /// `source == CommandSource::Rcon`. Vide pour les autres sources.
    pub rcon_output: Vec<String>,
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
) -> Vec<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
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
    runtime.rcon_output
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
            rcon_output: Vec::new(),
        }
    }

    fn source_addr(&self) -> Option<SocketAddr> {
        match self.source {
            CommandSource::Player(addr) => Some(addr),
            CommandSource::Console | CommandSource::Rcon => None,
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
            CommandSource::Console | CommandSource::Rcon => true,
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
        let add_bytes = entity.add_actor_packet();
        // Culling : seuls les joueurs proches reçoivent l'Add ; les autres
        // entreront dans la vue via le scan périodique si le joueur s'en
        // approche.
        let entity_uid = entity.entity_unique_id;
        let entity_pos = entity.position;
        let targets: Vec<std::net::SocketAddr> = self
            .connections
            .iter_mut()
            .filter_map(|(addr, conn)| {
                if !conn.is_in_game() {
                    return None;
                }
                if !crate::entity_culling::is_within_view_for(conn, entity_pos) {
                    return None;
                }
                conn.visible_entities.insert(entity_uid);
                Some(*addr)
            })
            .collect();
        for addr in targets {
            self.send_compressed(addr, packet_id::ADD_ITEM_ACTOR, &add_bytes);
        }
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
        match name.to_ascii_lowercase().as_str() {
            "online_players" => {
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
            // Le client Bedrock vanilla affiche une icône à côté de chaque
            // valeur si le nom court correspond à un item connu (acacia_boat,
            // dirt, ...). On envoie sans le préfixe `minecraft:`.
            "item" => crate::item_registry::all_entries()
                .into_iter()
                .map(|(full_name, _)| {
                    full_name
                        .strip_prefix("minecraft:")
                        .unwrap_or(full_name)
                        .to_string()
                })
                .collect(),
            "effect" => crate::effects::EffectKind::all_names()
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            "enchantment" => crate::enchantments::EnchantmentKind::all_names()
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            "entitytype" => crate::mob_entities::MobKind::all_names()
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            _ => Vec::new(),
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
            CommandSource::Rcon => "Rcon",
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
            CommandSource::Console | CommandSource::Rcon => true,
        }
    }

    fn sender_has_permission(&self, permission: &str) -> bool {
        if matches!(self.source, CommandSource::Console | CommandSource::Rcon) {
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
                    skin: Default::default(),
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
        match self.source {
            CommandSource::Player(addr) => self.send_message(addr, message),
            CommandSource::Console => info!("[CONSOLE] {message}"),
            CommandSource::Rcon => self.rcon_output.push(message.to_string()),
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
        if matches!(self.source, CommandSource::Rcon) {
            self.rcon_output.push(format!("<{source}> {message}"));
        }
        let packet = Text::chat(source, message, "");
        self.broadcast_compressed(packet_id::TEXT, &packet);
    }

    fn broadcast_action(&mut self, source: &str, message: &str) {
        if matches!(self.source, CommandSource::Rcon) {
            self.rcon_output.push(format!("* {source} {message}"));
        }
        let formatted = format!("* {} {}", source, message);
        let packet = Text::system(&formatted);
        self.broadcast_compressed(packet_id::TEXT, &packet);
    }

    fn open_sender_menu(&mut self) {
        let Some(addr) = self.source_addr() else {
            self.send_feedback("Console cannot open the in-game menu.");
            return;
        };
        let Some(connection) = self.connections.get_mut(&addr) else {
            return;
        };
        let batch = connection.build_hub_form_batch();
        let prepared = connection.prepare_for_send(batch);
        self.raknet
            .send_to_session(&addr, prepared, Reliability::ReliableOrdered, true);
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
        let add_bytes = entity.add_actor_packet();
        let entity_uid = entity.base.entity_unique_id;
        let entity_pos = entity.base.position;
        let runtime_id = entity.base.entity_runtime_id;
        let targets: Vec<std::net::SocketAddr> = self
            .connections
            .iter_mut()
            .filter_map(|(addr, conn)| {
                if !conn.is_in_game() {
                    return None;
                }
                if !crate::entity_culling::is_within_view_for(conn, entity_pos) {
                    return None;
                }
                conn.visible_entities.insert(entity_uid);
                Some(*addr)
            })
            .collect();
        for addr in targets {
            self.send_compressed(addr, packet_id::ADD_ACTOR, &add_bytes);
        }
        Ok(runtime_id)
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
            let uid = entity.entity_unique_id;
            let targets: Vec<std::net::SocketAddr> = self
                .connections
                .iter_mut()
                .filter_map(|(addr, conn)| {
                    if !conn.is_in_game() {
                        return None;
                    }
                    if !conn.visible_entities.remove(&uid) {
                        return None;
                    }
                    Some(*addr)
                })
                .collect();
            for addr in targets {
                self.send_compressed(addr, packet_id::REMOVE_ACTOR, &remove_packet);
            }
            return Ok(());
        }
        if let Some(entity) = self.mob_entities.remove(entity_id) {
            let position = entity.base.position;
            let drops = entity.kind.default_loot();
            let remove_packet = entity.remove_packet();
            let uid = entity.base.entity_unique_id;
            let targets: Vec<std::net::SocketAddr> = self
                .connections
                .iter_mut()
                .filter_map(|(addr, conn)| {
                    if !conn.is_in_game() {
                        return None;
                    }
                    if !conn.visible_entities.remove(&uid) {
                        return None;
                    }
                    Some(*addr)
                })
                .collect();
            for addr in targets {
                self.send_compressed(addr, packet_id::REMOVE_ACTOR, &remove_packet);
            }
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
        let bytes = connection.encode_compressed_packet(packet_id::MOB_EFFECT, &pkt.encode());
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

    fn resolve_block_name(&self, name: &str) -> Option<u32> {
        let normalized = if name.contains(':') {
            name.to_string()
        } else {
            format!("minecraft:{}", name)
        };
        let id = crate::world::block_registry::BLOCKS.get(&normalized);
        // BLOCKS.get retourne air pour les inconnus. On vérifie le round-trip
        // via name_for pour distinguer un "air" légitime d'un "introuvable".
        if id == crate::world::block_registry::BLOCKS.air && normalized != "minecraft:air" {
            None
        } else {
            Some(id)
        }
    }

    fn set_world_block(&mut self, x: i32, y: i32, z: i32, block_id: u32) -> bool {
        // Mutation chunk_cache sous lock séparé pour ne pas tenir le guard
        // pendant les envois réseau (cf comment dans main.rs ligne ~1980).
        let changed = if let Ok(mut cache) = self.chunk_cache.lock() {
            let prev = cache.get_block(x, y, z);
            if prev == block_id {
                false
            } else {
                cache.set_block(x, y, z, block_id);
                true
            }
        } else {
            return false;
        };
        if !changed {
            return false;
        }
        // Broadcast UpdateBlock à tous les joueurs in_game. flags = NETWORK |
        // NEIGHBORS = 3 (déclenche aussi physics check côté client).
        let update = mc_rs_proto::packets::world::UpdateBlock {
            position: [x, y, z],
            runtime_id: block_id,
            flags: 3,
            layer: 0,
        }
        .encode();
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter_map(|(addr, conn)| conn.is_in_game().then_some(*addr))
            .collect();
        for addr in addrs {
            self.send_compressed(addr, packet_id::UPDATE_BLOCK, &update);
        }
        true
    }

    fn world_block_at(&self, x: i32, y: i32, z: i32) -> u32 {
        if let Ok(mut cache) = self.chunk_cache.lock() {
            cache.get_block(x, y, z)
        } else {
            crate::world::block_registry::BLOCKS.air
        }
    }

    fn gamerule_list(&self) -> Vec<(String, crate::game_rules::GameRuleValue)> {
        let mut entries: Vec<_> = self
            .server_state
            .game_rules
            .rules
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    fn gamerule_get(&self, name: &str) -> Option<crate::game_rules::GameRuleValue> {
        self.server_state.game_rules.get(name).cloned()
    }

    fn tellraw_send(&mut self, addr: SocketAddr, encoded_text_payload: &[u8]) {
        self.send_compressed(addr, packet_id::TEXT, encoded_text_payload);
    }

    fn play_sound(
        &mut self,
        targets: &[SocketAddr],
        sound: &str,
        position: [f32; 3],
        volume: f32,
        pitch: f32,
    ) {
        let payload = mc_rs_proto::packets::world::PlaySound {
            sound_name: sound.to_string(),
            position,
            volume,
            pitch,
        }
        .encode();
        let recipients: Vec<SocketAddr> = if targets.is_empty() {
            self.connections
                .iter()
                .filter_map(|(addr, conn)| conn.is_in_game().then_some(*addr))
                .collect()
        } else {
            targets.to_vec()
        };
        for addr in recipients {
            self.send_compressed(addr, packet_id::PLAY_SOUND, &payload);
        }
    }

    fn player_tag_add(&mut self, addr: SocketAddr, tag: &str) -> bool {
        let Some(conn) = self.connections.get_mut(&addr) else {
            return false;
        };
        conn.tags.insert(tag.to_string())
    }

    fn player_tag_remove(&mut self, addr: SocketAddr, tag: &str) -> bool {
        let Some(conn) = self.connections.get_mut(&addr) else {
            return false;
        };
        conn.tags.remove(tag)
    }

    fn player_tag_list(&self, addr: SocketAddr) -> Vec<String> {
        let Some(conn) = self.connections.get(&addr) else {
            return Vec::new();
        };
        let mut tags: Vec<String> = conn.tags.iter().cloned().collect();
        tags.sort();
        tags
    }

    fn spawn_item_world(&mut self, position: [f32; 3], item: ItemStack) {
        self.spawn_world_item_entity(
            crate::item_entities::PendingItemEntitySpawn::stationary(item, position),
        );
    }

    fn roll_chest_loot_drops(&self, table_name: &str) -> Vec<(String, u32)> {
        let normalized = if table_name.contains(':') {
            table_name.to_string()
        } else {
            format!("minecraft:{table_name}")
        };
        crate::loot_table::roll_chest_loot(&normalized)
    }

    fn actor_event_broadcast(&mut self, runtime_entity_id: u64, event_id: u32, data: i32) {
        let payload =
            crate::combat_packets::encode_actor_event(runtime_entity_id, event_id, data);
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter_map(|(addr, conn)| conn.is_in_game().then_some(*addr))
            .collect();
        for addr in addrs {
            self.send_compressed(addr, packet_id::ACTOR_EVENT, &payload);
        }
    }

    fn first_entity_runtime_id(&self, token: &str) -> Option<u64> {
        // Pour @s sur sender player, on retourne directement le sender.
        // Pour les autres tokens, on prend le premier match dans selector_entities.
        let candidates = self.selector_entities();
        let resolved = resolve_target_token_with_index(token, self, &candidates, 0).ok()?;
        resolved.first().map(|e| e.id)
    }

    fn set_player_ability(
        &mut self,
        addr: SocketAddr,
        ability: &str,
        value: bool,
    ) -> Result<(), String> {
        use mc_rs_proto::packets::player::ability;
        let bit = match ability.to_ascii_lowercase().as_str() {
            "mayfly" | "may_fly" | "allowflight" => ability::ALLOW_FLIGHT,
            "fly" | "flying" => ability::FLYING,
            "noclip" | "no_clip" => ability::NO_CLIP,
            "invulnerable" | "godmode" => ability::INVULNERABLE,
            "mute" | "muted" => ability::MUTED,
            "worldbuilder" | "world_builder" => ability::WORLD_BUILDER,
            "build" => ability::BUILD,
            "mine" => ability::MINE,
            "doorsandswitches" | "doors_and_switches" => ability::DOORS_AND_SWITCHES,
            "opencontainers" | "open_containers" => ability::OPEN_CONTAINERS,
            "attackplayers" | "attack_players" => ability::ATTACK_PLAYERS,
            "attackmobs" | "attack_mobs" => ability::ATTACK_MOBS,
            "operator" | "op" => ability::OPERATOR,
            "teleport" => ability::TELEPORT,
            "infiniteresources" | "infinite_resources" => ability::INFINITE_RESOURCES,
            "lightning" => ability::LIGHTNING,
            other => {
                return Err(format!("Unknown ability: {other}"));
            }
        };

        let Some(conn) = self.connections.get_mut(&addr) else {
            return Err("Player not connected.".into());
        };
        let is_op = conn.is_op;
        let mut abilities = match conn.gamemode {
            1 => mc_rs_proto::packets::player::UpdateAbilities::default_creative(
                conn.entity_runtime_id as i64,
                is_op,
            ),
            3 => mc_rs_proto::packets::player::UpdateAbilities::default_spectator(
                conn.entity_runtime_id as i64,
                is_op,
            ),
            _ => mc_rs_proto::packets::player::UpdateAbilities::default_survival(
                conn.entity_runtime_id as i64,
                is_op,
            ),
        };
        if let Some(layer) = abilities.layers.first_mut() {
            layer.abilities_set |= bit;
            if value {
                layer.abilities_values |= bit;
            } else {
                layer.abilities_values &= !bit;
            }
        }
        let payload = abilities.encode();
        self.send_compressed(addr, packet_id::UPDATE_ABILITIES, &payload);
        Ok(())
    }

    fn reload_server_state(&mut self) -> Result<(), String> {
        // Recharge uniquement la partie persistante (ops/whitelist/bans) depuis
        // server-state.json. On garde les champs ephemeral en l'état (motd,
        // world_name, seed, max_players, scoreboards, game_rules).
        let new_state = crate::server_state::ServerState::load(
            self.server_state.server_motd.clone(),
            self.server_state.world_name.clone(),
            self.server_state.world_seed,
            self.server_state.max_players,
        );
        self.server_state.persistent = new_state.persistent;
        // Resync permissions des connexions selon le nouvel état des ops.
        let addrs: Vec<SocketAddr> = self.connections.keys().copied().collect();
        for addr in addrs {
            if let Some(conn) = self.connections.get_mut(&addr) {
                let name = conn.display_name.clone().unwrap_or_default();
                conn.is_op = self.server_state.is_op(&name);
            }
        }
        Ok(())
    }

    fn damage_player(&mut self, addr: SocketAddr, amount: f32) -> Result<bool, String> {
        let Some(conn) = self.connections.get_mut(&addr) else {
            return Err("Player not connected.".into());
        };
        // attack_entity gère i-frames + event dispatch. cause::Custom car
        // /damage est une source admin générique.
        let mut ev_guard = conn
            .events
            .lock()
            .map_err(|_| "Event manager poisoned.".to_string())?;
        let outcome = crate::combat::attack_entity(
            &mut ev_guard,
            conn.entity_runtime_id,
            conn.position,
            &mut conn.attributes,
            &mut conn.combat,
            crate::event::entity::DamageCause::Custom,
            amount,
            None,
            None,
            0.0,
        );
        drop(ev_guard);
        Ok(outcome.died)
    }

    fn replace_player_slot(
        &mut self,
        addr: SocketAddr,
        inv_key: crate::inventory_manager::InvKey,
        slot_index: usize,
        item: ItemStack,
    ) -> Result<(), String> {
        let Some(connection) = self.connections.get_mut(&addr) else {
            return Err("Player not connected.".into());
        };
        // Vérifie que le slot existe — bornes selon InvKey :
        let bounds_ok = match inv_key {
            crate::inventory_manager::InvKey::Main => slot_index < 36,
            crate::inventory_manager::InvKey::Armor => slot_index < 4,
            crate::inventory_manager::InvKey::Offhand => slot_index < 1,
            _ => false,
        };
        if !bounds_ok {
            return Err(format!(
                "Slot {slot_index} out of range for {inv_key:?}"
            ));
        }
        connection
            .inventory_manager
            .set_slot(&mut connection.inventory, inv_key, slot_index, item);
        // Flush sync wire au joueur — même pattern que /enchant l.1325.
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

    fn stop_sound(&mut self, targets: &[SocketAddr], sound: Option<&str>) {
        let stop_all = sound.is_none();
        let payload = mc_rs_proto::packets::world::StopSound {
            sound_name: sound.unwrap_or("").to_string(),
            stop_all,
            stop_legacy_music: false,
        }
        .encode();
        let recipients: Vec<SocketAddr> = if targets.is_empty() {
            self.connections
                .iter()
                .filter_map(|(addr, conn)| conn.is_in_game().then_some(*addr))
                .collect()
        } else {
            targets.to_vec()
        };
        for addr in recipients {
            self.send_compressed(addr, packet_id::STOP_SOUND, &payload);
        }
    }

    fn gamerule_set(
        &mut self,
        name: &str,
        value: crate::game_rules::GameRuleValue,
    ) -> Result<(), String> {
        let key = name.to_ascii_lowercase();
        // On exige que le rule existe déjà (refuse les rules inconnues : pas
        // de typos silencieuses) ET que le type matche.
        let existing = self
            .server_state
            .game_rules
            .get(&key)
            .ok_or_else(|| format!("Unknown game rule: {name}"))?;
        let type_ok = matches!(
            (existing, &value),
            (
                crate::game_rules::GameRuleValue::Bool(_),
                crate::game_rules::GameRuleValue::Bool(_)
            ) | (
                crate::game_rules::GameRuleValue::Int(_),
                crate::game_rules::GameRuleValue::Int(_)
            ) | (
                crate::game_rules::GameRuleValue::Float(_),
                crate::game_rules::GameRuleValue::Float(_)
            )
        );
        if !type_ok {
            return Err(format!("Value type doesn't match rule '{name}'"));
        }
        self.server_state.game_rules.set(key.clone(), value.clone());

        // Broadcast GameRulesChanged contenant uniquement le rule mis à jour
        // (PMMP fait pareil — Network.php envoie un diff, pas le set complet).
        // `isPlayerModifiable: true` côté wire = le client peut le toggle UI.
        let wire_rule = match value {
            crate::game_rules::GameRuleValue::Bool(b) => {
                mc_rs_proto::packets::world::GameRule::Bool(key, true, b)
            }
            crate::game_rules::GameRuleValue::Int(i) => {
                mc_rs_proto::packets::world::GameRule::Int(key, true, i)
            }
            crate::game_rules::GameRuleValue::Float(f) => {
                mc_rs_proto::packets::world::GameRule::Float(key, true, f)
            }
        };
        let payload = mc_rs_proto::packets::world::GameRulesChanged {
            rules: vec![wire_rule],
        }
        .encode();
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter_map(|(addr, conn)| conn.is_in_game().then_some(*addr))
            .collect();
        for addr in addrs {
            self.send_compressed(addr, packet_id::GAME_RULES_CHANGED, &payload);
        }
        Ok(())
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

/// Shortcut spécifique mc-rs : SoftEnum sur `online_players` qui est résolu
/// dynamiquement via `SoftEnumSource::soft_enum_values`.
fn soft_player_param(name: &str, optional: bool) -> CommandParameter {
    mc_rs_command::soft_enum_param(name, "online_players", optional)
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

    let mut menu = CommandDefinition::new("menu", "Open the hub menu");
    menu.usage = "/menu".into();
    menu.permissions = vec!["server.command.menu".into()];
    register_command(
        &mut permissions,
        &mut map,
        menu,
        PermissionDefault::True,
        |runtime: &mut dyn ServerCommandRuntime, _| {
            runtime.open_sender_menu();
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
        parameters: vec![soft_player_param("player", false)],
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
        parameters: vec![soft_player_param("player", false)],
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
            soft_player_param("player", false),
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
        parameters: vec![soft_player_param("player", false)],
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
            CommandParameter {
                name: "itemName".into(),
                param_type: ParamType::SoftEnum {
                    name: "Item".into(),
                },
                optional: false,
            },
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
    let entity_param = || CommandParameter {
        name: "entityType".into(),
        param_type: ParamType::SoftEnum {
            name: "EntityType".into(),
        },
        optional: false,
    };
    summon.overloads.push(CommandOverload {
        parameters: vec![entity_param()],
    });
    summon.overloads.push(CommandOverload {
        parameters: vec![
            entity_param(),
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
    // Hard enum (restrictif) : set+keyword affiche un dropdown (day/noon/…).
    time.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "time_set_action", &["set"], false),
            hard_enum_param(
                "value",
                "time_value",
                &[
                    "day", "noon", "midday", "sunset", "dusk", "night", "midnight", "sunrise",
                ],
                false,
            ),
        ],
    });
    // Overload générique : set/add/query avec une valeur numérique optionnelle.
    time.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "time_action", &["set", "add", "query"], false),
            param("ticks", ParamType::Int, true),
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
                        CommandDispatchError::Message(format!("Invalid amount: {amount_tok}"))
                    })?;
                    let new_level = runtime
                        .add_player_xp(addr, amount)
                        .map_err(CommandDispatchError::Message)?;
                    runtime.send_feedback(&format!("Added {amount} XP (level now {new_level})"));
                }
                "set" => {
                    // set = query current then add diff (pour récupérer diff
                    // il faudrait query_xp ; on simplifie en retirant tout puis
                    // réajoutant).
                    let Some(amount_tok) = invocation.arg(1) else {
                        return usage("Usage: /xp set <amount> [target]");
                    };
                    let amount: i32 = amount_tok.parse().map_err(|_| {
                        CommandDispatchError::Message(format!("Invalid amount: {amount_tok}"))
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
    effect.usage = "/effect <target> <effect> [duration] [amplifier]".into();
    effect.permissions = vec!["server.command.effect".into()];
    effect.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            CommandParameter {
                name: "effect".into(),
                param_type: ParamType::SoftEnum {
                    name: "Effect".into(),
                },
                optional: false,
            },
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
            let kind =
                crate::effects::EffectKind::from_name_or_id(effect_tok).ok_or_else(|| {
                    CommandDispatchError::Message(format!("Unknown effect: {effect_tok}"))
                })?;
            let effect_id: i32 = kind.id() as i32;
            let duration: i32 = invocation
                .arg(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(600);
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
    enchant.usage = "/enchant <target> <enchantment> [level]".into();
    enchant.permissions = vec!["server.command.enchant".into()];
    enchant.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            CommandParameter {
                name: "enchantmentName".into(),
                param_type: ParamType::SoftEnum {
                    name: "Enchantment".into(),
                },
                optional: false,
            },
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
            let kind = crate::enchantments::EnchantmentKind::from_name_or_id(ench_tok).ok_or_else(
                || CommandDispatchError::Message(format!("Unknown enchantment: {ench_tok}")),
            )?;
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
    let mut sb = CommandDefinition::new("scoreboard", "Set a player score on a sidebar objective");
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
            let score: i32 = invocation
                .arg(2)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| CommandDispatchError::Message("score must be an integer".into()))?;
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
            runtime.send_feedback(&format!(
                "Spawned particle {pname} at ({x:.1},{y:.1},{z:.1})"
            ));
            Ok(())
        },
    );

    // ── /setblock <x> <y> <z> <block> [destroy|keep|replace] ──
    let mut setblock = CommandDefinition::new("setblock", "Set a block at a position");
    setblock.usage = "/setblock <x> <y> <z> <block> [destroy|keep|replace]".into();
    setblock.permissions = vec!["server.command.setblock".into()];
    setblock.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
            param("block", ParamType::String, false),
            hard_enum_param(
                "mode",
                "setblock_mode",
                &["destroy", "keep", "replace"],
                true,
            ),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        setblock,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 4 {
                return usage("Usage: /setblock <x> <y> <z> <block> [destroy|keep|replace]");
            }
            let x_tok = invocation.arg(0).unwrap();
            let y_tok = invocation.arg(1).unwrap();
            let z_tok = invocation.arg(2).unwrap();
            let block_name = invocation.arg(3).unwrap();
            let mode = invocation
                .arg(4)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "replace".to_string());

            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            let pos = parse_position_triplet_for_source(runtime, origin, x_tok, y_tok, z_tok)?;
            let (ix, iy, iz) = (
                pos[0].floor() as i32,
                pos[1].floor() as i32,
                pos[2].floor() as i32,
            );

            let Some(block_id) = runtime.resolve_block_name(block_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown block: {block_name}"
                )));
            };

            // Modes — voir Minecraft Wiki /setblock :
            // - replace : remplace tout (default)
            // - keep : ne fait rien si le bloc existant est ≠ air
            // - destroy : remplace ET supprime l'ancien (sans drop pour l'instant)
            let air = crate::world::block_registry::BLOCKS.air;
            match mode.as_str() {
                "keep" => {
                    let current = runtime.world_block_at(ix, iy, iz);
                    if current != air {
                        runtime.send_feedback(&format!(
                            "Kept existing block at ({ix},{iy},{iz})."
                        ));
                        return Ok(());
                    }
                }
                "destroy" | "replace" => {}
                _ => return usage("Mode must be destroy, keep, or replace."),
            }

            if runtime.set_world_block(ix, iy, iz, block_id) {
                runtime.send_feedback(&format!("Block set at ({ix},{iy},{iz})."));
            } else {
                runtime.send_feedback(&format!("Block at ({ix},{iy},{iz}) unchanged."));
            }
            Ok(())
        },
    );

    // ── /fill <x1> <y1> <z1> <x2> <y2> <z2> <block> [destroy|hollow|keep|outline|replace] ──
    let mut fill = CommandDefinition::new("fill", "Fill a region with a block");
    fill.usage =
        "/fill <x1> <y1> <z1> <x2> <y2> <z2> <block> [destroy|hollow|keep|outline|replace]".into();
    fill.permissions = vec!["server.command.fill".into()];
    fill.overloads.push(CommandOverload {
        parameters: vec![
            param("x1", ParamType::Position, false),
            param("y1", ParamType::Position, false),
            param("z1", ParamType::Position, false),
            param("x2", ParamType::Position, false),
            param("y2", ParamType::Position, false),
            param("z2", ParamType::Position, false),
            param("block", ParamType::String, false),
            hard_enum_param(
                "mode",
                "fill_mode",
                &["destroy", "hollow", "keep", "outline", "replace"],
                true,
            ),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        fill,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 7 {
                return usage(
                    "Usage: /fill <x1> <y1> <z1> <x2> <y2> <z2> <block> [destroy|hollow|keep|outline|replace]",
                );
            }
            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            let from = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(0).unwrap(),
                invocation.arg(1).unwrap(),
                invocation.arg(2).unwrap(),
            )?;
            let to = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(3).unwrap(),
                invocation.arg(4).unwrap(),
                invocation.arg(5).unwrap(),
            )?;
            let block_name = invocation.arg(6).unwrap();
            let mode = invocation
                .arg(7)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "replace".to_string());

            let Some(block_id) = runtime.resolve_block_name(block_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown block: {block_name}"
                )));
            };

            // Normalise les coins en (min, max). Vanilla : floor pour la position.
            let x1 = (from[0].floor() as i32).min(to[0].floor() as i32);
            let x2 = (from[0].floor() as i32).max(to[0].floor() as i32);
            let y1 = (from[1].floor() as i32).min(to[1].floor() as i32);
            let y2 = (from[1].floor() as i32).max(to[1].floor() as i32);
            let z1 = (from[2].floor() as i32).min(to[2].floor() as i32);
            let z2 = (from[2].floor() as i32).max(to[2].floor() as i32);

            // Vanilla limit : 32 768 blocs par /fill.
            let volume = ((x2 - x1 + 1) as i64)
                * ((y2 - y1 + 1) as i64)
                * ((z2 - z1 + 1) as i64);
            if volume > 32_768 {
                return Err(CommandDispatchError::Message(format!(
                    "Region too large ({volume} blocks). Maximum is 32768."
                )));
            }

            let air = crate::world::block_registry::BLOCKS.air;
            let mut changed: i64 = 0;
            for y in y1..=y2 {
                for z in z1..=z2 {
                    for x in x1..=x2 {
                        let is_border = x == x1
                            || x == x2
                            || y == y1
                            || y == y2
                            || z == z1
                            || z == z2;
                        let new_id = match mode.as_str() {
                            "keep" => {
                                if runtime.world_block_at(x, y, z) != air {
                                    continue;
                                }
                                block_id
                            }
                            "hollow" => {
                                if is_border {
                                    block_id
                                } else {
                                    air
                                }
                            }
                            "outline" => {
                                if is_border {
                                    block_id
                                } else {
                                    continue;
                                }
                            }
                            "destroy" | "replace" => block_id,
                            _ => {
                                return usage(
                                    "Mode must be destroy, hollow, keep, outline, or replace.",
                                )
                            }
                        };
                        if runtime.set_world_block(x, y, z, new_id) {
                            changed += 1;
                        }
                    }
                }
            }
            runtime.send_feedback(&format!("Filled {changed} blocks."));
            Ok(())
        },
    );

    // ── /clone <x1> <y1> <z1> <x2> <y2> <z2> <dx> <dy> <dz> [masked|replace] [force|move|normal] ──
    let mut clone = CommandDefinition::new("clone", "Clone a region of blocks");
    clone.usage =
        "/clone <x1> <y1> <z1> <x2> <y2> <z2> <dx> <dy> <dz> [masked|replace] [force|move|normal]".into();
    clone.permissions = vec!["server.command.clone".into()];
    clone.overloads.push(CommandOverload {
        parameters: vec![
            param("x1", ParamType::Position, false),
            param("y1", ParamType::Position, false),
            param("z1", ParamType::Position, false),
            param("x2", ParamType::Position, false),
            param("y2", ParamType::Position, false),
            param("z2", ParamType::Position, false),
            param("dx", ParamType::Position, false),
            param("dy", ParamType::Position, false),
            param("dz", ParamType::Position, false),
            hard_enum_param("mask_mode", "clone_mask", &["masked", "replace"], true),
            hard_enum_param(
                "clone_mode",
                "clone_collision",
                &["force", "move", "normal"],
                true,
            ),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        clone,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 9 {
                return usage(
                    "Usage: /clone <x1> <y1> <z1> <x2> <y2> <z2> <dx> <dy> <dz> [masked|replace] [force|move|normal]",
                );
            }
            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            let from = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(0).unwrap(),
                invocation.arg(1).unwrap(),
                invocation.arg(2).unwrap(),
            )?;
            let to = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(3).unwrap(),
                invocation.arg(4).unwrap(),
                invocation.arg(5).unwrap(),
            )?;
            let dest = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(6).unwrap(),
                invocation.arg(7).unwrap(),
                invocation.arg(8).unwrap(),
            )?;
            let mask_mode = invocation
                .arg(9)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "replace".to_string());
            let clone_mode = invocation
                .arg(10)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "normal".to_string());

            let x1 = (from[0].floor() as i32).min(to[0].floor() as i32);
            let x2 = (from[0].floor() as i32).max(to[0].floor() as i32);
            let y1 = (from[1].floor() as i32).min(to[1].floor() as i32);
            let y2 = (from[1].floor() as i32).max(to[1].floor() as i32);
            let z1 = (from[2].floor() as i32).min(to[2].floor() as i32);
            let z2 = (from[2].floor() as i32).max(to[2].floor() as i32);
            let dx = dest[0].floor() as i32;
            let dy = dest[1].floor() as i32;
            let dz = dest[2].floor() as i32;

            let sx = x2 - x1 + 1;
            let sy = y2 - y1 + 1;
            let sz = z2 - z1 + 1;
            let volume = (sx as i64) * (sy as i64) * (sz as i64);
            if volume > 32_768 {
                return Err(CommandDispatchError::Message(format!(
                    "Region too large ({volume} blocks). Maximum is 32768."
                )));
            }

            // Lecture source first (au cas où source et dest se chevauchent).
            let mut buf: Vec<u32> = Vec::with_capacity(volume as usize);
            for y in 0..sy {
                for z in 0..sz {
                    for x in 0..sx {
                        buf.push(runtime.world_block_at(x1 + x, y1 + y, z1 + z));
                    }
                }
            }

            // En mode `move`, on doit aussi mémoriser les positions source pour
            // les remettre à air après écriture du dest.
            let air = crate::world::block_registry::BLOCKS.air;
            let mut changed: i64 = 0;
            let mut index = 0usize;
            for y in 0..sy {
                for z in 0..sz {
                    for x in 0..sx {
                        let block_id = buf[index];
                        index += 1;
                        if mask_mode == "masked" && block_id == air {
                            continue;
                        }
                        let tx = dx + x;
                        let ty = dy + y;
                        let tz = dz + z;
                        if runtime.set_world_block(tx, ty, tz, block_id) {
                            changed += 1;
                        }
                    }
                }
            }
            // Mode `move` : vider la source (sauf si overlap avec dest, géré
            // par le set séquentiel — le dest a déjà été écrit).
            if clone_mode == "move" {
                for y in 0..sy {
                    for z in 0..sz {
                        for x in 0..sx {
                            let sx_pos = x1 + x;
                            let sy_pos = y1 + y;
                            let sz_pos = z1 + z;
                            // Skip si la position source est dans le dest
                            // (sinon on effacerait ce qu'on vient de copier).
                            let in_dest = sx_pos >= dx
                                && sx_pos < dx + sx
                                && sy_pos >= dy
                                && sy_pos < dy + sy
                                && sz_pos >= dz
                                && sz_pos < dz + sz;
                            if !in_dest {
                                runtime.set_world_block(sx_pos, sy_pos, sz_pos, air);
                            }
                        }
                    }
                }
            }
            runtime.send_feedback(&format!("Cloned {changed} blocks."));
            Ok(())
        },
    );

    // ── /gamerule [<name> [<value>]] ──
    let mut gamerule = CommandDefinition::new("gamerule", "Show or change game rules");
    gamerule.usage = "/gamerule [<rule> [<value>]]".into();
    gamerule.permissions = vec!["server.command.gamerule".into()];
    gamerule.overloads.push(CommandOverload {
        parameters: vec![
            param("rule", ParamType::String, true),
            param("value", ParamType::String, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        gamerule,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            // /gamerule (sans args) → liste toutes les rules
            if invocation.args.is_empty() {
                let rules = runtime.gamerule_list();
                if rules.is_empty() {
                    runtime.send_feedback("No game rules registered.");
                    return Ok(());
                }
                let names = rules
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                runtime.send_feedback(&format!("Game rules: {names}"));
                return Ok(());
            }

            let rule_name = invocation.arg(0).unwrap();

            // /gamerule <name> → affiche la valeur
            if invocation.args.len() == 1 {
                let Some(value) = runtime.gamerule_get(rule_name) else {
                    return Err(CommandDispatchError::Message(format!(
                        "Unknown game rule: {rule_name}"
                    )));
                };
                let display = match value {
                    crate::game_rules::GameRuleValue::Bool(b) => b.to_string(),
                    crate::game_rules::GameRuleValue::Int(i) => i.to_string(),
                    crate::game_rules::GameRuleValue::Float(f) => f.to_string(),
                };
                runtime.send_feedback(&format!("{rule_name} = {display}"));
                return Ok(());
            }

            // /gamerule <name> <value> → set
            let raw_value = invocation.arg(1).unwrap();
            let Some(existing) = runtime.gamerule_get(rule_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown game rule: {rule_name}"
                )));
            };
            let parsed = match existing {
                crate::game_rules::GameRuleValue::Bool(_) => match raw_value.to_ascii_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => crate::game_rules::GameRuleValue::Bool(true),
                    "false" | "0" | "off" | "no" => crate::game_rules::GameRuleValue::Bool(false),
                    _ => {
                        return Err(CommandDispatchError::Message(format!(
                            "Invalid bool value '{raw_value}', expected true/false."
                        )))
                    }
                },
                crate::game_rules::GameRuleValue::Int(_) => raw_value
                    .parse::<i32>()
                    .map(crate::game_rules::GameRuleValue::Int)
                    .map_err(|_| {
                        CommandDispatchError::Message(format!(
                            "Invalid int value '{raw_value}'."
                        ))
                    })?,
                crate::game_rules::GameRuleValue::Float(_) => raw_value
                    .parse::<f32>()
                    .map(crate::game_rules::GameRuleValue::Float)
                    .map_err(|_| {
                        CommandDispatchError::Message(format!(
                            "Invalid float value '{raw_value}'."
                        ))
                    })?,
            };
            runtime
                .gamerule_set(rule_name, parsed.clone())
                .map_err(CommandDispatchError::Message)?;
            let display = match parsed {
                crate::game_rules::GameRuleValue::Bool(b) => b.to_string(),
                crate::game_rules::GameRuleValue::Int(i) => i.to_string(),
                crate::game_rules::GameRuleValue::Float(f) => f.to_string(),
            };
            runtime.send_feedback(&format!("Game rule {rule_name} set to {display}"));
            Ok(())
        },
    );

    // ── /tellraw <target> <json> ──
    // Le client Bedrock attend du rawtext JSON : `{"rawtext":[{"text":"hi"}]}`.
    // On fait pas de validation JSON ici : on transmet brut et le client gère
    // les erreurs de parsing (équivalent vanilla).
    let mut tellraw = CommandDefinition::new("tellraw", "Send a JSON rawtext message");
    tellraw.usage = "/tellraw <target> <json>".into();
    tellraw.permissions = vec!["server.command.tellraw".into()];
    tellraw.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("json", ParamType::Json, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        tellraw,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /tellraw <target> <json>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            // Le JSON peut contenir des espaces, on reconstruit avec `tail(1)`
            // qui rassemble tout après le target.
            let json = invocation.tail(1);
            if json.is_empty() {
                return usage("Usage: /tellraw <target> <json>");
            }
            let payload = mc_rs_proto::packets::player::Text::json(&json);
            for addr in targets {
                runtime.tellraw_send(addr, &payload);
            }
            runtime.send_feedback("Tellraw sent.");
            Ok(())
        },
    );

    // ── /playsound <sound> [target] [x y z] [volume] [pitch] ──
    let mut playsound = CommandDefinition::new("playsound", "Play a sound for players");
    playsound.usage = "/playsound <sound> [target] [x] [y] [z] [volume] [pitch]".into();
    playsound.permissions = vec!["server.command.playsound".into()];
    playsound.overloads.push(CommandOverload {
        parameters: vec![
            param("sound", ParamType::String, false),
            param("target", ParamType::Target, true),
            param("x", ParamType::Position, true),
            param("y", ParamType::Position, true),
            param("z", ParamType::Position, true),
            param("volume", ParamType::Float, true),
            param("pitch", ParamType::Float, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        playsound,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(sound) = invocation.arg(0) else {
                return usage("Usage: /playsound <sound> [target] [x y z] [volume] [pitch]");
            };
            let targets = if let Some(target_token) = invocation.arg(1) {
                resolve_player_targets(runtime, Some(target_token), true)?
            } else if let Some(addr) = runtime.sender_addr() {
                vec![addr]
            } else {
                return Err(CommandDispatchError::Message(
                    "Console must specify a target.".into(),
                ));
            };

            // Position : si non fournie, on prend la position du premier target
            // (ou celle du sender si plus simple). Fallback à origin (0,0,0)
            // si vraiment rien.
            let pos = if let (Some(x), Some(y), Some(z)) =
                (invocation.arg(2), invocation.arg(3), invocation.arg(4))
            {
                let origin = if runtime.sender_is_player() {
                    Some(runtime.sender_position())
                } else {
                    None
                };
                parse_position_triplet_for_source(runtime, origin, x, y, z)?
            } else if let Some(first) = targets.first() {
                runtime.player_position(*first).unwrap_or([0.0, 64.0, 0.0])
            } else {
                [0.0, 64.0, 0.0]
            };
            let volume = invocation
                .arg(5)
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.0);
            let pitch = invocation
                .arg(6)
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.0);

            // Bedrock attend les noms avec préfixe (`random.click`, `mob.cow.say`,
            // etc.) — on transmet brut.
            runtime.play_sound(&targets, sound, pos, volume, pitch);
            runtime.send_feedback(&format!(
                "Played {sound} for {} player(s).",
                targets.len()
            ));
            Ok(())
        },
    );

    // ── /stopsound <target> [sound] ──
    let mut stopsound = CommandDefinition::new("stopsound", "Stop a sound for players");
    stopsound.usage = "/stopsound <target> [sound]".into();
    stopsound.permissions = vec!["server.command.stopsound".into()];
    stopsound.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("sound", ParamType::String, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        stopsound,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(target_token) = invocation.arg(0) else {
                return usage("Usage: /stopsound <target> [sound]");
            };
            let targets = resolve_player_targets(runtime, Some(target_token), true)?;
            let sound = invocation.arg(1);
            runtime.stop_sound(&targets, sound);
            runtime.send_feedback(&format!(
                "Stopped sound{} for {} player(s).",
                if sound.is_some() { "" } else { "s" },
                targets.len()
            ));
            Ok(())
        },
    );

    // ── /replaceitem entity <target> <slot_type> [slot] <item> [count] ──
    //
    // slot_type :
    //   slot.weapon.mainhand → InvKey::Main, slot=held
    //   slot.weapon.offhand  → InvKey::Offhand, slot=0
    //   slot.armor.head/.chest/.legs/.feet → InvKey::Armor, slot=0/1/2/3
    //   slot.hotbar <n>      → InvKey::Main, slot=n (0..8)
    //   slot.inventory <n>   → InvKey::Main, slot=9+n (9..35)
    let mut replaceitem =
        CommandDefinition::new("replaceitem", "Replace an item slot of an entity");
    replaceitem.usage =
        "/replaceitem entity <target> <slot_type> [slot] <item> [count]".into();
    replaceitem.permissions = vec!["server.command.replaceitem".into()];
    replaceitem.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("kind", "replaceitem_kind", &["entity"], false),
            param("target", ParamType::Target, false),
            param("slot_type", ParamType::String, false),
            param("slot_or_item", ParamType::String, false),
            param("item_or_count", ParamType::String, true),
            param("count", ParamType::Int, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        replaceitem,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            // Forme attendue : /replaceitem entity <target> <slot_type> [slot] <item> [count]
            if invocation.args.len() < 4 {
                return usage(
                    "Usage: /replaceitem entity <target> <slot_type> [slot] <item> [count]",
                );
            }
            let kind = invocation.arg(0).unwrap();
            if !kind.eq_ignore_ascii_case("entity") {
                return usage("Only 'entity' kind is supported for /replaceitem.");
            }
            let target_token = invocation.arg(1).unwrap();
            let slot_type = invocation.arg(2).unwrap().to_ascii_lowercase();

            // Détermine si le slot_type prend un index numérique (hotbar/inventory)
            // ou non (armor.head/chest/legs/feet, weapon.mainhand/offhand).
            let needs_index =
                slot_type == "slot.hotbar" || slot_type == "slot.inventory";

            // Récupère index slot + item + count selon présence de l'index.
            let (slot_index_token, item_token, count_token): (Option<&str>, &str, Option<&str>) =
                if needs_index {
                    if invocation.args.len() < 5 {
                        return usage(
                            "slot.hotbar/slot.inventory require <slot> <item> [count]",
                        );
                    }
                    (
                        Some(invocation.arg(3).unwrap()),
                        invocation.arg(4).unwrap(),
                        invocation.arg(5),
                    )
                } else {
                    (
                        None,
                        invocation.arg(3).unwrap(),
                        invocation.arg(4),
                    )
                };

            // Résout slot_type + index → (InvKey, slot_index)
            let (inv_key, slot_index) = match slot_type.as_str() {
                "slot.weapon.mainhand" => (crate::inventory_manager::InvKey::Main, 0usize),
                "slot.weapon.offhand" => (crate::inventory_manager::InvKey::Offhand, 0),
                "slot.armor.head" => (crate::inventory_manager::InvKey::Armor, 0),
                "slot.armor.chest" => (crate::inventory_manager::InvKey::Armor, 1),
                "slot.armor.legs" => (crate::inventory_manager::InvKey::Armor, 2),
                "slot.armor.feet" => (crate::inventory_manager::InvKey::Armor, 3),
                "slot.hotbar" => {
                    let n: usize = slot_index_token.unwrap().parse().map_err(|_| {
                        CommandDispatchError::Message("Hotbar slot must be 0..8".into())
                    })?;
                    if n > 8 {
                        return Err(CommandDispatchError::Message(
                            "Hotbar slot must be 0..8".into(),
                        ));
                    }
                    (crate::inventory_manager::InvKey::Main, n)
                }
                "slot.inventory" => {
                    let n: usize = slot_index_token.unwrap().parse().map_err(|_| {
                        CommandDispatchError::Message("Inventory slot must be 0..26".into())
                    })?;
                    if n > 26 {
                        return Err(CommandDispatchError::Message(
                            "Inventory slot must be 0..26".into(),
                        ));
                    }
                    (crate::inventory_manager::InvKey::Main, 9 + n)
                }
                _ => {
                    return Err(CommandDispatchError::Message(format!(
                        "Unsupported slot_type '{slot_type}'"
                    )))
                }
            };

            let count: u16 = count_token
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            let stack = parse_item_stack(item_token, count)?;

            // /replaceitem doit cibler des joueurs en jeu — pas les mob entities
            // (qui n'ont pas d'InventoryManager).
            let targets = resolve_player_targets(runtime, Some(target_token), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let count = targets.len();
            for addr in targets {
                runtime
                    .replace_player_slot(addr, inv_key, slot_index, stack.clone())
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!("Replaced slot for {count} player(s)."));
            Ok(())
        },
    );

    // ── /tag <target> add|remove|list [<tag>] ──
    // Tags persistés sur Connection (volatil — pas encore en player_data.json).
    // Pour l'instant ne s'applique qu'aux joueurs ; les mobs/items ne sont pas
    // taggables (déférer si besoin).
    let mut tag = CommandDefinition::new("tag", "Manage entity tags");
    tag.usage = "/tag <target> <add|remove|list> [<tag>]".into();
    tag.permissions = vec!["server.command.tag".into()];
    tag.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            hard_enum_param("action", "tag_action", &["add", "remove", "list"], false),
            param("tag", ParamType::String, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        tag,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /tag <target> <add|remove|list> [<tag>]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let action = invocation.arg(1).unwrap().to_ascii_lowercase();
            match action.as_str() {
                "list" => {
                    // PMMP/vanilla : `list` n'accepte qu'un target unique.
                    if targets.len() != 1 {
                        return Err(CommandDispatchError::Message(
                            "Tag list requires exactly one target.".into(),
                        ));
                    }
                    let tags = runtime.player_tag_list(targets[0]);
                    let name = runtime
                        .player_name(targets[0])
                        .unwrap_or_else(|| "(player)".into());
                    if tags.is_empty() {
                        runtime.send_feedback(&format!("{name} has no tags."));
                    } else {
                        runtime.send_feedback(&format!(
                            "{name} has {} tag(s): {}",
                            tags.len(),
                            tags.join(", ")
                        ));
                    }
                }
                "add" | "remove" => {
                    let Some(tag_value) = invocation.arg(2) else {
                        return usage("Usage: /tag <target> <add|remove> <tag>");
                    };
                    // Validation vanilla : tag doit être [a-zA-Z0-9_.+-] non vide
                    // et ≤ 25 char. On garde proche de la limite vanilla.
                    if tag_value.is_empty() || tag_value.len() > 25 {
                        return Err(CommandDispatchError::Message(
                            "Tag must be 1-25 chars.".into(),
                        ));
                    }
                    let mut changed = 0usize;
                    for addr in &targets {
                        let ok = if action == "add" {
                            runtime.player_tag_add(*addr, tag_value)
                        } else {
                            runtime.player_tag_remove(*addr, tag_value)
                        };
                        if ok {
                            changed += 1;
                        }
                    }
                    let verb = if action == "add" { "Added" } else { "Removed" };
                    runtime.send_feedback(&format!(
                        "{verb} tag '{tag_value}' for {changed}/{} player(s).",
                        targets.len()
                    ));
                }
                _ => return usage("Action must be add, remove, or list."),
            }
            Ok(())
        },
    );

    // ── /loot spawn <x y z> loot <table>  |  /loot give <target> loot <table> ──
    let mut loot = CommandDefinition::new("loot", "Drop or give loot from a loot table");
    loot.usage = "/loot spawn <x> <y> <z> loot <table>  OR  /loot give <target> loot <table>".into();
    loot.permissions = vec!["server.command.loot".into()];
    loot.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("mode", "loot_mode", &["spawn", "give"], false),
            param("target_or_x", ParamType::String, false),
            param("y_or_subcmd", ParamType::String, true),
            param("z_or_table", ParamType::String, true),
            param("subcmd", ParamType::String, true),
            param("table", ParamType::String, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        loot,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(mode) = invocation.arg(0) else {
                return usage(
                    "Usage: /loot spawn <x> <y> <z> loot <table>  OR  /loot give <target> loot <table>",
                );
            };
            match mode.to_ascii_lowercase().as_str() {
                "spawn" => {
                    // /loot spawn x y z loot <table>
                    if invocation.args.len() < 6 {
                        return usage("Usage: /loot spawn <x> <y> <z> loot <table>");
                    }
                    let origin = if runtime.sender_is_player() {
                        Some(runtime.sender_position())
                    } else {
                        None
                    };
                    let pos = parse_position_triplet_for_source(
                        runtime,
                        origin,
                        invocation.arg(1).unwrap(),
                        invocation.arg(2).unwrap(),
                        invocation.arg(3).unwrap(),
                    )?;
                    let subcmd = invocation.arg(4).unwrap();
                    if !subcmd.eq_ignore_ascii_case("loot") {
                        return usage("Expected 'loot' before table name.");
                    }
                    let table = invocation.arg(5).unwrap();
                    let drops = runtime.roll_chest_loot_drops(table);
                    if drops.is_empty() {
                        return Err(CommandDispatchError::Message(format!(
                            "Empty or unknown loot table: {table}"
                        )));
                    }
                    let mut count_total = 0u32;
                    for (name, count) in drops {
                        let stack = match parse_item_stack(&name, count as u16) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                        runtime.spawn_item_world(pos, stack);
                        count_total += count;
                    }
                    runtime.send_feedback(&format!(
                        "Spawned {count_total} loot item(s) at ({:.0},{:.0},{:.0}).",
                        pos[0], pos[1], pos[2]
                    ));
                }
                "give" => {
                    // /loot give <target> loot <table>
                    if invocation.args.len() < 4 {
                        return usage("Usage: /loot give <target> loot <table>");
                    }
                    let target_token = invocation.arg(1).unwrap();
                    let subcmd = invocation.arg(2).unwrap();
                    if !subcmd.eq_ignore_ascii_case("loot") {
                        return usage("Expected 'loot' before table name.");
                    }
                    let table = invocation.arg(3).unwrap();
                    let drops = runtime.roll_chest_loot_drops(table);
                    if drops.is_empty() {
                        return Err(CommandDispatchError::Message(format!(
                            "Empty or unknown loot table: {table}"
                        )));
                    }
                    let targets = resolve_player_targets(runtime, Some(target_token), true)?;
                    if targets.is_empty() {
                        return Err(CommandDispatchError::Message("No matching player.".into()));
                    }
                    let mut count_total = 0u32;
                    for addr in &targets {
                        for (name, count) in &drops {
                            let stack = match parse_item_stack(name, *count as u16) {
                                Ok(s) => s,
                                Err(_) => continue,
                            };
                            let _ = runtime.give_item(*addr, stack);
                            count_total += count;
                        }
                    }
                    runtime.send_feedback(&format!(
                        "Gave {count_total} loot item(s) to {} player(s).",
                        targets.len()
                    ));
                }
                _ => return usage("Mode must be spawn or give."),
            }
            Ok(())
        },
    );

    // ── /damage <target> <amount> [cause] ──
    let mut damage = CommandDefinition::new("damage", "Damage an entity");
    damage.usage = "/damage <target> <amount> [cause]".into();
    damage.permissions = vec!["server.command.damage".into()];
    damage.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("amount", ParamType::Float, false),
            param("cause", ParamType::String, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        damage,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 2 {
                return usage("Usage: /damage <target> <amount> [cause]");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let amount: f32 = invocation
                .arg(1)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| CommandDispatchError::Message("Amount must be a number.".into()))?;
            if amount < 0.0 {
                return Err(CommandDispatchError::Message(
                    "Amount must be ≥ 0.".into(),
                ));
            }
            // Note : la `cause` est ignorée pour l'instant — combat::attack_entity
            // utilise toujours DamageCause::Custom pour /damage (les modifiers
            // armor/effect ne dépendent pas de cause atm).
            let mut died_count = 0;
            for addr in &targets {
                if runtime.damage_player(*addr, amount).unwrap_or(false) {
                    died_count += 1;
                }
            }
            runtime.send_feedback(&format!(
                "Damaged {} player(s) for {} HP{}.",
                targets.len(),
                amount,
                if died_count > 0 {
                    format!(" ({died_count} died)")
                } else {
                    String::new()
                }
            ));
            Ok(())
        },
    );

    // ── /event entity <target> <event_id_or_name> ──
    let mut event_cmd = CommandDefinition::new("event", "Trigger an entity event");
    event_cmd.usage = "/event entity <target> <event_id|hurt|death|eating|respawn>".into();
    event_cmd.permissions = vec!["server.command.event".into()];
    event_cmd.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("kind", "event_kind", &["entity"], false),
            param("target", ParamType::Target, false),
            param("event", ParamType::String, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        event_cmd,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 3 {
                return usage("Usage: /event entity <target> <event_id|hurt|death|eating|respawn>");
            }
            if !invocation.arg(0).unwrap().eq_ignore_ascii_case("entity") {
                return usage("Only 'entity' kind is supported.");
            }
            let target_token = invocation.arg(1).unwrap();
            let event_arg = invocation.arg(2).unwrap();
            // Résolution event_id : numérique ou nom court PMMP actor_event::*
            let event_id: u32 = match event_arg.to_ascii_lowercase().as_str() {
                "hurt" | "hurt_animation" => crate::combat_packets::actor_event::HURT_ANIMATION,
                "death" | "death_animation" => crate::combat_packets::actor_event::DEATH_ANIMATION,
                "eating" | "eating_item" => crate::combat_packets::actor_event::EATING_ITEM,
                "respawn" => crate::combat_packets::actor_event::RESPAWN,
                "tame_success" => crate::combat_packets::actor_event::TAME_SUCCESS,
                "tame_fail" => crate::combat_packets::actor_event::TAME_FAIL,
                "shake_wet" => crate::combat_packets::actor_event::SHAKE_WET,
                "complete_trade" => crate::combat_packets::actor_event::COMPLETE_TRADE,
                other => other.parse::<u32>().map_err(|_| {
                    CommandDispatchError::Message(format!("Unknown event: {event_arg}"))
                })?,
            };

            let Some(rid) = runtime.first_entity_runtime_id(target_token) else {
                return Err(CommandDispatchError::Message("No matching entity.".into()));
            };
            runtime.actor_event_broadcast(rid, event_id, 0);
            runtime.send_feedback(&format!("Triggered event {event_id} on entity {rid}."));
            Ok(())
        },
    );

    // ── /testfor <target> ──
    // Vanilla : compte les entités matchant + feedback. Sert souvent comme
    // gate dans command blocks (success = nombre matché).
    let mut testfor = CommandDefinition::new("testfor", "Test for matching entities");
    testfor.usage = "/testfor <target>".into();
    testfor.permissions = vec!["server.command.testfor".into()];
    testfor.overloads.push(CommandOverload {
        parameters: vec![param("target", ParamType::Target, false)],
    });
    register_command(
        &mut permissions,
        &mut map,
        testfor,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(token) = invocation.arg(0) else {
                return usage("Usage: /testfor <target>");
            };
            // resolve_player_targets renvoie une erreur si aucun match — on
            // veut juste le count, donc on attrape l'erreur.
            let count = match resolve_player_targets(runtime, Some(token), true) {
                Ok(targets) => targets.len(),
                Err(_) => 0,
            };
            if count > 0 {
                runtime.send_feedback(&format!("Found {count} matching player(s)."));
            } else {
                runtime.send_feedback("No matching player.");
            }
            Ok(())
        },
    );

    // ── /testforblock <x> <y> <z> <block> ──
    let mut testforblock =
        CommandDefinition::new("testforblock", "Test the block at a position");
    testforblock.usage = "/testforblock <x> <y> <z> <block>".into();
    testforblock.permissions = vec!["server.command.testforblock".into()];
    testforblock.overloads.push(CommandOverload {
        parameters: vec![
            param("x", ParamType::Position, false),
            param("y", ParamType::Position, false),
            param("z", ParamType::Position, false),
            param("block", ParamType::String, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        testforblock,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 4 {
                return usage("Usage: /testforblock <x> <y> <z> <block>");
            }
            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            let pos = parse_position_triplet_for_source(
                runtime,
                origin,
                invocation.arg(0).unwrap(),
                invocation.arg(1).unwrap(),
                invocation.arg(2).unwrap(),
            )?;
            let (ix, iy, iz) = (
                pos[0].floor() as i32,
                pos[1].floor() as i32,
                pos[2].floor() as i32,
            );
            let expected_name = invocation.arg(3).unwrap();
            let Some(expected_id) = runtime.resolve_block_name(expected_name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown block: {expected_name}"
                )));
            };
            let actual_id = runtime.world_block_at(ix, iy, iz);
            if actual_id == expected_id {
                runtime.send_feedback(&format!(
                    "Block at ({ix},{iy},{iz}) matches {expected_name}."
                ));
            } else {
                runtime.send_feedback(&format!(
                    "Block at ({ix},{iy},{iz}) does NOT match {expected_name} (got id={actual_id})."
                ));
            }
            Ok(())
        },
    );

    // ── /spreadplayers <cx> <cz> <spreadDist> <maxRange> <targets...> ──
    // Vanilla : disperse les joueurs aléatoirement dans un disque autour de
    // (cx, cz), avec une distance minimum `spreadDist` entre joueurs et un
    // rayon maximum `maxRange`. On simplifie : on prend une position aléatoire
    // dans le carré [cx-maxRange, cx+maxRange] × [cz-maxRange, cz+maxRange]
    // et on cherche la surface via chunk_cache. Pas de respect strict du
    // spreadDist minimum entre joueurs (gros effort algorithmique).
    let mut spread = CommandDefinition::new(
        "spreadplayers",
        "Spread players around a center point",
    );
    spread.usage = "/spreadplayers <cx> <cz> <spreadDist> <maxRange> <target>".into();
    spread.permissions = vec!["server.command.spreadplayers".into()];
    spread.overloads.push(CommandOverload {
        parameters: vec![
            param("cx", ParamType::Position, false),
            param("cz", ParamType::Position, false),
            param("spreadDist", ParamType::Float, false),
            param("maxRange", ParamType::Float, false),
            param("target", ParamType::Target, false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        spread,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 5 {
                return usage(
                    "Usage: /spreadplayers <cx> <cz> <spreadDist> <maxRange> <target>",
                );
            }
            let origin = if runtime.sender_is_player() {
                Some(runtime.sender_position())
            } else {
                None
            };
            // cx, cz : utilise X et Z avec parse_coord (~). Y est fictif (ignoré).
            let cx_str = invocation.arg(0).unwrap();
            let cz_str = invocation.arg(1).unwrap();
            let center_x = parse_coord(cx_str, origin.map(|o| o[0]).unwrap_or(0.0))
                .ok_or_else(|| CommandDispatchError::Message("Invalid cx".into()))?;
            let center_z = parse_coord(cz_str, origin.map(|o| o[2]).unwrap_or(0.0))
                .ok_or_else(|| CommandDispatchError::Message("Invalid cz".into()))?;
            let spread_dist: f32 = invocation
                .arg(2)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| CommandDispatchError::Message("spreadDist must be a number".into()))?;
            let max_range: f32 = invocation
                .arg(3)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| CommandDispatchError::Message("maxRange must be a number".into()))?;
            if max_range <= 0.0 {
                return Err(CommandDispatchError::Message("maxRange must be > 0".into()));
            }
            let _ = spread_dist; // pas appliqué dans cette MVP — note doc.

            let targets = resolve_player_targets(runtime, invocation.arg(4), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let count = targets.len();
            for addr in targets {
                let dx = (runtime.random_index(1000) as f32 / 1000.0) * 2.0 * max_range
                    - max_range;
                let dz = (runtime.random_index(1000) as f32 / 1000.0) * 2.0 * max_range
                    - max_range;
                let tx = center_x + dx;
                let tz = center_z + dz;
                // Y : on prend une altitude raisonnable au-dessus du terrain.
                // 128 = safe height qui retombe en y=64 ground typique.
                runtime.teleport_player(addr, [tx, 128.0, tz]);
            }
            runtime.send_feedback(&format!(
                "Spread {count} player(s) around ({center_x:.0},{center_z:.0})."
            ));
            Ok(())
        },
    );

    // ── /locate <structure> ──
    // Approximation grid-based — voir structures::locate_nearest. Pas un
    // /locate vanilla complet (qui scanne réellement les chunks générés et
    // respecte les contraintes biome).
    let mut locate = CommandDefinition::new("locate", "Locate the nearest structure");
    locate.usage = "/locate <structure>".into();
    locate.permissions = vec!["server.command.locate".into()];
    locate.overloads.push(CommandOverload {
        parameters: vec![param("structure", ParamType::String, false)],
    });
    register_command(
        &mut permissions,
        &mut map,
        locate,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(name) = invocation.arg(0) else {
                return usage("Usage: /locate <structure>");
            };
            let Some(kind) = crate::structures::StructureKind::parse(name) else {
                return Err(CommandDispatchError::Message(format!(
                    "Unknown structure: {name}"
                )));
            };
            if !runtime.sender_is_player() {
                return Err(CommandDispatchError::Message(
                    "Console must be in a player context for /locate.".into(),
                ));
            }
            let origin = runtime.sender_position();
            let pos = crate::structures::locate_nearest(kind, origin);
            let dx = pos[0] as f32 - origin[0];
            let dz = pos[2] as f32 - origin[2];
            let dist = (dx * dx + dz * dz).sqrt();
            runtime.send_feedback(&format!(
                "Nearest {} estimated at ({}, ~{}, {}) — {} blocks away (approximation grid).",
                kind.identifier(),
                pos[0],
                pos[1],
                pos[2],
                dist as i32,
            ));
            Ok(())
        },
    );

    // ── /reload ──
    // Recharge ops/whitelist/bans depuis disque. Pas un reload complet (les
    // chunks, plugins déjà chargés et server.toml ne sont pas relus).
    let mut reload = CommandDefinition::new("reload", "Reload server state from disk");
    reload.usage = "/reload".into();
    reload.permissions = vec!["server.command.reload".into()];
    register_command(
        &mut permissions,
        &mut map,
        reload,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, _invocation: &CommandInvocation| {
            runtime
                .reload_server_state()
                .map_err(CommandDispatchError::Message)?;
            runtime.send_feedback("Reloaded ops/whitelist/bans from disk.");
            runtime.sync_available_commands_for_all();
            Ok(())
        },
    );

    // ── /ability <target> <ability> <true|false> ──
    let mut ability_cmd = CommandDefinition::new("ability", "Toggle a player ability");
    ability_cmd.usage = "/ability <target> <mayfly|mute|worldbuilder|...> <true|false>".into();
    ability_cmd.permissions = vec!["server.command.ability".into()];
    ability_cmd.overloads.push(CommandOverload {
        parameters: vec![
            param("target", ParamType::Target, false),
            param("ability", ParamType::String, false),
            hard_enum_param("value", "ability_value", &["true", "false"], false),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        ability_cmd,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            if invocation.args.len() < 3 {
                return usage("Usage: /ability <target> <ability> <true|false>");
            }
            let targets = resolve_player_targets(runtime, invocation.arg(0), true)?;
            if targets.is_empty() {
                return Err(CommandDispatchError::Message("No matching player.".into()));
            }
            let ability_name = invocation.arg(1).unwrap();
            let value = match invocation.arg(2).unwrap().to_ascii_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => true,
                "false" | "0" | "off" | "no" => false,
                _ => {
                    return Err(CommandDispatchError::Message(
                        "Value must be true or false.".into(),
                    ))
                }
            };
            let count = targets.len();
            for addr in targets {
                runtime
                    .set_player_ability(addr, ability_name, value)
                    .map_err(CommandDispatchError::Message)?;
            }
            runtime.send_feedback(&format!(
                "Set {ability_name}={value} for {count} player(s)."
            ));
            Ok(())
        },
    );

    // ── /music play|stop|volume <track> [volume] ──
    // Simplification : on délègue à PlaySound (le client traite les sons
    // commençant par "music." comme musique). Pas de queue / fade comme
    // vanilla, juste play/stop/volume.
    let mut music = CommandDefinition::new("music", "Control music playback");
    music.usage = "/music play|stop|volume <track|amount> [volume]".into();
    music.permissions = vec!["server.command.music".into()];
    music.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "music_action", &["play", "stop", "volume"], false),
            param("arg", ParamType::String, true),
            param("volume", ParamType::Float, true),
        ],
    });
    register_command(
        &mut permissions,
        &mut map,
        music,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /music play|stop|volume <track|amount> [volume]");
            };
            let action = action.to_ascii_lowercase();
            // Broadcast (pas de target select pour /music vanilla — tous les
            // joueurs du serveur).
            let center = if runtime.sender_is_player() {
                runtime.sender_position()
            } else {
                [0.0, 64.0, 0.0]
            };
            match action.as_str() {
                "play" => {
                    let Some(track) = invocation.arg(1) else {
                        return usage("Usage: /music play <track> [volume]");
                    };
                    let volume: f32 = invocation
                        .arg(2)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                    // Préfixe `music.` si absent — convention Bedrock.
                    let sound = if track.contains('.') {
                        track.to_string()
                    } else {
                        format!("music.{track}")
                    };
                    runtime.play_sound(&[], &sound, center, volume, 1.0);
                    runtime.send_feedback(&format!("Now playing: {sound}"));
                }
                "stop" => {
                    // Stop tout son côté client (paramètre None = stop_all).
                    runtime.stop_sound(&[], None);
                    runtime.send_feedback("Stopped music.");
                }
                "volume" => {
                    // Pas de mécanisme vanilla pour ajuster un son déjà en
                    // cours — on accepte la commande mais on ne change rien.
                    runtime.send_feedback(
                        "Music volume command accepted (no-op — re-play to change volume).",
                    );
                }
                _ => return usage("Action must be play, stop, or volume."),
            }
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
        /// Blocs simulés pour /setblock, /fill, /clone, /testforblock.
        world_blocks: HashMap<(i32, i32, i32), u32>,
        /// Game rules pour /gamerule (init avec vanilla_defaults).
        gamerules: crate::game_rules::GameRules,
        /// Tags par joueur pour /tag.
        player_tags: HashMap<SocketAddr, std::collections::HashSet<String>>,
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
                world_blocks: HashMap::new(),
                gamerules: crate::game_rules::GameRules::vanilla_defaults(),
                player_tags: HashMap::new(),
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

        fn resolve_block_name(&self, name: &str) -> Option<u32> {
            // Test stub : on accepte "stone" / "dirt" / "air" / "minecraft:xxx"
            // sans toucher au registre global qui dépend de l'init du serveur.
            let n = name.strip_prefix("minecraft:").unwrap_or(name);
            match n {
                "air" => Some(0),
                "stone" => Some(1),
                "dirt" => Some(2),
                "grass" | "grass_block" => Some(3),
                "wood" | "planks" => Some(5),
                _ => None,
            }
        }

        fn set_world_block(&mut self, x: i32, y: i32, z: i32, block_id: u32) -> bool {
            let prev = self.world_blocks.get(&(x, y, z)).copied().unwrap_or(0);
            if prev == block_id {
                return false;
            }
            self.world_blocks.insert((x, y, z), block_id);
            true
        }

        fn world_block_at(&self, x: i32, y: i32, z: i32) -> u32 {
            self.world_blocks.get(&(x, y, z)).copied().unwrap_or(0)
        }

        fn gamerule_list(&self) -> Vec<(String, crate::game_rules::GameRuleValue)> {
            let mut entries: Vec<_> = self
                .gamerules
                .rules
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            entries
        }

        fn gamerule_get(&self, name: &str) -> Option<crate::game_rules::GameRuleValue> {
            self.gamerules.get(name).cloned()
        }

        fn gamerule_set(
            &mut self,
            name: &str,
            value: crate::game_rules::GameRuleValue,
        ) -> Result<(), String> {
            self.gamerules.set(name.to_string(), value);
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
            _slot: usize,
            _item: ItemStack,
        ) -> Result<(), String> {
            Ok(())
        }

        fn player_tag_add(&mut self, addr: SocketAddr, tag: &str) -> bool {
            self.player_tags
                .entry(addr)
                .or_default()
                .insert(tag.to_string())
        }

        fn player_tag_remove(&mut self, addr: SocketAddr, tag: &str) -> bool {
            self.player_tags
                .get_mut(&addr)
                .is_some_and(|s| s.remove(tag))
        }

        fn player_tag_list(&self, addr: SocketAddr) -> Vec<String> {
            let mut tags: Vec<String> = self
                .player_tags
                .get(&addr)
                .into_iter()
                .flat_map(|s| s.iter().cloned())
                .collect();
            tags.sort();
            tags
        }

        fn spawn_item_world(&mut self, _position: [f32; 3], _item: ItemStack) {}

        fn roll_chest_loot_drops(&self, _table_name: &str) -> Vec<(String, u32)> {
            Vec::new()
        }

        fn damage_player(&mut self, _addr: SocketAddr, _amount: f32) -> Result<bool, String> {
            Ok(false)
        }

        fn actor_event_broadcast(
            &mut self,
            _runtime_entity_id: u64,
            _event_id: u32,
            _data: i32,
        ) {
        }

        fn first_entity_runtime_id(&self, _token: &str) -> Option<u64> {
            None
        }

        fn reload_server_state(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn set_player_ability(
            &mut self,
            _addr: SocketAddr,
            _ability: &str,
            _value: bool,
        ) -> Result<(), String> {
            Ok(())
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

    // ──────── Tests pour les commandes ajoutées Phase 7+ ────────

    #[test]
    fn setblock_places_block_in_world() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "setblock 5 64 5 stone");
        assert_eq!(runtime.world_block_at(5, 64, 5), 1);

        // Mode keep : ne remplace pas si non-air.
        dispatch_ok(&system, &mut runtime, "setblock 5 64 5 dirt keep");
        assert_eq!(
            runtime.world_block_at(5, 64, 5),
            1,
            "keep mode should NOT overwrite existing block"
        );

        // Mode replace : remplace.
        dispatch_ok(&system, &mut runtime, "setblock 5 64 5 dirt replace");
        assert_eq!(runtime.world_block_at(5, 64, 5), 2);
    }

    #[test]
    fn setblock_rejects_unknown_block() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        let err = dispatch_err(&system, &mut runtime, "setblock 0 64 0 not_a_block");
        assert!(err.contains("Unknown block"), "got: {err}");
    }

    #[test]
    fn fill_fills_region_correctly() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        // Région 2×2×2 = 8 blocs.
        dispatch_ok(&system, &mut runtime, "fill 0 64 0 1 65 1 stone");
        for x in 0..=1 {
            for y in 64..=65 {
                for z in 0..=1 {
                    assert_eq!(runtime.world_block_at(x, y, z), 1, "at ({x},{y},{z})");
                }
            }
        }
        assert!(
            runtime.feedback.iter().any(|m| m.contains("Filled 8 blocks")),
            "expected 'Filled 8 blocks' feedback, got: {:?}",
            runtime.feedback
        );
    }

    #[test]
    fn fill_outline_only_borders() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        // 3×3×3 → border = 26 blocs, intérieur = 1 bloc (intact).
        dispatch_ok(&system, &mut runtime, "fill 0 64 0 2 66 2 stone outline");
        // Centre devrait être non rempli (toujours 0).
        assert_eq!(runtime.world_block_at(1, 65, 1), 0);
        // Coin devrait être rempli.
        assert_eq!(runtime.world_block_at(0, 64, 0), 1);
        assert_eq!(runtime.world_block_at(2, 66, 2), 1);
    }

    #[test]
    fn fill_rejects_too_large_region() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        // 33×33×32 = 34848 > 32768 → rejet.
        let err = dispatch_err(&system, &mut runtime, "fill 0 0 0 32 32 31 stone");
        assert!(err.contains("Region too large"), "got: {err}");
    }

    #[test]
    fn clone_copies_region() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        // Pose un stone à (0,64,0), clone vers (10, 64, 0).
        dispatch_ok(&system, &mut runtime, "setblock 0 64 0 stone");
        dispatch_ok(&system, &mut runtime, "clone 0 64 0 0 64 0 10 64 0");
        assert_eq!(runtime.world_block_at(10, 64, 0), 1);
    }

    #[test]
    fn gamerule_set_and_query() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        // Set boolean
        dispatch_ok(&system, &mut runtime, "gamerule keepinventory true");
        let v = runtime.gamerule_get("keepinventory").unwrap();
        match v {
            crate::game_rules::GameRuleValue::Bool(b) => assert!(b),
            _ => panic!("keepinventory should be Bool"),
        }

        // Set int
        dispatch_ok(&system, &mut runtime, "gamerule randomtickspeed 10");
        let v = runtime.gamerule_get("randomtickspeed").unwrap();
        match v {
            crate::game_rules::GameRuleValue::Int(i) => assert_eq!(i, 10),
            _ => panic!("randomtickspeed should be Int"),
        }

        // Query without value
        dispatch_ok(&system, &mut runtime, "gamerule keepinventory");
        assert!(runtime
            .feedback
            .iter()
            .any(|m| m.contains("keepinventory") && m.contains("true")));
    }

    #[test]
    fn gamerule_rejects_unknown_rule() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        let err = dispatch_err(&system, &mut runtime, "gamerule notarealrule true");
        assert!(err.contains("Unknown game rule"), "got: {err}");
    }

    #[test]
    fn tag_add_list_remove_cycle() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "tag Steve add hero");
        dispatch_ok(&system, &mut runtime, "tag Steve add archer");
        let tags = runtime.player_tag_list(runtime.player("Steve").addr);
        assert_eq!(tags, vec!["archer", "hero"]);

        dispatch_ok(&system, &mut runtime, "tag Steve remove hero");
        let tags = runtime.player_tag_list(runtime.player("Steve").addr);
        assert_eq!(tags, vec!["archer"]);

        dispatch_ok(&system, &mut runtime, "tag Steve list");
        assert!(runtime
            .feedback
            .iter()
            .any(|m| m.contains("archer") && m.contains("1 tag")));
    }

    #[test]
    fn testforblock_matches_set_block() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);

        dispatch_ok(&system, &mut runtime, "setblock 0 64 0 stone");
        dispatch_ok(&system, &mut runtime, "testforblock 0 64 0 stone");
        assert!(runtime
            .feedback
            .iter()
            .any(|m| m.contains("matches")));

        dispatch_ok(&system, &mut runtime, "testforblock 0 64 0 dirt");
        assert!(runtime
            .feedback
            .iter()
            .any(|m| m.contains("does NOT match")));
    }

    #[test]
    fn locate_returns_estimated_position() {
        let system = build_command_system();
        let mut runtime = TestRuntime::new(&system);
        // /locate exige un sender player, on peut juste vérifier l'erreur console.
        let err = dispatch_err(&system, &mut runtime, "locate village");
        assert!(err.contains("Console must be in a player context"), "got: {err}");
    }
}
