use crate::io::{ProtoReader, ProtoWriter};

// ── PlayerAuthInput (C→S, 0x90) ──

/// Block action from PlayerAuthInput.
pub struct BlockAction {
    pub action_type: i32,
    pub position: [i32; 3],
    pub face: i32,
}

/// Item interaction data from PlayerAuthInput (UseItemTransactionData).
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct SlotInfo {
    pub container_id: u8,
    pub slot_id: u8,
    pub stack_id: i32,
}

/// A single action in an ItemStackRequest.
#[derive(Clone, Debug)]
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
    /// Décrément durabilité côté serveur quand le client casse un bloc.
    /// PMMP `MineBlockStackRequestAction`. Sans gestion durabilité, on
    /// se contente d'ack pour ne pas resync.
    MineBlock {
        hotbar_slot: i32,
        predicted_durability: i32,
        network_stack_id: i32,
    },
    /// Créatif : le client demande un item du creative inventory.
    /// PMMP `CreativeCreateStackRequestAction`.
    CraftCreative {
        creative_item_network_id: u32,
    },
    /// PMMP `CraftRecipeStackRequestAction` (action_type=12).
    /// Le client envoie le recipe_id (que le serveur a annoncé via
    /// CraftingDataPacket) + `times` = nombre de fois qu'on souhaite craft.
    CraftRecipe {
        recipe_id: u32,
        times: u8,
    },
    /// `CraftRecipeAuto` (action_type=13) : recipe + times + ingredients
    /// list (ItemStack par ingredient). Utilisé par le recipe book quand
    /// le joueur a coché "auto-craft from inventory".
    CraftRecipeAuto {
        recipe_id: u32,
        times: u8,
    },
    /// Other actions we don't handle yet.
    Unknown(u8),
}

/// An ItemStackRequest decoded from PlayerAuthInput.
#[derive(Clone, Debug)]
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
    /// Bitset brut des PlayerAuthInputFlags — exposé pour diag côté server
    /// (ex: vérifier si PERFORM_ITEM_INTERACTION=34 est bien set au placement).
    pub input_flags: u128,
    pub block_actions: Vec<BlockAction>,
    pub item_interaction: Option<ItemInteractionData>,
    pub item_stack_request: Option<ItemStackRequest>,
}

/// Container/slot resync hint carried by InventoryTransaction when request_id != 0.
#[derive(Clone, Debug)]
pub struct InventoryTransactionChangedSlots {
    pub container_id: u8,
    pub changed_slots: Vec<u8>,
}

/// A single low-level inventory action from InventoryTransaction.
#[derive(Clone, Debug)]
pub struct NetworkInventoryAction {
    pub source_type: u32,
    pub window_id: Option<i32>,
    pub source_flags: Option<u32>,
    pub inventory_slot: u32,
    pub old_item: ItemStackWrapper,
    pub new_item: ItemStackWrapper,
}

/// InventoryTransaction payload variants we care about server-side.
#[derive(Clone, Debug)]
pub enum InventoryTransactionData {
    Normal {
        actions: Vec<NetworkInventoryAction>,
    },
    Mismatch {
        actions: Vec<NetworkInventoryAction>,
    },
    UseItem {
        actions: Vec<NetworkInventoryAction>,
        data: ItemInteractionData,
    },
    UseItemOnEntity {
        actions: Vec<NetworkInventoryAction>,
        actor_runtime_id: u64,
        action_type: u32,
        hotbar_slot: i32,
        item_in_hand: ItemStackWrapper,
        player_position: [f32; 3],
        click_position: [f32; 3],
    },
    ReleaseItem {
        actions: Vec<NetworkInventoryAction>,
        action_type: u32,
        hotbar_slot: i32,
        item_in_hand: ItemStackWrapper,
        head_position: [f32; 3],
    },
    Unknown {
        transaction_type: u32,
        actions: Vec<NetworkInventoryAction>,
        remaining_data: Vec<u8>,
    },
}

/// InventoryTransaction packet (0x1E).
#[derive(Clone, Debug)]
pub struct InventoryTransaction {
    pub request_id: i32,
    pub changed_slots: Vec<InventoryTransactionChangedSlots>,
    pub data: InventoryTransactionData,
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
fn decode_item_stack_wrapper(
    reader: &mut ProtoReader,
) -> Result<ItemStackWrapper, crate::io::reader::ProtoReadError> {
    let id = reader.read_var_i32()?;
    if id == 0 {
        return Ok(ItemStackWrapper::air());
    }
    let count = reader.read_u16_le()?;
    let meta = reader.read_var_u32()?;
    let has_net_id = reader.read_bool()?;
    let stack_id = if has_net_id {
        reader.read_var_i32()?
    } else {
        0
    };
    let block_runtime_id = reader.read_var_i32()?;
    let extra_data = reader.read_byte_array()?;
    Ok(ItemStackWrapper::new(
        ItemStack {
            id,
            count,
            meta,
            block_runtime_id,
            extra_data,
        },
        stack_id,
    ))
}

/// Skip an ItemStackWrapper in the reader.
fn skip_item_stack_wrapper(
    reader: &mut ProtoReader,
) -> Result<(), crate::io::reader::ProtoReadError> {
    let _ = decode_item_stack_wrapper(reader)?;
    Ok(())
}

/// Decode a NetworkInventoryAction from the reader.
fn decode_network_inventory_action(
    reader: &mut ProtoReader,
) -> Result<NetworkInventoryAction, crate::io::reader::ProtoReadError> {
    let source_type = reader.read_var_u32()?;
    let (window_id, source_flags) = match source_type {
        0 => (Some(reader.read_var_i32()?), None), // SOURCE_CONTAINER
        2 => (None, Some(reader.read_var_u32()?)), // SOURCE_WORLD
        3 => (None, None),                         // SOURCE_CREATIVE
        99999 => (Some(reader.read_var_i32()?), None), // SOURCE_TODO
        _ => (None, None),
    };
    let inventory_slot = reader.read_var_u32()?;
    let old_item = decode_item_stack_wrapper(reader)?;
    let new_item = decode_item_stack_wrapper(reader)?;

    Ok(NetworkInventoryAction {
        source_type,
        window_id,
        source_flags,
        inventory_slot,
        old_item,
        new_item,
    })
}

fn decode_inventory_actions(
    reader: &mut ProtoReader,
) -> Result<Vec<NetworkInventoryAction>, crate::io::reader::ProtoReadError> {
    let action_count = reader.read_var_u32()?;
    let mut actions = Vec::new();
    for _ in 0..action_count.min(100) {
        actions.push(decode_network_inventory_action(reader)?);
    }
    Ok(actions)
}

fn decode_changed_slots_hack(
    reader: &mut ProtoReader,
) -> Result<Vec<InventoryTransactionChangedSlots>, crate::io::reader::ProtoReadError> {
    let changed_slots_count = reader.read_var_u32()?;
    let mut changed_slots = Vec::new();
    for _ in 0..changed_slots_count.min(32) {
        let container_id = reader.read_u8()?;
        let slot_count = reader.read_var_u32()?;
        let mut slots = Vec::new();
        for _ in 0..slot_count.min(128) {
            slots.push(reader.read_u8()?);
        }
        changed_slots.push(InventoryTransactionChangedSlots {
            container_id,
            changed_slots: slots,
        });
    }
    Ok(changed_slots)
}

fn decode_use_item_transaction_data(
    reader: &mut ProtoReader,
) -> Result<ItemInteractionData, crate::io::reader::ProtoReadError> {
    let action_type = reader.read_var_u32()?;
    let trigger_type = reader.read_var_u32()?;
    let _ = trigger_type;

    // Bedrock `BlockPosition` = 3 VarInt SIGNED (PMMP CommonTypes::getBlockPosition
    // + gophertunnel BlockPos). Lire Y comme unsigned donnait Y=zig-zag(vrai_Y)
    // (ex: vrai_Y=68 → reçu comme 136), décalant toute la suite de la chaîne
    // place → break.
    let bx = reader.read_var_i32()?;
    let by = reader.read_var_i32()?;
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
    // Protocol 944 (PMMP UseItemTransactionData.php:98) : clientCooldownState u8
    // ajouté après clientInteractPrediction. Sans ce read, on est décalé d'1
    // byte → item_stack_request suivant se décode sur un mauvais offset →
    // block_place silencieusement ignoré.
    let _client_cooldown_state = reader.read_u8()?;

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

fn decode_use_item_on_entity_transaction_data(
    reader: &mut ProtoReader,
) -> Result<InventoryTransactionData, crate::io::reader::ProtoReadError> {
    let actor_runtime_id = reader.read_var_u64()?;
    let action_type = reader.read_var_u32()?;
    let hotbar_slot = reader.read_var_i32()?;
    let item_in_hand = decode_item_stack_wrapper(reader)?;
    let player_position = [
        reader.read_f32_le()?,
        reader.read_f32_le()?,
        reader.read_f32_le()?,
    ];
    let click_position = [
        reader.read_f32_le()?,
        reader.read_f32_le()?,
        reader.read_f32_le()?,
    ];
    Ok(InventoryTransactionData::UseItemOnEntity {
        actions: Vec::new(),
        actor_runtime_id,
        action_type,
        hotbar_slot,
        item_in_hand,
        player_position,
        click_position,
    })
}

fn decode_release_item_transaction_data(
    reader: &mut ProtoReader,
) -> Result<InventoryTransactionData, crate::io::reader::ProtoReadError> {
    let action_type = reader.read_var_u32()?;
    let hotbar_slot = reader.read_var_i32()?;
    let item_in_hand = decode_item_stack_wrapper(reader)?;
    let head_position = [
        reader.read_f32_le()?,
        reader.read_f32_le()?,
        reader.read_f32_le()?,
    ];
    Ok(InventoryTransactionData::ReleaseItem {
        actions: Vec::new(),
        action_type,
        hotbar_slot,
        item_in_hand,
        head_position,
    })
}

impl InventoryTransaction {
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let request_id = reader.read_var_i32()?;
        let changed_slots = if request_id != 0 {
            decode_changed_slots_hack(reader)?
        } else {
            Vec::new()
        };

        let transaction_type = reader.read_var_u32()?;
        let actions = decode_inventory_actions(reader)?;

        let data = match transaction_type {
            0 => InventoryTransactionData::Normal { actions },
            1 => InventoryTransactionData::Mismatch { actions },
            2 => InventoryTransactionData::UseItem {
                actions,
                data: decode_use_item_transaction_data(reader)?,
            },
            3 => {
                let mut data = decode_use_item_on_entity_transaction_data(reader)?;
                if let InventoryTransactionData::UseItemOnEntity {
                    actions: tx_actions,
                    ..
                } = &mut data
                {
                    *tx_actions = actions;
                }
                data
            }
            4 => {
                let mut data = decode_release_item_transaction_data(reader)?;
                if let InventoryTransactionData::ReleaseItem {
                    actions: tx_actions,
                    ..
                } = &mut data
                {
                    *tx_actions = actions;
                }
                data
            }
            _ => InventoryTransactionData::Unknown {
                transaction_type,
                actions,
                remaining_data: reader.read_remaining(),
            },
        };

        Ok(Self {
            request_id,
            changed_slots,
            data,
        })
    }
}

/// Decode the ItemInteractionData from PlayerAuthInput.
fn decode_item_interaction(
    reader: &mut ProtoReader,
) -> Result<ItemInteractionData, crate::io::reader::ProtoReadError> {
    // Legacy request ID
    let request_id = reader.read_var_i32()?;
    if request_id != 0 {
        let _ = decode_changed_slots_hack(reader)?;
    }

    let _ = decode_inventory_actions(reader)?;
    decode_use_item_transaction_data(reader)
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
                let hotbar_slot = reader.read_var_i32()?;
                let predicted_durability = reader.read_var_i32()?;
                let network_stack_id = reader.read_var_i32()?;
                StackRequestAction::MineBlock {
                    hotbar_slot,
                    predicted_durability,
                    network_stack_id,
                }
            }
            12 => {
                let recipe_id = reader.read_var_u32()?;
                let times = reader.read_u8()?;
                StackRequestAction::CraftRecipe { recipe_id, times }
            }
            13 => {
                // PMMP CraftRecipeAutoStackRequestAction::read :
                //   recipeId (VarU32) + repetitions (u8) + repetitions2 (u8 — Mojang dup)
                //   ingredient_count (u8) + foreach: RecipeIngredient
                // RecipeIngredient = descriptor_type (u8) + descriptor body + count (var_i32)
                //   descriptor_type = 0=NONE, 1=INT_ID_META, 2=STRING_ID_META, 3=TAG, 4=MOLANG, 5=COMPLEX_ALIAS
                let recipe_id = reader.read_var_u32()?;
                let times = reader.read_u8()?;
                let _times2 = reader.read_u8()?;
                let ingredient_count = reader.read_u8()?;
                for _ in 0..ingredient_count {
                    let descriptor_type = reader.read_u8()?;
                    match descriptor_type {
                        1 => {
                            // IntIdMeta : i16 LE id + (if id!=0) i16 LE meta
                            let id = reader.read_i16_le()?;
                            if id != 0 {
                                let _meta = reader.read_i16_le()?;
                            }
                        }
                        2 => {
                            // StringIdMeta : string + u16 LE meta
                            let _string_id = reader.read_string()?;
                            let _meta = reader.read_u16_le()?;
                        }
                        3 => {
                            // Tag : string
                            let _tag = reader.read_string()?;
                        }
                        4 => {
                            // Molang : string + u8 version
                            let _expr = reader.read_string()?;
                            let _version = reader.read_u8()?;
                        }
                        5 => {
                            // ComplexAlias : string
                            let _alias = reader.read_string()?;
                        }
                        _ => {}
                    }
                    let _count = reader.read_var_i32()?;
                }
                StackRequestAction::CraftRecipeAuto { recipe_id, times }
            }
            14 => {
                // PMMP CreativeCreateStackRequestAction::read :
                //   creativeItemId: VarU32
                //   repetitions:    u8
                // Sans le read du u8, les actions suivantes (Place/Take) se
                // décodent 1 byte trop tôt → slot_info corrompu → items disparus.
                let creative_item_network_id = reader.read_var_u32()?;
                let _repetitions = reader.read_u8()?;
                StackRequestAction::CraftCreative {
                    creative_item_network_id,
                }
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
                Err(_e) => {
                    return Ok(Self {
                        pitch,
                        yaw,
                        position: [pos_x, pos_y, pos_z],
                        move_vec_x,
                        move_vec_z,
                        head_yaw,
                        input_flags,
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
                Err(_e) => {
                    return Ok(Self {
                        pitch,
                        yaw,
                        position: [pos_x, pos_y, pos_z],
                        move_vec_x,
                        move_vec_z,
                        head_yaw,
                        input_flags,
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
            input_flags,
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

    /// Encode a JSON_WHISPER message (type 10) carrying a Bedrock rawtext JSON
    /// string — utilisé par /tellraw. Le client parse le JSON et rend le rawtext
    /// array (text/translate/with/score/selector).
    pub fn json(json: &str) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(json.len() + 32);
        w.write_bool(false); // needsTranslation
        w.write_u8(0); // category = MESSAGE_ONLY
        w.write_u8(10); // type = JSON_WHISPER (PMMP TextPacket::TYPE_JSON_WHISPER)
        w.write_string(json);
        w.write_string(""); // xboxUserId
        w.write_string(""); // platformChatId
        w.write_bool(false); // filteredMessage = None
        w.into_bytes()
    }
}

// ── Transfer (S→C, 0x55) ──

pub struct Transfer {
    pub address: String,
    pub port: u16,
    pub reload_world: bool,
}

impl Transfer {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_string(&self.address);
        w.write_u16_le(self.port);
        w.write_bool(self.reload_world);
        w.into_bytes()
    }
}

// ── SetTitle (S→C, 0x58) ──

pub struct SetTitle {
    pub action_type: i32,
    pub text: String,
    pub fade_in_time: i32,
    pub stay_time: i32,
    pub fade_out_time: i32,
    pub xuid: String,
    pub platform_online_id: String,
    pub filtered_text: String,
}

impl SetTitle {
    pub const TYPE_CLEAR: i32 = 0;
    pub const TYPE_RESET: i32 = 1;
    pub const TYPE_TITLE: i32 = 2;
    pub const TYPE_SUBTITLE: i32 = 3;
    pub const TYPE_ACTIONBAR: i32 = 4;
    pub const TYPE_TIMES: i32 = 5;

    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(96);
        w.write_var_i32(self.action_type);
        w.write_string(&self.text);
        w.write_var_i32(self.fade_in_time);
        w.write_var_i32(self.stay_time);
        w.write_var_i32(self.fade_out_time);
        w.write_string(&self.xuid);
        w.write_string(&self.platform_online_id);
        w.write_string(&self.filtered_text);
        w.into_bytes()
    }

    pub fn simple(action_type: i32, text: impl Into<String>) -> Self {
        Self {
            action_type,
            text: text.into(),
            fade_in_time: 0,
            stay_time: 0,
            fade_out_time: 0,
            xuid: String::new(),
            platform_online_id: String::new(),
            filtered_text: String::new(),
        }
    }

    pub fn times(fade_in_time: i32, stay_time: i32, fade_out_time: i32) -> Self {
        Self {
            action_type: Self::TYPE_TIMES,
            text: String::new(),
            fade_in_time,
            stay_time,
            fade_out_time,
            xuid: String::new(),
            platform_online_id: String::new(),
            filtered_text: String::new(),
        }
    }
}

// ── PlayerList (S→C, 0x3F) ──

/// Skin sérialisé wire-format Bedrock (protocol 944+). Port partiel de PMMP
/// `SerializedSkin.php` — couvre les champs minimum nécessaires pour qu'un
/// client affiche correctement le joueur. Si `Default` est utilisé, le client
/// voit un Steve transparent (texture 64x64 RGBA tout-à-zéro).
#[derive(Debug, Clone)]
pub struct SerializedSkin {
    pub skin_id: String,
    pub play_fab_id: String,
    pub skin_resource_patch: String,
    pub skin_width: u32,
    pub skin_height: u32,
    pub skin_data: Vec<u8>,
    pub cape_width: u32,
    pub cape_height: u32,
    pub cape_data: Vec<u8>,
    pub geometry_data: String,
    pub geometry_data_engine_version: String,
    pub animation_data: String,
    pub cape_id: String,
    pub full_skin_id: String,
    pub arm_size: String,
    pub skin_color: String,
    pub premium: bool,
    pub persona: bool,
    pub persona_cape_on_classic: bool,
    pub primary_user: bool,
}

impl Default for SerializedSkin {
    fn default() -> Self {
        Self {
            skin_id: "Custom".to_string(),
            play_fab_id: String::new(),
            skin_resource_patch: "geometry.humanoid.custom".to_string(),
            skin_width: 64,
            skin_height: 64,
            skin_data: vec![0u8; 64 * 64 * 4],
            cape_width: 0,
            cape_height: 0,
            cape_data: Vec::new(),
            geometry_data: String::new(),
            geometry_data_engine_version: String::new(),
            animation_data: String::new(),
            cape_id: String::new(),
            full_skin_id: String::new(),
            arm_size: String::new(),
            skin_color: String::new(),
            premium: false,
            persona: false,
            persona_cape_on_classic: false,
            primary_user: false,
        }
    }
}

pub struct PlayerListAdd {
    pub uuid: [u8; 16],
    pub entity_id: i64, // varint64 (actor unique ID)
    pub username: String,
    pub xuid: String,
    pub platform_chat_id: String,
    pub build_platform: i32, // i32_le
    pub skin: SerializedSkin,
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
                write_serialized_skin(&mut w, &entry.skin);
                w.write_bool(entry.is_teacher);
                w.write_bool(entry.is_host);
                w.write_bool(entry.is_subclient);
                // Protocol 944 : ARGB color u32 LE (PMMP PlayerListPacket.php:108,
                // ajouté dans la refonte 1.26.10). Default white = 0xFFFFFFFF.
                w.write_u32_le(0xFFFFFFFF);
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

fn write_serialized_skin(w: &mut ProtoWriter, skin: &SerializedSkin) {
    w.write_string(&skin.skin_id);
    w.write_string(&skin.play_fab_id);
    w.write_string(&skin.skin_resource_patch);
    // Skin image data
    w.write_u32_le(skin.skin_width);
    w.write_u32_le(skin.skin_height);
    w.write_byte_array(&skin.skin_data);

    // Animations — empty (extension future : itérer skin.animations)
    w.write_u32_le(0);

    // Cape image
    w.write_u32_le(skin.cape_width);
    w.write_u32_le(skin.cape_height);
    w.write_byte_array(&skin.cape_data);

    w.write_string(&skin.geometry_data);
    w.write_string(&skin.geometry_data_engine_version);
    w.write_string(&skin.animation_data);

    w.write_string(&skin.cape_id);
    w.write_string(&skin.full_skin_id);
    w.write_string(&skin.arm_size);
    w.write_string(&skin.skin_color);

    // Persona pieces — empty (extension future)
    w.write_u32_le(0);
    // Persona tint colors — empty
    w.write_u32_le(0);

    w.write_bool(skin.premium);
    w.write_bool(skin.persona);
    w.write_bool(skin.persona_cape_on_classic);
    w.write_bool(skin.primary_user);

    w.write_bool(false); // override_appearance (new in 1.26.x — false par défaut)
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

// ── AddItemActor (S→C, 0x0F) ──

#[derive(Clone, Debug)]
pub struct AddActorAttribute {
    pub name: String,
    pub min: f32,
    pub current: f32,
    pub max: f32,
}

pub struct AddActor {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub actor_type: String,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub body_yaw: f32,
    pub attributes: Vec<AddActorAttribute>,
    pub metadata: Vec<(u32, u32, MetadataValue)>,
}

impl AddActor {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(192);
        w.write_var_i64(self.entity_unique_id);
        w.write_var_u64(self.entity_runtime_id);
        w.write_string(&self.actor_type);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_f32_le(self.velocity[0]);
        w.write_f32_le(self.velocity[1]);
        w.write_f32_le(self.velocity[2]);
        w.write_f32_le(self.pitch);
        w.write_f32_le(self.yaw);
        w.write_f32_le(self.head_yaw);
        w.write_f32_le(self.body_yaw);
        w.write_var_u32(self.attributes.len() as u32);
        for attr in &self.attributes {
            w.write_string(&attr.name);
            w.write_f32_le(attr.min);
            w.write_f32_le(attr.current);
            w.write_f32_le(attr.max);
        }
        write_actor_metadata(&mut w, &self.metadata);
        w.write_var_u32(0); // synced int properties
        w.write_var_u32(0); // synced float properties
        w.write_var_u32(0); // actor links
        w.into_bytes()
    }
}

pub struct AddItemActor {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub item: ItemStackWrapper,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub metadata: Vec<(u32, u32, MetadataValue)>,
    pub is_from_fishing: bool,
}

impl AddItemActor {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_var_i64(self.entity_unique_id);
        w.write_var_u64(self.entity_runtime_id);
        self.item.encode(&mut w);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_f32_le(self.velocity[0]);
        w.write_f32_le(self.velocity[1]);
        w.write_f32_le(self.velocity[2]);
        write_actor_metadata(&mut w, &self.metadata);
        w.write_bool(self.is_from_fishing);
        w.into_bytes()
    }
}

// ── TakeItemActor (S→C, 0x11) ──

pub struct TakeItemActor {
    pub item_actor_runtime_id: u64,
    pub taker_actor_runtime_id: u64,
}

impl TakeItemActor {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(16);
        w.write_var_u64(self.item_actor_runtime_id);
        w.write_var_u64(self.taker_actor_runtime_id);
        w.into_bytes()
    }
}

pub struct MoveActorAbsolute {
    pub runtime_entity_id: u64,
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub flags: u8,
}

impl MoveActorAbsolute {
    pub const FLAG_GROUND: u8 = 1 << 0;
    pub const FLAG_TELEPORT: u8 = 1 << 1;

    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32);
        w.write_var_u64(self.runtime_entity_id);
        w.write_u8(self.flags);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_u8(angle_to_byte(self.pitch));
        w.write_u8(angle_to_byte(self.yaw));
        w.write_u8(angle_to_byte(self.head_yaw));
        w.into_bytes()
    }
}

pub struct SetActorMotion {
    pub runtime_entity_id: u64,
    pub motion: [f32; 3],
    /// Server tick at which the packet was sent. Used by the client in relation
    /// to CorrectPlayerMovePrediction. Must be present since protocol 419+ —
    /// omitting it corrupts the batch and crashes the client.
    pub tick: u64,
}

impl SetActorMotion {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32);
        w.write_var_u64(self.runtime_entity_id);
        w.write_f32_le(self.motion[0]);
        w.write_f32_le(self.motion[1]);
        w.write_f32_le(self.motion[2]);
        w.write_var_u64(self.tick);
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

    /// Survival mode abilities — no fly, can walk/mine/build/attack.
    /// `is_op` drives `permission_level` + `command_permission` + OPERATOR bit,
    /// strictement comme PMMP `NetworkSession::syncAbilities` (permissions liées
    /// à `hasPermission(ROOT_OPERATOR)`, pas au gamemode).
    pub fn default_survival(entity_id: i64, is_op: bool) -> Self {
        let set = ability::ALL;
        let mut values = ability::BUILD
            | ability::MINE
            | ability::DOORS_AND_SWITCHES
            | ability::OPEN_CONTAINERS
            | ability::ATTACK_PLAYERS
            | ability::ATTACK_MOBS;
        if is_op {
            values |= ability::OPERATOR | ability::TELEPORT;
        }

        Self {
            entity_id,
            permission_level: if is_op { 2 } else { 1 },
            command_permission: if is_op { 1 } else { 0 },
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

    /// Creative mode abilities — allow_flight + invulnerable + infinite_resources.
    /// FLYING n'est pas auto-enabled (PMMP lit `isFlying()` courant ; le joueur
    /// double-jump pour activer le vol). NO_CLIP reste désactivé (collision
    /// normale en créatif — seul le spectator a `!hasBlockCollision()`).
    pub fn default_creative(entity_id: i64, is_op: bool) -> Self {
        let set = ability::ALL;
        let mut values = ability::BUILD
            | ability::MINE
            | ability::DOORS_AND_SWITCHES
            | ability::OPEN_CONTAINERS
            | ability::ATTACK_PLAYERS
            | ability::ATTACK_MOBS
            | ability::ALLOW_FLIGHT
            | ability::INVULNERABLE
            | ability::INFINITE_RESOURCES
            | ability::FLY_SPEED
            | ability::WALK_SPEED;
        if is_op {
            values |= ability::OPERATOR | ability::TELEPORT;
        }

        Self {
            entity_id,
            permission_level: if is_op { 2 } else { 1 },
            command_permission: if is_op { 1 } else { 0 },
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
    pub fn default_spectator(entity_id: i64, is_op: bool) -> Self {
        let set = ability::ALL;
        let mut values = ability::ALLOW_FLIGHT
            | ability::FLYING
            | ability::INVULNERABLE
            | ability::NO_CLIP
            | ability::FLY_SPEED
            | ability::WALK_SPEED;
        if is_op {
            values |= ability::OPERATOR | ability::TELEPORT;
        }

        Self {
            entity_id,
            permission_level: if is_op { 2 } else { 1 },
            command_permission: if is_op { 1 } else { 0 },
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

#[derive(Clone, Debug)]
pub enum MetadataValue {
    Byte(u8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    Long(i64),
}

fn write_actor_metadata(w: &mut ProtoWriter, metadata: &[(u32, u32, MetadataValue)]) {
    w.write_var_u32(metadata.len() as u32);
    let mut ordered = metadata.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(key, _, _)| *key);
    for (key, data_type, value) in ordered {
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
}

fn angle_to_byte(angle: f32) -> u8 {
    let wrapped = angle.rem_euclid(360.0);
    ((wrapped * 256.0 / 360.0).round() as i32 & 0xFF) as u8
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
        write_actor_metadata(&mut w, &self.metadata);
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

    pub fn legacy(item: ItemStack) -> Self {
        let stack_id = if item.is_air() { 0 } else { 1 };
        Self { stack_id, item }
    }

    pub fn new(item: ItemStack, stack_id: i32) -> Self {
        Self { stack_id, item }
    }

    /// Encode to network bytes.
    ///
    /// Mirrors PMMP `ItemStackWrapper::write()` — the `stack_id` field is
    /// authoritative: 0 means "no net id" (air or legacy), non-zero means
    /// the server-assigned unique stack ID (set by `InventoryManager`).
    pub fn encode(&self, w: &mut ProtoWriter) {
        w.write_var_i32(self.item.id);
        if self.item.id == 0 {
            return;
        }
        w.write_u16_le(self.item.count);
        w.write_var_u32(self.item.meta);
        let stack_id = if self.item.is_air() { 0 } else { self.stack_id };
        let has_net_id = stack_id != 0;
        w.write_bool(has_net_id);
        if has_net_id {
            w.write_var_i32(stack_id);
        }
        w.write_var_i32(self.item.block_runtime_id);
        let extra_data = if self.item.extra_data.is_empty() {
            minimal_item_extra_data(self.item.id)
        } else {
            self.item.extra_data.clone()
        };
        w.write_byte_array(&extra_data);
    }

    /// Encode as `NetworkItemStackDescriptor` — the **new** format introduced
    /// in protocol 975 (Bedrock 1.26.20), used ONLY by `InventorySlotPacket`
    /// and `MobEquipmentPacket`. `InventoryContentPacket`, `AddPlayer`,
    /// `AddItemActor` and inventory transactions still use the legacy
    /// [`Self::encode`].
    ///
    /// Port fidèle de PMMP `CommonTypes::putNetworkItemStackDescriptor`
    /// (`.reference/BedrockProtocol` tag `57.1.0+bedrock-1.26.20`) :
    /// `LE i16 id` + `LE u16 count` + `VarU32 meta` + `bool hasNetId`
    /// (+ `VarU32 variant` + `VarI32 stackId` si présent) + `VarU32
    /// blockRuntimeId` + `string rawExtraData` — plus de court-circuit sur
    /// id==0, blockRuntimeId et extraData TOUJOURS écrits.
    pub fn encode_descriptor(&self, w: &mut ProtoWriter) {
        w.write_i16_le(self.item.id as i16);
        w.write_u16_le(self.item.count);
        w.write_var_u32(self.item.meta);
        let stack_id = if self.item.is_air() { 0 } else { self.stack_id };
        let has_net_id = stack_id != 0;
        w.write_bool(has_net_id);
        if has_net_id {
            w.write_var_u32(0); // stackId variant (PMMP getStackIdVariant, 0 = défaut)
            w.write_var_i32(stack_id);
        }
        w.write_var_u32(self.item.block_runtime_id as u32);
        if self.item.is_air() {
            // PMMP ItemStack::null() → rawExtraData = "" (string vide).
            w.write_byte_array(&[]);
        } else {
            let extra_data = if self.item.extra_data.is_empty() {
                minimal_item_extra_data(self.item.id)
            } else {
                self.item.extra_data.clone()
            };
            w.write_byte_array(&extra_data);
        }
    }
}

fn minimal_item_extra_data(_item_id: i32) -> Vec<u8> {
    // Matches PMMP ItemStackExtraData::write() / gophertunnel 944 ItemInstance
    // for an item with no NBT and no can-place-on / can-destroy lists.
    // Bedrock expects fixed-width LE counts, NOT VarInts.
    //
    // Note: shields (protocol 944 = `minecraft:shield`, network id 387) carry
    // an additional Int64 "blocking tick" trailer. mc-rs does not currently
    // synthesize shield items, and the previous hardcoded `item_id == 355`
    // check was incorrect — id 355 is actually `minecraft:golden_shovel` on
    // protocol 944, so adding the trailer here corrupted that specific item.
    // When shield support lands, add a real registry-driven check.
    let mut extra = ProtoWriter::with_capacity(10);
    extra.write_i16_le(0); // NBT length (0 = no NBT compound tag)
    extra.write_i32_le(0); // canPlaceOn count
    extra.write_i32_le(0); // canDestroy count
    extra.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{decode_item_stack_wrapper, minimal_item_extra_data, ItemStack, ItemStackWrapper};
    use crate::io::{ProtoReader, ProtoWriter};

    #[test]
    fn item_stack_wrapper_writes_minimal_valid_extra_data_for_basic_items() {
        let stack = ItemStackWrapper::legacy(ItemStack::new(3, 1, 9853));
        let mut writer = ProtoWriter::new();
        stack.encode(&mut writer);

        let bytes = writer.into_bytes();
        let mut reader = ProtoReader::new(&bytes);
        let decoded = decode_item_stack_wrapper(&mut reader).expect("wrapper should decode");

        assert_eq!(decoded.item.id, 3);
        assert_eq!(decoded.item.count, 1);
        assert_eq!(decoded.item.block_runtime_id, 9853);
        assert_eq!(decoded.item.extra_data, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn minimal_item_extra_data_uses_fixed_width_counts() {
        let extra = minimal_item_extra_data(3);
        assert_eq!(extra.len(), 10);
        assert_eq!(extra[0..2], [0, 0]); // no NBT
        assert_eq!(extra[2..6], [0, 0, 0, 0]); // canPlace count
        assert_eq!(extra[6..10], [0, 0, 0, 0]); // canBreak count
    }
}

#[derive(Clone, Debug)]
pub struct FullContainerName {
    pub container_id: u8,
    pub dynamic_id: Option<u32>,
}

impl FullContainerName {
    pub fn new(container_id: u8) -> Self {
        Self {
            container_id,
            dynamic_id: None,
        }
    }

    pub fn encode(&self, w: &mut ProtoWriter) {
        w.write_u8(self.container_id);
        w.write_bool(self.dynamic_id.is_some());
        if let Some(dynamic_id) = self.dynamic_id {
            w.write_u32_le(dynamic_id);
        }
    }
}

// ── InventoryContent (S→C, 0x31) ──

pub struct InventoryContent;

impl InventoryContent {
    /// Encode an empty inventory for a window (all air items).
    pub fn encode_empty(
        window_id: u32,
        slot_count: u32,
        full_container_name: &FullContainerName,
    ) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_var_u32(window_id);
        w.write_var_u32(slot_count);
        for _ in 0..slot_count {
            w.write_var_i32(0); // air
        }
        full_container_name.encode(&mut w);
        ItemStackWrapper::air().encode(&mut w);
        w.into_bytes()
    }

    /// Encode inventory with actual items.
    pub fn encode_items(
        window_id: u32,
        items: &[ItemStackWrapper],
        full_container_name: &FullContainerName,
    ) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(256);
        w.write_var_u32(window_id);
        w.write_var_u32(items.len() as u32);
        for item in items {
            item.encode(&mut w);
        }
        full_container_name.encode(&mut w);
        ItemStackWrapper::air().encode(&mut w);
        w.into_bytes()
    }
}

// ── InventorySlot (S→C, 0x32) ──

pub struct InventorySlot;

impl InventorySlot {
    /// Encode a single slot update.
    pub fn encode(
        window_id: u32,
        slot: u32,
        item: &ItemStackWrapper,
        full_container_name: &FullContainerName,
    ) -> Vec<u8> {
        // Protocol 975 (PMMP `InventorySlotPacket::encodePayload`,
        // tag 57.1.0+bedrock-1.26.20) :
        //   VarU32 windowId + VarU32 inventorySlot
        //   writeOptional(containerName)  → bool + FullContainerName::write
        //   writeOptional(storage)        → bool (+ descriptor si présent)
        //   NetworkItemStackDescriptor item
        // Avant 975 : containerName et storage étaient écrits sans préfixe
        // Optional, et les items utilisaient l'ancien ItemStackWrapper.
        let mut w = ProtoWriter::with_capacity(64);
        w.write_var_u32(window_id);
        w.write_var_u32(slot);
        // containerName : toujours présent côté serveur → Optional(true).
        w.write_bool(true);
        full_container_name.encode(&mut w);
        // storage : non utilisé (slots non-bundle) → Optional(false).
        w.write_bool(false);
        item.encode_descriptor(&mut w);
        w.into_bytes()
    }
}

// ── MobEquipment (S→C, 0x1F) ──

pub struct MobEquipment;

impl MobEquipment {
    /// Encode empty hand (air item).
    ///
    /// Protocol 975 (PMMP `MobEquipmentPacket::encodePayload`,
    /// tag 57.1.0+bedrock-1.26.20) : `actorRuntimeId` + `NetworkItemStack
    /// Descriptor item` + `Byte inventorySlot` + `Byte hotbarSlot` + `Byte
    /// windowId`. L'item air n'est PLUS court-circuité (`VarI32 0`) : il
    /// faut un descriptor complet.
    pub fn encode_empty(runtime_entity_id: u64) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(16);
        w.write_var_u64(runtime_entity_id);
        ItemStackWrapper::air().encode_descriptor(&mut w);
        w.write_u8(0); // inventory_slot
        w.write_u8(0); // hotbar_slot
        w.write_u8(0); // container_id (inventory)
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
        item.encode_descriptor(&mut w);
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
