use crate::io::{ProtoReader, ProtoWriter};

// ── PlayerAuthInput (C→S, 0x90) ──

/// Block action from PlayerAuthInput.
pub struct BlockAction {
    pub action_type: i32,
    pub position: [i32; 3],
    pub face: i32,
}

/// Item interaction data from PlayerAuthInput (UseItemTransactionData).
pub struct ItemInteractionData {
    pub action_type: u32, // 0=CLICK_BLOCK, 1=CLICK_AIR, 2=BREAK_BLOCK
    pub block_position: [i32; 3],
    pub face: i32,
    pub hotbar_slot: i32,
    pub player_position: [f32; 3],
    pub click_position: [f32; 3],
    pub block_runtime_id: u32,
    pub client_prediction: u32,
}

/// Slot info from ItemStackRequest actions.
pub struct SlotInfo {
    pub container_id: u8,
    pub slot_id: u8,
    pub stack_id: i32,
}

/// A single action in an ItemStackRequest.
pub enum StackRequestAction {
    /// Take/Place: move `count` items from source to destination.
    Take {
        count: u8,
        source: SlotInfo,
        destination: SlotInfo,
    },
    Place {
        count: u8,
        source: SlotInfo,
        destination: SlotInfo,
    },
    Swap {
        count: u8,
        source: SlotInfo,
        destination: SlotInfo,
    },
    Drop {
        count: u8,
        source: SlotInfo,
    },
    Destroy {
        count: u8,
        source: SlotInfo,
    },
    /// Other actions we don't handle yet.
    Unknown(u8),
}

/// An ItemStackRequest decoded from PlayerAuthInput.
pub struct ItemStackRequest {
    pub request_id: i32,
    pub actions: Vec<StackRequestAction>,
}

/// Player movement and input.
pub struct PlayerAuthInput {
    pub pitch: f32,
    pub yaw: f32,
    pub position: [f32; 3],
    pub move_vec_x: f32,
    pub move_vec_z: f32,
    pub head_yaw: f32,
    pub block_actions: Vec<BlockAction>,
    pub item_interaction: Option<ItemInteractionData>,
    pub item_stack_request: Option<ItemStackRequest>,
}

// PlayerAuthInput flag bits (from PMMP PlayerAuthInputFlags.php)
const FLAG_PERFORM_ITEM_INTERACTION: usize = 34;
const FLAG_PERFORM_BLOCK_ACTIONS: usize = 35;
const FLAG_PERFORM_ITEM_STACK_REQUEST: usize = 36;

/// Read a BitSet encoded as VarInt-style bytes (7 bits per byte, bit 7 = continuation).
fn read_bitset(reader: &mut ProtoReader) -> Result<u128, crate::io::reader::ProtoReadError> {
    let mut result: u128 = 0;
    let mut shift = 0;
    loop {
        let b = reader.read_u8()?;
        result |= ((b & 0x7F) as u128) << shift;
        if (b & 0x80) == 0 {
            break;
        }
        shift += 7;
        if shift >= 128 {
            break;
        }
    }
    Ok(result)
}

/// Skip an ItemStackWrapper in the reader.
fn skip_item_stack_wrapper(
    reader: &mut ProtoReader,
) -> Result<(), crate::io::reader::ProtoReadError> {
    let id = reader.read_var_i32()?;
    if id == 0 {
        return Ok(());
    }
    let _count = reader.read_u16_le()?;
    let _meta = reader.read_var_u32()?;
    let has_net_id = reader.read_bool()?;
    if has_net_id {
        let _stack_id = reader.read_var_i32()?;
    }
    let _block_runtime_id = reader.read_var_i32()?;
    let _extra_data = reader.read_string()?;
    Ok(())
}

/// Skip a NetworkInventoryAction in the reader.
fn skip_network_inventory_action(
    reader: &mut ProtoReader,
) -> Result<(), crate::io::reader::ProtoReadError> {
    let source_type = reader.read_var_u32()?;
    match source_type {
        0 => {
            let _window_id = reader.read_var_i32()?;
        } // SOURCE_CONTAINER
        1 => {
            let _flags = reader.read_var_u32()?;
        } // SOURCE_WORLD
        2 => {} // SOURCE_CREATIVE
        _ => {}
    }
    let _slot = reader.read_var_u32()?;
    skip_item_stack_wrapper(reader)?; // old item
    skip_item_stack_wrapper(reader)?; // new item
    Ok(())
}

/// Decode the ItemInteractionData from PlayerAuthInput.
fn decode_item_interaction(
    reader: &mut ProtoReader,
) -> Result<ItemInteractionData, crate::io::reader::ProtoReadError> {
    // Legacy request ID
    let request_id = reader.read_var_i32()?;
    if request_id != 0 {
        let changed_slots_count = reader.read_var_u32()?;
        for _ in 0..changed_slots_count {
            let _container_id = reader.read_u8()?;
            let slot_count = reader.read_var_u32()?;
            for _ in 0..slot_count {
                let _slot = reader.read_u8()?;
            }
        }
    }

    // UseItemTransactionData
    let action_count = reader.read_var_u32()?;
    for _ in 0..action_count.min(100) {
        skip_network_inventory_action(reader)?;
    }

    let action_type = reader.read_var_u32()?;
    let trigger_type = reader.read_var_u32()?;
    let _ = trigger_type;

    let bx = reader.read_var_i32()?;
    let by = reader.read_var_u32()? as i32;
    let bz = reader.read_var_i32()?;

    let face = reader.read_var_i32()?;
    let hotbar_slot = reader.read_var_i32()?;

    skip_item_stack_wrapper(reader)?; // item in hand

    let px = reader.read_f32_le()?;
    let py = reader.read_f32_le()?;
    let pz = reader.read_f32_le()?;

    let cx = reader.read_f32_le()?;
    let cy = reader.read_f32_le()?;
    let cz = reader.read_f32_le()?;

    let block_runtime_id = reader.read_var_u32()?;
    let client_prediction = reader.read_var_u32()?;

    Ok(ItemInteractionData {
        action_type,
        block_position: [bx, by, bz],
        face,
        hotbar_slot,
        player_position: [px, py, pz],
        click_position: [cx, cy, cz],
        block_runtime_id,
        client_prediction,
    })
}

/// Read a SlotInfo from the reader.
fn read_slot_info(reader: &mut ProtoReader) -> Result<SlotInfo, crate::io::reader::ProtoReadError> {
    let container_id = reader.read_u8()?;
    let has_dynamic = reader.read_bool()?;
    if has_dynamic {
        let _dynamic_id = reader.read_u32_le()?;
    }
    let slot_id = reader.read_u8()?;
    let stack_id = reader.read_var_i32()?;
    Ok(SlotInfo {
        container_id,
        slot_id,
        stack_id,
    })
}

/// Decode an ItemStackRequest from the reader.
fn decode_item_stack_request(
    reader: &mut ProtoReader,
) -> Result<ItemStackRequest, crate::io::reader::ProtoReadError> {
    let request_id = reader.read_var_i32()?;
    let action_count = reader.read_var_u32()?;
    let mut actions = Vec::new();

    for _ in 0..action_count.min(60) {
        let action_type = reader.read_u8()?;
        let action = match action_type {
            0 => {
                let count = reader.read_u8()?;
                let source = read_slot_info(reader)?;
                let destination = read_slot_info(reader)?;
                StackRequestAction::Take {
                    count,
                    source,
                    destination,
                }
            }
            1 => {
                let count = reader.read_u8()?;
                let source = read_slot_info(reader)?;
                let destination = read_slot_info(reader)?;
                StackRequestAction::Place {
                    count,
                    source,
                    destination,
                }
            }
            2 => {
                let count = reader.read_u8()?;
                let source = read_slot_info(reader)?;
                let destination = read_slot_info(reader)?;
                StackRequestAction::Swap {
                    count,
                    source,
                    destination,
                }
            }
            3 => {
                let count = reader.read_u8()?;
                let source = read_slot_info(reader)?;
                StackRequestAction::Drop { count, source }
            }
            4 => {
                let count = reader.read_u8()?;
                let source = read_slot_info(reader)?;
                StackRequestAction::Destroy { count, source }
            }
            5 => {
                // CraftingConsumeInput — skip
                let _count = reader.read_u8()?;
                read_slot_info(reader)?;
                StackRequestAction::Unknown(5)
            }
            6 => {
                let _slot = reader.read_u8()?;
                StackRequestAction::Unknown(6)
            }
            11 => {
                let _hotbar = reader.read_var_i32()?;
                let _durability = reader.read_var_i32()?;
                let _stack_id = reader.read_var_i32()?;
                StackRequestAction::Unknown(11)
            }
            12 => {
                let _recipe_id = reader.read_var_u32()?;
                let _times = reader.read_u8()?;
                StackRequestAction::Unknown(12)
            }
            13 => {
                let _recipe_id = reader.read_var_u32()?;
                let _times = reader.read_u8()?;
                let ingredient_count = reader.read_u8()?;
                for _ in 0..ingredient_count {
                    let _item = reader.read_u8()?;
                }
                StackRequestAction::Unknown(13)
            }
            14 => {
                let _slot = reader.read_var_u32()?;
                StackRequestAction::Unknown(14)
            }
            other => StackRequestAction::Unknown(other),
        };
        actions.push(action);
        if matches!(action_type, 6..=10 | 15..) {
            // Unknown actions may have unparseable data, stop
            if action_type > 14 {
                break;
            }
        }
    }

    // Filter strings
    let filter_count = reader.read_var_u32()?;
    for _ in 0..filter_count {
        let _s = reader.read_string()?;
    }
    let _filter_cause = reader.read_i32_le()?;

    Ok(ItemStackRequest {
        request_id,
        actions,
    })
}

impl PlayerAuthInput {
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let pitch = reader.read_f32_le()?;
        let yaw = reader.read_f32_le()?;
        let pos_x = reader.read_f32_le()?;
        let pos_y = reader.read_f32_le()?;
        let pos_z = reader.read_f32_le()?;
        let move_vec_x = reader.read_f32_le()?;
        let move_vec_z = reader.read_f32_le()?;
        let head_yaw = reader.read_f32_le()?;

        let input_flags = read_bitset(reader)?;

        let _input_mode = reader.read_var_u32()?;
        let _play_mode = reader.read_var_u32()?;
        let _interaction_mode = reader.read_var_u32()?;
        let _interact_rot_x = reader.read_f32_le()?;
        let _interact_rot_y = reader.read_f32_le()?;
        let _tick = reader.read_var_u64()?;
        let _delta_x = reader.read_f32_le()?;
        let _delta_y = reader.read_f32_le()?;
        let _delta_z = reader.read_f32_le()?;

        // Decode order must match PMMP: item_interaction, item_stack_request, block_actions

        // Item interaction (bit 34) — decode fully
        let item_interaction = if (input_flags >> FLAG_PERFORM_ITEM_INTERACTION) & 1 == 1 {
            match decode_item_interaction(reader) {
                Ok(data) => Some(data),
                Err(_) => {
                    return Ok(Self {
                        pitch,
                        yaw,
                        position: [pos_x, pos_y, pos_z],
                        move_vec_x,
                        move_vec_z,
                        head_yaw,
                        block_actions: Vec::new(),
                        item_interaction: None,
                        item_stack_request: None,
                    });
                }
            }
        } else {
            None
        };

        // Item stack request (bit 36) — decode
        let item_stack_request = if (input_flags >> FLAG_PERFORM_ITEM_STACK_REQUEST) & 1 == 1 {
            match decode_item_stack_request(reader) {
                Ok(req) => Some(req),
                Err(_) => {
                    return Ok(Self {
                        pitch,
                        yaw,
                        position: [pos_x, pos_y, pos_z],
                        move_vec_x,
                        move_vec_z,
                        head_yaw,
                        block_actions: Vec::new(),
                        item_interaction,
                        item_stack_request: None,
                    });
                }
            }
        } else {
            None
        };

        // Block actions (bit 35)
        let mut block_actions = Vec::new();
        if (input_flags >> FLAG_PERFORM_BLOCK_ACTIONS) & 1 == 1 {
            let count = reader.read_var_i32().unwrap_or(0);
            for _ in 0..count.min(100) {
                let Ok(action_type) = reader.read_var_i32() else {
                    break;
                };
                if action_type == 2 {
                    // STOP_BREAK — no position data
                    block_actions.push(BlockAction {
                        action_type,
                        position: [0, 0, 0],
                        face: 0,
                    });
                } else {
                    let bx = reader.read_var_i32().unwrap_or(0);
                    let by = reader.read_var_i32().unwrap_or(0);
                    let bz = reader.read_var_i32().unwrap_or(0);
                    let face = reader.read_var_i32().unwrap_or(0);
                    block_actions.push(BlockAction {
                        action_type,
                        position: [bx, by, bz],
                        face,
                    });
                }
            }
        }

        Ok(Self {
            pitch,
            yaw,
            position: [pos_x, pos_y, pos_z],
            move_vec_x,
            move_vec_z,
            head_yaw,
            block_actions,
            item_interaction,
            item_stack_request,
        })
    }
}

// ── MovePlayer (S→C, 0x13) ──

pub struct MovePlayer {
    pub runtime_entity_id: u64,
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub mode: u8, // 0=normal, 1=reset, 2=teleport, 3=rotation
    pub on_ground: bool,
    pub riding_runtime_id: u64,
    pub tick: u64,
}

impl MovePlayer {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_var_u64(self.runtime_entity_id);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_f32_le(self.pitch);
        w.write_f32_le(self.yaw);
        w.write_f32_le(self.head_yaw);
        w.write_u8(self.mode);
        w.write_bool(self.on_ground);
        w.write_var_u64(self.riding_runtime_id);
        if self.mode == 2 {
            // Teleport: cause + source entity type
            w.write_i32_le(0); // cause = unknown
            w.write_i32_le(0); // source entity type
        }
        w.write_var_u64(self.tick);
        w.into_bytes()
    }
}

// ── Text (Bi-directional, 0x09) ──
// Protocol 924 format (from PMMP TextPacket.php):
//   needsTranslation(bool) + category(u8) + type(u8) + [conditional] + xuid + platformChatId + filteredMessage(optional)
// Categories: 0=MESSAGE_ONLY, 1=AUTHORED_MESSAGE, 2=MESSAGE_WITH_PARAMS

pub struct Text {
    pub text_type: u8,
    pub needs_translation: bool,
    pub source_name: String,
    pub message: String,
    pub xuid: String,
    pub platform_chat_id: String,
}

impl Text {
    /// Decode a Text packet from the client.
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let needs_translation = reader.read_bool()?;
        let _category = reader.read_u8()?; // category byte (protocol 924)
        let text_type = reader.read_u8()?;

        let (source_name, message) = match text_type {
            1 | 7 | 8 => {
                // CHAT, WHISPER, ANNOUNCEMENT — category=1 (authored) → source + message
                let source = reader.read_string()?;
                let msg = reader.read_string()?;
                (source, msg)
            }
            0 | 5 | 6 | 10 => {
                // RAW, TIP, SYSTEM, JSON — category=0 (message_only) → message only
                let msg = reader.read_string()?;
                (String::new(), msg)
            }
            2..=4 => {
                // TRANSLATION, POPUP, JUKEBOX — category=2 (with_params) → message + params
                let msg = reader.read_string()?;
                let count = reader.read_var_u32()?;
                for _ in 0..count {
                    let _ = reader.read_string()?;
                }
                (String::new(), msg)
            }
            _ => {
                let msg = reader.read_string()?;
                (String::new(), msg)
            }
        };

        let xuid = reader.read_string().unwrap_or_default();
        let platform_chat_id = reader.read_string().unwrap_or_default();
        // filteredMessage (optional) — skip
        let _ = reader.read_bool(); // has_filtered
                                    // if has_filtered, read string — but we skip

        Ok(Self {
            text_type,
            needs_translation,
            source_name,
            message,
            xuid,
            platform_chat_id,
        })
    }

    /// Encode a chat message to broadcast (category=1 AUTHORED_MESSAGE).
    pub fn chat(source: &str, message: &str, xuid: &str) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_bool(false); // needsTranslation
        w.write_u8(1); // category = AUTHORED_MESSAGE
        w.write_u8(1); // type = CHAT
        w.write_string(source);
        w.write_string(message);
        w.write_string(xuid); // xboxUserId
        w.write_string(""); // platformChatId
        w.write_bool(false); // filteredMessage = None
        w.into_bytes()
    }

    /// Encode a system/raw message (category=0 MESSAGE_ONLY).
    pub fn system(message: &str) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_bool(false); // needsTranslation
        w.write_u8(0); // category = MESSAGE_ONLY
        w.write_u8(0); // type = RAW
        w.write_string(message);
        w.write_string(""); // xboxUserId
        w.write_string(""); // platformChatId
        w.write_bool(false); // filteredMessage = None
        w.into_bytes()
    }
}

// ── PlayerList (S→C, 0x3F) ──

pub struct PlayerListAdd {
    pub uuid: [u8; 16],
    pub entity_id: i64, // varint64 (actor unique ID)
    pub username: String,
    pub xuid: String,
    pub platform_chat_id: String,
    pub build_platform: i32, // i32_le
    pub is_teacher: bool,
    pub is_host: bool,
    pub is_subclient: bool,
}

pub struct PlayerList {
    pub action: u8, // 0 = add, 1 = remove
    pub entries: Vec<PlayerListAdd>,
}

impl PlayerList {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(256);
        w.write_u8(self.action);
        w.write_var_u32(self.entries.len() as u32);

        if self.action == 0 {
            // Add entries
            for entry in &self.entries {
                w.write_raw(&entry.uuid);
                w.write_var_i64(entry.entity_id);
                w.write_string(&entry.username);
                w.write_string(&entry.xuid);
                w.write_string(&entry.platform_chat_id);
                w.write_i32_le(entry.build_platform);
                // Skin data — minimal empty skin
                write_empty_skin(&mut w);
                w.write_bool(entry.is_teacher);
                w.write_bool(entry.is_host);
                w.write_bool(entry.is_subclient);
            }
            // Verified skins (one per entry)
            for _ in &self.entries {
                w.write_bool(true);
            }
        } else {
            // Remove entries — just UUIDs
            for entry in &self.entries {
                w.write_raw(&entry.uuid);
            }
        }

        w.into_bytes()
    }
}

fn write_empty_skin(w: &mut ProtoWriter) {
    w.write_string("Custom"); // skin_id
    w.write_string(""); // play_fab_id
    w.write_string("geometry.humanoid.custom"); // skin_resource_patch
                                                // Skin image data
    w.write_u32_le(64); // width
    w.write_u32_le(64); // height
    let skin_data = vec![0u8; 64 * 64 * 4]; // RGBA
    w.write_byte_array(&skin_data);

    // Animations — empty
    w.write_u32_le(0);

    // Cape image — empty
    w.write_u32_le(0); // width
    w.write_u32_le(0); // height
    w.write_byte_array(&[]); // data

    w.write_string(""); // geometry_data
    w.write_string(""); // geometry_data_engine_version
    w.write_string(""); // animation_data

    w.write_string(""); // cape_id
    w.write_string(""); // full_skin_id
    w.write_string(""); // arm_size
    w.write_string(""); // skin_color

    // Persona pieces — empty
    w.write_u32_le(0);
    // Persona tint colors — empty
    w.write_u32_le(0);

    w.write_bool(false); // premium_skin
    w.write_bool(false); // persona_skin
    w.write_bool(false); // persona_cape_on_classic
    w.write_bool(false); // primary_user

    w.write_bool(false); // override_appearance (new)
}

// ── AddPlayer (S→C, 0x0C) ──

pub struct AddPlayer {
    pub uuid: [u8; 16],
    pub username: String,
    pub runtime_entity_id: u64,
    pub platform_chat_id: String,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub gamemode: i32,
    pub entity_unique_id: i64,
    pub permission_level: u8,
    pub command_permission: u8,
}

impl AddPlayer {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(512);
        w.write_raw(&self.uuid);
        w.write_string(&self.username);
        w.write_var_u64(self.runtime_entity_id);
        w.write_string(&self.platform_chat_id);
        // Position
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        // Velocity
        w.write_f32_le(self.velocity[0]);
        w.write_f32_le(self.velocity[1]);
        w.write_f32_le(self.velocity[2]);
        // Rotation
        w.write_f32_le(self.pitch);
        w.write_f32_le(self.yaw);
        w.write_f32_le(self.head_yaw);
        // Held item (air = empty)
        w.write_var_i32(0); // runtime_id = 0 = air/empty
                            // Gamemode
        w.write_var_i32(self.gamemode);
        // Entity metadata (5 entries)
        w.write_var_u32(5);
        // FLAGS (key=0, type=7=VarLong, value=0)
        w.write_var_u32(0);
        w.write_var_u32(7);
        w.write_var_i64(0);
        // NAMETAG (key=4, type=4=String)
        w.write_var_u32(4);
        w.write_var_u32(4);
        w.write_string(&self.username);
        // SCALE (key=23/0x17, type=3=Float, value=1.0)
        w.write_var_u32(0x17);
        w.write_var_u32(3);
        w.write_f32_le(1.0);
        // BOUNDING_BOX_WIDTH (key=38/0x26, type=3=Float, value=0.6)
        w.write_var_u32(0x26);
        w.write_var_u32(3);
        w.write_f32_le(0.6);
        // BOUNDING_BOX_HEIGHT (key=39/0x27, type=3=Float, value=1.8)
        w.write_var_u32(0x27);
        w.write_var_u32(3);
        w.write_f32_le(1.8);
        // AbilityData
        w.write_u8(self.command_permission);
        w.write_u8(self.permission_level);
        w.write_i64_le(self.entity_unique_id);
        // 1 ability layer (Base)
        w.write_var_u32(1);
        w.write_u16_le(1); // layer type = Base
        w.write_u32_le(0x1BFFF); // abilities set
        w.write_u32_le(0x18063); // abilities values (creative defaults)
        w.write_f32_le(0.05); // fly speed
        w.write_f32_le(0.1); // walk speed
                             // Entity links (none)
        w.write_var_u32(0);
        // Device ID + OS
        w.write_string(""); // device_id
        w.write_i32_le(0); // device_os
        w.into_bytes()
    }
}

// ── RemoveEntity (S→C, 0x0E) ──

pub struct RemoveEntity {
    pub entity_unique_id: i64,
}

impl RemoveEntity {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(8);
        w.write_var_i64(self.entity_unique_id);
        w.into_bytes()
    }
}

// ── UpdateAbilities (S→C, 0x12B) ──

pub struct UpdateAbilities {
    pub entity_id: i64,
    pub permission_level: u8,
    pub command_permission: u8,
    pub layers: Vec<AbilitiesLayer>,
}

pub struct AbilitiesLayer {
    pub layer_type: u16,
    pub abilities_set: u32,
    pub abilities_values: u32,
    pub fly_speed: f32,
    pub vertical_fly_speed: f32,
    pub walk_speed: f32,
}

// Ability flag bits (from PMMP AbilitiesLayer.php)
#[allow(dead_code)]
pub mod ability {
    pub const BUILD: u32 = 1 << 0;
    pub const MINE: u32 = 1 << 1;
    pub const DOORS_AND_SWITCHES: u32 = 1 << 2;
    pub const OPEN_CONTAINERS: u32 = 1 << 3;
    pub const ATTACK_PLAYERS: u32 = 1 << 4;
    pub const ATTACK_MOBS: u32 = 1 << 5;
    pub const OPERATOR: u32 = 1 << 6;
    pub const TELEPORT: u32 = 1 << 7;
    pub const INVULNERABLE: u32 = 1 << 8;
    pub const FLYING: u32 = 1 << 9;
    pub const ALLOW_FLIGHT: u32 = 1 << 10;
    pub const INFINITE_RESOURCES: u32 = 1 << 11; // note: inverted logic
    pub const LIGHTNING: u32 = 1 << 12;
    pub const FLY_SPEED: u32 = 1 << 13;
    pub const WALK_SPEED: u32 = 1 << 14;
    pub const MUTED: u32 = 1 << 15;
    pub const WORLD_BUILDER: u32 = 1 << 16;
    pub const NO_CLIP: u32 = 1 << 17;
    pub const PRIVILEGED_BUILDER: u32 = 1 << 18;
    pub const VERTICAL_FLY_SPEED: u32 = 1 << 19; // CRITICAL — must be in abilities_set!

    /// All abilities that can be set in the BASE layer (bits 0-19)
    pub const ALL: u32 = BUILD
        | MINE
        | DOORS_AND_SWITCHES
        | OPEN_CONTAINERS
        | ATTACK_PLAYERS
        | ATTACK_MOBS
        | OPERATOR
        | TELEPORT
        | INVULNERABLE
        | FLYING
        | ALLOW_FLIGHT
        | INFINITE_RESOURCES
        | LIGHTNING
        | FLY_SPEED
        | WALK_SPEED
        | MUTED
        | WORLD_BUILDER
        | NO_CLIP
        | PRIVILEGED_BUILDER
        | VERTICAL_FLY_SPEED;
}

impl UpdateAbilities {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_i64_le(self.entity_id);
        w.write_u8(self.permission_level);
        w.write_u8(self.command_permission);

        w.write_u8(self.layers.len() as u8);
        for layer in &self.layers {
            w.write_u16_le(layer.layer_type);
            w.write_u32_le(layer.abilities_set);
            w.write_u32_le(layer.abilities_values);
            w.write_f32_le(layer.fly_speed);
            w.write_f32_le(layer.vertical_fly_speed);
            w.write_f32_le(layer.walk_speed);
        }

        w.into_bytes()
    }

    /// Survival mode abilities — no fly, can walk/mine/build/attack
    pub fn default_survival(entity_id: i64) -> Self {
        // ALL abilities are SET (we provide values for all of them)
        let set = ability::ALL;
        // Only these are ENABLED (true):
        let values = ability::BUILD
            | ability::MINE
            | ability::DOORS_AND_SWITCHES
            | ability::OPEN_CONTAINERS
            | ability::ATTACK_PLAYERS
            | ability::ATTACK_MOBS;
        // All others are false: FLYING, ALLOW_FLIGHT, NO_CLIP, INVULNERABLE,
        // OPERATOR, TELEPORT, INFINITE_RESOURCES, etc.

        Self {
            entity_id,
            permission_level: 1, // MEMBER (PMMP PlayerPermissions::MEMBER = 1, NOT 0 which is VISITOR)
            command_permission: 0, // NORMAL
            layers: vec![AbilitiesLayer {
                layer_type: 1, // BASE
                abilities_set: set,
                abilities_values: values,
                fly_speed: 0.05,
                vertical_fly_speed: 1.0,
                walk_speed: 0.1,
            }],
        }
    }

    /// Creative mode abilities — fly, invulnerable, infinite resources
    pub fn default_creative(entity_id: i64) -> Self {
        let set = ability::ALL;
        let values = ability::BUILD
            | ability::MINE
            | ability::DOORS_AND_SWITCHES
            | ability::OPEN_CONTAINERS
            | ability::ATTACK_PLAYERS
            | ability::ATTACK_MOBS
            | ability::ALLOW_FLIGHT
            | ability::FLYING
            | ability::INVULNERABLE
            | ability::INFINITE_RESOURCES
            | ability::FLY_SPEED
            | ability::WALK_SPEED
            | ability::NO_CLIP;

        Self {
            entity_id,
            permission_level: 2,   // OPERATOR
            command_permission: 1, // GAME_DIRECTORS
            layers: vec![AbilitiesLayer {
                layer_type: 1, // BASE
                abilities_set: set,
                abilities_values: values,
                fly_speed: 0.05,
                vertical_fly_speed: 1.0,
                walk_speed: 0.1,
            }],
        }
    }

    /// Spectator mode abilities — fly, noclip, no interactions (PMMP syncAbilities)
    pub fn default_spectator(entity_id: i64) -> Self {
        let set = ability::ALL;
        // Spectator: fly + noclip + invulnerable, NO build/mine/attack
        let values = ability::ALLOW_FLIGHT
            | ability::FLYING
            | ability::INVULNERABLE
            | ability::NO_CLIP
            | ability::FLY_SPEED
            | ability::WALK_SPEED;

        Self {
            entity_id,
            permission_level: 1,   // MEMBER
            command_permission: 0, // NORMAL
            layers: vec![
                // BASE layer
                AbilitiesLayer {
                    layer_type: 1, // BASE
                    abilities_set: set,
                    abilities_values: values,
                    fly_speed: 0.05,
                    vertical_fly_speed: 1.0,
                    walk_speed: 0.1,
                },
                // SPECTATOR layer — PMMP hack: forces FLYING=true so client
                // doesn't fall when clipping into blocks
                AbilitiesLayer {
                    layer_type: 2, // SPECTATOR
                    abilities_set: ability::FLYING,
                    abilities_values: ability::FLYING,
                    fly_speed: 0.0,
                    vertical_fly_speed: 0.0,
                    walk_speed: 0.0,
                },
            ],
        }
    }
}

// ── UpdateAttributes (S→C, 0x1D) ──

pub struct PlayerAttribute {
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub current: f32,
    pub default: f32,
}

pub struct UpdateAttributes {
    pub runtime_entity_id: u64,
    pub attributes: Vec<PlayerAttribute>,
    pub tick: u64,
}

impl UpdateAttributes {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(256);
        w.write_var_u64(self.runtime_entity_id);
        w.write_var_u32(self.attributes.len() as u32);
        for attr in &self.attributes {
            w.write_f32_le(attr.min);
            w.write_f32_le(attr.max);
            w.write_f32_le(attr.current);
            w.write_f32_le(attr.min); // default_min
            w.write_f32_le(attr.max); // default_max
            w.write_f32_le(attr.default);
            w.write_string(&attr.name);
            w.write_var_u32(0); // modifiers count = 0
        }
        w.write_var_u64(self.tick);
        w.into_bytes()
    }

    /// Default attributes for a survival player.
    pub fn default_survival(runtime_entity_id: u64) -> Self {
        Self {
            runtime_entity_id,
            attributes: vec![
                PlayerAttribute {
                    name: "minecraft:health".to_string(),
                    min: 0.0,
                    max: 20.0,
                    current: 20.0,
                    default: 20.0,
                },
                PlayerAttribute {
                    name: "minecraft:player.hunger".to_string(),
                    min: 0.0,
                    max: 20.0,
                    current: 20.0,
                    default: 20.0,
                },
                PlayerAttribute {
                    name: "minecraft:player.saturation".to_string(),
                    min: 0.0,
                    max: 20.0,
                    current: 20.0,
                    default: 20.0,
                },
                PlayerAttribute {
                    name: "minecraft:movement".to_string(),
                    min: 0.0,
                    max: 3.4028235e38,
                    current: 0.1,
                    default: 0.1,
                },
                PlayerAttribute {
                    name: "minecraft:attack_damage".to_string(),
                    min: 0.0,
                    max: 3.4028235e38,
                    current: 1.0,
                    default: 1.0,
                },
                PlayerAttribute {
                    name: "minecraft:absorption".to_string(),
                    min: 0.0,
                    max: 3.4028235e38,
                    current: 0.0,
                    default: 0.0,
                },
                PlayerAttribute {
                    name: "minecraft:knockback_resistance".to_string(),
                    min: 0.0,
                    max: 1.0,
                    current: 0.0,
                    default: 0.0,
                },
                PlayerAttribute {
                    name: "minecraft:follow_range".to_string(),
                    min: 0.0,
                    max: 2048.0,
                    current: 16.0,
                    default: 16.0,
                },
                PlayerAttribute {
                    name: "minecraft:player.level".to_string(),
                    min: 0.0,
                    max: 24791.0,
                    current: 0.0,
                    default: 0.0,
                },
                PlayerAttribute {
                    name: "minecraft:player.experience".to_string(),
                    min: 0.0,
                    max: 1.0,
                    current: 0.0,
                    default: 0.0,
                },
            ],
            tick: 0,
        }
    }
}

// ── SetActorData (S→C, 0x27) ──

pub struct SetActorData {
    pub runtime_entity_id: u64,
    pub metadata: Vec<(u32, u32, MetadataValue)>, // (key, type, value)
    pub tick: u64,
}

pub enum MetadataValue {
    Byte(u8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    Long(i64),
}

/// Entity metadata flag bits (from PMMP EntityMetadataFlags.php)
#[allow(dead_code)]
/// Entity metadata flag bits (from PMMP EntityMetadataFlags.php)
pub mod entity_flags {
    pub const ONFIRE: i64 = 1 << 0;
    pub const SNEAKING: i64 = 1 << 1;
    pub const RIDING: i64 = 1 << 2;
    pub const SPRINTING: i64 = 1 << 3;
    pub const USING_ITEM: i64 = 1 << 4;
    pub const INVISIBLE: i64 = 1 << 5; // PMMP INVISIBLE = 5
    pub const CAN_SHOW_NAMETAG: i64 = 1 << 14; // PMMP = 14 (was wrongly at bit 5!)
    pub const NO_AI: i64 = 1 << 16; // aka IMMOBILE — disables client physics
    pub const SILENT: i64 = 1 << 17; // no footstep/ambient sounds
    pub const CAN_CLIMB: i64 = 1 << 19;
    pub const CAN_FLY: i64 = 1 << 21;
    pub const BREATHING: i64 = 1 << 35; // NOT in water
    pub const HAS_COLLISION: i64 = 1 << 48;
    pub const HAS_GRAVITY: i64 = 1 << 49; // AFFECTED_BY_GRAVITY
}

impl SetActorData {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_var_u64(self.runtime_entity_id);
        w.write_var_u32(self.metadata.len() as u32);
        for (key, data_type, value) in &self.metadata {
            w.write_var_u32(*key);
            w.write_var_u32(*data_type);
            match value {
                MetadataValue::Byte(v) => w.write_u8(*v),
                MetadataValue::Short(v) => w.write_i16_le(*v),
                MetadataValue::Int(v) => w.write_var_i32(*v),
                MetadataValue::Float(v) => w.write_f32_le(*v),
                MetadataValue::String(v) => w.write_string(v),
                MetadataValue::Long(v) => w.write_var_i64(*v),
            }
        }
        // PropertySyncData (PMMP) — BEFORE tick!
        w.write_var_u32(0); // property_int_count
        w.write_var_u32(0); // property_float_count
        w.write_var_u64(self.tick);
        w.into_bytes()
    }

    /// Player metadata for pre-spawn (NO_AI=true to freeze during chunk loading)
    pub fn player_pre_spawn(runtime_entity_id: u64, name: &str) -> Self {
        let flags = entity_flags::CAN_SHOW_NAMETAG
            | entity_flags::NO_AI       // freeze during pre-spawn
            | entity_flags::BREATHING   // not underwater
            | entity_flags::HAS_GRAVITY // affected by gravity (once NO_AI is removed)
            | entity_flags::HAS_COLLISION;
        Self {
            runtime_entity_id,
            metadata: vec![
                (0, 7, MetadataValue::Long(flags)),
                (4, 4, MetadataValue::String(name.to_string())),
                (38, 3, MetadataValue::Float(1.0)), // SCALE (PMMP = 38)
                (53, 3, MetadataValue::Float(0.6)), // BOUNDING_BOX_WIDTH (PMMP = 53)
                (54, 3, MetadataValue::Float(1.8)), // BOUNDING_BOX_HEIGHT (PMMP = 54)
            ],
            tick: 0,
        }
    }

    /// Player metadata for in-game (NO_AI=false, gravity active)
    pub fn player_in_game(runtime_entity_id: u64, name: &str) -> Self {
        let flags = entity_flags::CAN_SHOW_NAMETAG
            | entity_flags::BREATHING
            | entity_flags::HAS_GRAVITY
            | entity_flags::HAS_COLLISION;
        // NO_AI is NOT set — client can apply physics
        Self {
            runtime_entity_id,
            metadata: vec![
                (0, 7, MetadataValue::Long(flags)),
                (4, 4, MetadataValue::String(name.to_string())),
                (38, 3, MetadataValue::Float(1.0)), // SCALE
                (53, 3, MetadataValue::Float(0.6)), // BOUNDING_BOX_WIDTH
                (54, 3, MetadataValue::Float(1.8)), // BOUNDING_BOX_HEIGHT
            ],
            tick: 0,
        }
    }

    /// Player metadata for spectator mode (no collision, silent, flying)
    pub fn player_spectator(runtime_entity_id: u64, name: &str) -> Self {
        let flags = entity_flags::CAN_SHOW_NAMETAG
            | entity_flags::BREATHING
            | entity_flags::HAS_GRAVITY
            | entity_flags::SILENT; // no footstep sounds
                                    // HAS_COLLISION is NOT set — noclip through blocks
        Self {
            runtime_entity_id,
            metadata: vec![
                (0, 7, MetadataValue::Long(flags)),
                (4, 4, MetadataValue::String(name.to_string())),
                (38, 3, MetadataValue::Float(1.0)), // SCALE
                (53, 3, MetadataValue::Float(0.6)), // BOUNDING_BOX_WIDTH
                (54, 3, MetadataValue::Float(1.8)), // BOUNDING_BOX_HEIGHT
            ],
            tick: 0,
        }
    }
}

// ── ItemStack / ItemStackWrapper ──

/// An item stack with network encoding.
#[derive(Clone, Debug)]
pub struct ItemStack {
    pub id: i32, // network item ID (0 = air/empty)
    pub count: u16,
    pub meta: u32,
    pub block_runtime_id: i32, // for block items, the block runtime ID
    pub extra_data: Vec<u8>,   // raw extra data (empty for basic items)
}

impl ItemStack {
    pub const AIR: Self = Self {
        id: 0,
        count: 0,
        meta: 0,
        block_runtime_id: 0,
        extra_data: Vec::new(),
    };

    pub fn new(id: i32, count: u16, block_runtime_id: i32) -> Self {
        Self {
            id,
            count,
            meta: 0,
            block_runtime_id,
            extra_data: Vec::new(),
        }
    }

    pub fn is_air(&self) -> bool {
        self.id == 0 || self.count == 0
    }
}

/// ItemStackWrapper adds a server-assigned stack ID for inventory tracking.
#[derive(Clone, Debug)]
pub struct ItemStackWrapper {
    pub stack_id: i32,
    pub item: ItemStack,
}

impl ItemStackWrapper {
    pub fn air() -> Self {
        Self {
            stack_id: 0,
            item: ItemStack::AIR,
        }
    }

    pub fn new(item: ItemStack, stack_id: i32) -> Self {
        Self { stack_id, item }
    }

    /// Encode to network bytes (PMMP CommonTypes::writeItemStackWrapper).
    pub fn encode(&self, w: &mut ProtoWriter) {
        w.write_var_i32(self.item.id);
        if self.item.id == 0 {
            return;
        }
        w.write_u16_le(self.item.count);
        w.write_var_u32(self.item.meta);
        let has_net_id = self.stack_id != 0;
        w.write_bool(has_net_id);
        if has_net_id {
            w.write_var_i32(self.stack_id);
        }
        w.write_var_i32(self.item.block_runtime_id);
        // Extra data as length-prefixed string
        w.write_var_u32(self.item.extra_data.len() as u32);
        if !self.item.extra_data.is_empty() {
            w.write_raw(&self.item.extra_data);
        }
    }
}

// ── InventoryContent (S→C, 0x31) ──

pub struct InventoryContent;

impl InventoryContent {
    /// Encode an empty inventory for a window (all air items).
    pub fn encode_empty(window_id: u32, slot_count: u32, container_id: u8) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_var_u32(window_id);
        w.write_var_u32(slot_count);
        for _ in 0..slot_count {
            w.write_var_i32(0); // air
        }
        w.write_u8(container_id);
        w.write_bool(false);
        w.write_var_i32(0);
        w.into_bytes()
    }

    /// Encode inventory with actual items.
    pub fn encode_items(window_id: u32, items: &[ItemStackWrapper], container_id: u8) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(256);
        w.write_var_u32(window_id);
        w.write_var_u32(items.len() as u32);
        for item in items {
            item.encode(&mut w);
        }
        w.write_u8(container_id);
        w.write_bool(false);
        w.write_var_i32(0);
        w.into_bytes()
    }
}

// ── InventorySlot (S→C, 0x32) ──

pub struct InventorySlot;

impl InventorySlot {
    /// Encode a single slot update.
    pub fn encode(window_id: u32, slot: u32, item: &ItemStackWrapper, container_id: u8) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_var_u32(window_id);
        w.write_var_u32(slot);
        // FullContainerName
        w.write_u8(container_id);
        w.write_bool(false);
        // Storage item
        w.write_var_i32(0);
        // Item
        item.encode(&mut w);
        w.into_bytes()
    }
}

// ── MobEquipment (S→C, 0x1F) ──

pub struct MobEquipment;

impl MobEquipment {
    /// Encode empty hand (air item).
    pub fn encode_empty(runtime_entity_id: u64) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(16);
        w.write_var_u64(runtime_entity_id);
        w.write_var_i32(0); // air
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(0);
        w.into_bytes()
    }

    /// Encode with actual held item.
    pub fn encode_item(
        runtime_entity_id: u64,
        item: &ItemStackWrapper,
        hotbar_slot: u8,
    ) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_var_u64(runtime_entity_id);
        item.encode(&mut w);
        w.write_u8(hotbar_slot); // inventory_slot
        w.write_u8(hotbar_slot); // hotbar_slot
        w.write_u8(0); // container_id (inventory)
        w.into_bytes()
    }
}

// ── UpdateAdventureSettings (S→C, 0x12C) ──

pub struct UpdateAdventureSettings {
    pub no_pvm: bool,
    pub no_mvp: bool,
    pub immutable_world: bool,
    pub show_name_tags: bool,
    pub auto_jump: bool,
}

impl UpdateAdventureSettings {
    /// Default adventure settings for survival (matches PMMP syncAdventureSettings)
    pub fn default_survival() -> Self {
        Self {
            no_pvm: false,
            no_mvp: false,
            immutable_world: false,
            show_name_tags: true,
            auto_jump: true,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(8);
        w.write_bool(self.no_pvm);
        w.write_bool(self.no_mvp);
        w.write_bool(self.immutable_world);
        w.write_bool(self.show_name_tags);
        w.write_bool(self.auto_jump);
        w.into_bytes()
    }
}
