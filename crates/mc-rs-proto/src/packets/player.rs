use crate::io::reader::ProtoReadError;
use crate::io::{ProtoReader, ProtoWriter};

// ── Primitives Cereal (protocole 2168+) ──
//
// Depuis la conversion « Cereal » des paquets (changelog Mojang 2168), les
// champs optionnels sont précédés d'un booléen de présence. Les optionnels
// « doubles » de 2168 (booléen externe toujours vrai, puis présence et valeur)
// et les booléens de présence imposés à vrai d'InventoryTransaction ont
// disparu en 2193 (gophertunnel v1.62.0 retire `DoubleOptionalFunc`,
// CloudburstProtocol `PlayerAuthInputSerializer_v2193`,
// `InventoryTransactionSerializer_v2193`).

/// Taille maximale d'une liste générique (gophertunnel `maxSliceLength`).
const MAX_LIST_LENGTH: u32 = 1024;
/// Maximum d'actions par transaction, requête ou bloc d'actions (schémas
/// Mojang 2193 : `maxItems: 100`).
const MAX_ACTIONS: u32 = 100;
/// Nombre de drapeaux PlayerAuthInput : identifiants 0 à 65 (gophertunnel
/// `InputFlagCount`, 65 = InternalUpdate ajouté en 2168).
const INPUT_FLAG_COUNT: u32 = 66;

/// Lit un optionnel (booléen de présence, puis valeur).
fn read_optional<T>(
    reader: &mut ProtoReader,
    read: impl FnOnce(&mut ProtoReader) -> Result<T, ProtoReadError>,
) -> Result<Option<T>, ProtoReadError> {
    if !reader.read_bool()? {
        return Ok(None);
    }
    read(reader).map(Some)
}

/// Lit un nombre d'éléments et le borne au maximum du protocole.
fn read_count(
    reader: &mut ProtoReader,
    what: &'static str,
    max: u32,
) -> Result<u32, ProtoReadError> {
    let count = reader.read_var_u32()?;
    if count > max {
        return Err(ProtoReadError::LimitExceeded {
            what,
            value: u64::from(count),
            max: u64::from(max),
        });
    }
    Ok(count)
}

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
    /// 0 = CLICK_BLOCK, 1 = CLICK_AIR, 2 = BREAK_BLOCK, 3 = USE_AS_ATTACK
    /// (VarInt32 zigzag sur le fil).
    pub action_type: u32,
    pub block_position: [i32; 3],
    /// Face cliquée (u8 sur le fil ; 255 est rendu comme -1, la valeur
    /// « aucune face » des versions où le champ était un VarInt signé).
    pub face: i32,
    pub hotbar_slot: i32,
    /// Main utilisée (`HandSlot` : 0 = main principale, 1 = main secondaire ;
    /// ajouté en 2193).
    pub hand: u8,
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
    /// Échange complet de deux slots (pas de quantité sur le fil).
    Swap {
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
    /// Consommation d'un ingrédient de craft (PMMP
    /// `CraftingConsumeInputStackRequestAction`).
    Consume {
        count: u8,
        source: SlotInfo,
    },
    /// Choix du résultat à produire pour une recette à sorties multiples
    /// (PMMP `CraftingCreateSpecificResultStackRequestAction`).
    Create {
        results_slot: u8,
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
    /// list. Utilisé par le recipe book quand le joueur a coché
    /// "auto-craft from inventory".
    CraftRecipeAuto {
        recipe_id: u32,
        times: u8,
    },
    /// Résultats de craft annoncés par le client, sans usage côté serveur
    /// (PMMP `DeprecatedCraftingResultsStackRequestAction` : « no obvious use »).
    CraftResultsDeprecated,
    /// Actions décodées mais non gérées (identifiant legacy de l'action).
    Unknown(u8),
}

/// Identifiants legacy des actions ItemStackRequest (PMMP
/// `ItemStackRequestActionType`, gophertunnel `StackRequestAction*`).
mod stack_request_action_id {
    pub const TAKE: u8 = 0;
    pub const PLACE: u8 = 1;
    pub const SWAP: u8 = 2;
    pub const DROP: u8 = 3;
    pub const DESTROY: u8 = 4;
    pub const CONSUME: u8 = 5;
    pub const CREATE: u8 = 6;
    // 7 (PlaceInContainer) et 8 (TakeOutContainer) ne sont jamais envoyés et
    // n'ont pas de variante Cereal.
    pub const LAB_TABLE_COMBINE: u8 = 9;
    pub const BEACON_PAYMENT: u8 = 10;
    pub const MINE_BLOCK: u8 = 11;
    pub const CRAFT_RECIPE: u8 = 12;
    pub const CRAFT_RECIPE_AUTO: u8 = 13;
    pub const CRAFT_CREATIVE: u8 = 14;
    pub const CRAFT_RECIPE_OPTIONAL: u8 = 15;
    pub const CRAFT_GRINDSTONE: u8 = 16;
    pub const CRAFT_LOOM: u8 = 17;
    pub const CRAFT_NON_IMPLEMENTED: u8 = 18;
    pub const CRAFT_RESULTS_DEPRECATED: u8 = 19;
}

/// Index de variante Cereal → identifiant legacy : la liste des variantes
/// omet les deux actions conteneur (7 et 8), d'où le décalage de 2 au-delà
/// de CREATE (gophertunnel `stackRequestActionVariant`).
fn stack_request_action_from_variant(variant: u32) -> Result<u8, ProtoReadError> {
    match variant {
        0..=6 => Ok(variant as u8),
        7..=17 => Ok(variant as u8 + 2),
        _ => Err(ProtoReadError::InvalidValue {
            what: "stack request action variant",
            value: i64::from(variant),
        }),
    }
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
    /// Erreur rencontrée dans les blocs optionnels qui suivent le mouvement :
    /// le mouvement reste exploitable, le reste du paquet est abandonné.
    pub trailing_error: Option<String>,
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

/// Lit les drapeaux PlayerAuthInput : liste des identifiants de drapeaux
/// actifs (VarInt32 zigzag, 0 à 65), sans booléen de présence depuis 2193.
/// Réf : gophertunnel `input_flags.go` `InputFlagList`, CloudburstProtocol
/// `PlayerAuthInputSerializer_v2193` (`readInputData`).
fn read_input_flags(reader: &mut ProtoReader) -> Result<u128, ProtoReadError> {
    let count = read_count(reader, "player auth input flags", INPUT_FLAG_COUNT)?;
    let mut flags: u128 = 0;
    for _ in 0..count {
        let id = reader.read_var_i32()?;
        if !(0..INPUT_FLAG_COUNT as i32).contains(&id) {
            return Err(ProtoReadError::InvalidValue {
                what: "player auth input flag",
                value: i64::from(id),
            });
        }
        flags |= 1u128 << id;
    }
    Ok(flags)
}

/// Décode un `NetworkItemStackDescriptor` (protocole 2168+) : `i16` id,
/// `u16` count, VarUInt32 aux, net id optionnel (booléen + VarInt32 unique,
/// plus d'index de variante), VarUInt32 block runtime id, user data.
/// Réf : gophertunnel `reader.go` `ItemInstance`, schéma Mojang
/// `cerealizer_NetworkItemStackDescriptor___SerializedData.json`.
pub fn decode_network_item_stack_descriptor(
    reader: &mut ProtoReader,
) -> Result<ItemStackWrapper, ProtoReadError> {
    let id = i32::from(reader.read_i16_le()?);
    let count = reader.read_u16_le()?;
    let meta = reader.read_var_u32()?;
    let stack_id = if reader.read_bool()? {
        reader.read_var_i32()?
    } else {
        0
    };
    let block_runtime_id = reader.read_var_u32()? as i32;
    let extra_data = reader.read_byte_array()?;
    if id == 0 {
        return Ok(ItemStackWrapper::air());
    }
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

/// Decode a NetworkInventoryAction : source, identifiant de fenêtre et
/// drapeaux optionnels, puis deux descripteurs d'item.
/// Réf : gophertunnel `inventory.go` `InventoryAction.Marshal`,
/// CloudburstProtocol `BedrockCodecHelper_v2193::readSource`.
fn decode_network_inventory_action(
    reader: &mut ProtoReader,
) -> Result<NetworkInventoryAction, ProtoReadError> {
    let source_type = reader.read_var_u32()?;
    let window_id = read_optional(reader, |r| r.read_i8())?.map(i32::from);
    let source_flags = read_optional(reader, |r| r.read_var_u32())?;
    let inventory_slot = reader.read_var_u32()?;
    let old_item = decode_network_item_stack_descriptor(reader)?;
    let new_item = decode_network_item_stack_descriptor(reader)?;

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
) -> Result<Vec<NetworkInventoryAction>, ProtoReadError> {
    let action_count = read_count(reader, "inventory actions", MAX_ACTIONS)?;
    let mut actions = Vec::with_capacity(action_count as usize);
    for _ in 0..action_count {
        actions.push(decode_network_inventory_action(reader)?);
    }
    Ok(actions)
}

/// Liste `LegacySetSlot` : conteneur (u8) + slots (tableau d'octets).
fn decode_changed_slots_hack(
    reader: &mut ProtoReader,
) -> Result<Vec<InventoryTransactionChangedSlots>, ProtoReadError> {
    let changed_slots_count = read_count(reader, "legacy set item slots", MAX_LIST_LENGTH)?;
    let mut changed_slots = Vec::with_capacity(changed_slots_count as usize);
    for _ in 0..changed_slots_count {
        let container_id = reader.read_u8()?;
        let slots = reader.read_byte_array()?;
        changed_slots.push(InventoryTransactionChangedSlots {
            container_id,
            changed_slots: slots,
        });
    }
    Ok(changed_slots)
}

/// Données d'une transaction UseItem (partagées entre InventoryTransaction et
/// PlayerAuthInput). Réf : gophertunnel `inventory.go`
/// `UseItemTransactionData.Marshal`, schéma `ItemUseInventoryTransaction.json`
/// (main utilisée après le slot de barre rapide depuis 2193).
fn decode_use_item_transaction_data(
    reader: &mut ProtoReader,
) -> Result<ItemInteractionData, ProtoReadError> {
    let action_type = reader.read_var_i32()? as u32;
    let _trigger_type = reader.read_u8()?;

    // Bedrock `BlockPosition` = 3 VarInt SIGNED (PMMP CommonTypes::getBlockPosition
    // + gophertunnel BlockPos).
    let bx = reader.read_var_i32()?;
    let by = reader.read_var_i32()?;
    let bz = reader.read_var_i32()?;

    let face = i32::from(reader.read_u8()? as i8);
    let hotbar_slot = reader.read_var_i32()?;
    let hand = reader.read_u8()?;

    let _held_item = decode_network_item_stack_descriptor(reader)?;

    let player_position = read_vec3(reader)?;
    let click_position = read_vec3(reader)?;

    let block_runtime_id = reader.read_var_u32()?;
    let client_prediction = u32::from(reader.read_u8()?);
    let _client_cooldown_state = reader.read_u8()?;

    Ok(ItemInteractionData {
        action_type,
        block_position: [bx, by, bz],
        face,
        hotbar_slot,
        hand,
        player_position,
        click_position,
        block_runtime_id,
        client_prediction,
    })
}

fn read_vec3(reader: &mut ProtoReader) -> Result<[f32; 3], ProtoReadError> {
    Ok([
        reader.read_f32_le()?,
        reader.read_f32_le()?,
        reader.read_f32_le()?,
    ])
}

fn decode_use_item_on_entity_transaction_data(
    reader: &mut ProtoReader,
    actions: Vec<NetworkInventoryAction>,
) -> Result<InventoryTransactionData, ProtoReadError> {
    let actor_runtime_id = reader.read_var_u64()?;
    let action_type = reader.read_var_i32()? as u32;
    let hotbar_slot = reader.read_var_i32()?;
    let item_in_hand = decode_network_item_stack_descriptor(reader)?;
    let player_position = read_vec3(reader)?;
    let click_position = read_vec3(reader)?;
    Ok(InventoryTransactionData::UseItemOnEntity {
        actions,
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
    actions: Vec<NetworkInventoryAction>,
) -> Result<InventoryTransactionData, ProtoReadError> {
    let action_type = reader.read_var_i32()? as u32;
    let hotbar_slot = reader.read_var_i32()?;
    let item_in_hand = decode_network_item_stack_descriptor(reader)?;
    let head_position = read_vec3(reader)?;
    Ok(InventoryTransactionData::ReleaseItem {
        actions,
        action_type,
        hotbar_slot,
        item_in_hand,
        head_position,
    })
}

impl InventoryTransaction {
    /// Réf : gophertunnel `packet/inventory_transaction.go`, PMMP
    /// `InventoryTransactionPacket::decodePayload`, CloudburstProtocol
    /// `InventoryTransactionSerializer_v2193` : depuis 2193, le type et les
    /// actions ne sont plus précédés d'un booléen de présence.
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, ProtoReadError> {
        let request_id = reader.read_var_i32()?;
        let changed_slots = if reader.read_bool()? {
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
            3 => decode_use_item_on_entity_transaction_data(reader, actions)?,
            4 => decode_release_item_transaction_data(reader, actions)?,
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

/// Transaction UseItem « empaquetée » de PlayerAuthInput
/// (`PackedItemUseLegacyInventoryTransaction`, gophertunnel `reader.go`
/// `PlayerInventoryAction`) : les actions sont une liste simple depuis 2193.
fn decode_item_interaction(
    reader: &mut ProtoReader,
) -> Result<ItemInteractionData, ProtoReadError> {
    let _legacy_request_id = reader.read_var_i32()?;
    if reader.read_bool()? {
        let _ = decode_changed_slots_hack(reader)?;
    }
    let _ = decode_inventory_actions(reader)?;
    decode_use_item_transaction_data(reader)
}

/// Read a SlotInfo : FullContainerName (u8 + u32 optionnel), slot, puis net id
/// en `i32` LE fixe (gophertunnel `item_stack.go` `StackReqSlotInfo`,
/// CloudburstProtocol `BedrockCodecHelper_v2168`).
fn read_slot_info(reader: &mut ProtoReader) -> Result<SlotInfo, ProtoReadError> {
    let container_id = reader.read_u8()?;
    if reader.read_bool()? {
        let _dynamic_id = reader.read_u32_le()?;
    }
    let slot_id = reader.read_u8()?;
    let stack_id = reader.read_i32_le()?;
    Ok(SlotInfo {
        container_id,
        slot_id,
        stack_id,
    })
}

/// Ingrédient d'une action CraftRecipeAuto : variante (0 vide, 1 nom,
/// 2 Molang, 3 tag), type legacy (même valeur), corps, puis `u16` quantité.
/// Réf : gophertunnel `item_descriptor.go` `StackRequestItemDescriptorCount`.
fn skip_stack_request_ingredient(reader: &mut ProtoReader) -> Result<(), ProtoReadError> {
    let variant = reader.read_var_u32()?;
    let _descriptor_type = reader.read_u8()?;
    match variant {
        0 => {}
        1 => {
            let _name = reader.read_string()?;
            let _aux = reader.read_var_i32()?;
        }
        2 => {
            let _expression = reader.read_string()?;
            let _version = reader.read_i16_le()?;
        }
        3 => {
            let _tag = reader.read_string()?;
        }
        other => {
            return Err(ProtoReadError::InvalidValue {
                what: "stack request ingredient descriptor",
                value: i64::from(other),
            })
        }
    }
    let _count = reader.read_u16_le()?;
    Ok(())
}

/// Item d'une action CraftResultsDeprecated : descripteur par nom (variante
/// 0 vide ou 1 nom + aux), `i16` count, VarUInt32 block runtime id, user data.
/// Réf : gophertunnel `reader.go` `StackRequestItem`.
fn skip_stack_request_item(reader: &mut ProtoReader) -> Result<(), ProtoReadError> {
    let variant = reader.read_var_u32()?;
    let _legacy_variant = reader.read_u8()?;
    match variant {
        0 => {}
        1 => {
            let _identifier = reader.read_string()?;
            let _aux = reader.read_var_i32()?;
        }
        other => {
            return Err(ProtoReadError::InvalidValue {
                what: "stack request item descriptor",
                value: i64::from(other),
            })
        }
    }
    let _count = reader.read_i16_le()?;
    let _block_runtime_id = reader.read_var_u32()?;
    let _user_data = reader.read_byte_array()?;
    Ok(())
}

/// Décode une action : index de variante Cereal (VarUInt32), identifiant
/// legacy (u8, ignoré comme chez gophertunnel et CloudburstProtocol), corps.
/// Tous les corps sont lus, y compris ceux des actions non gérées, pour que
/// les actions suivantes et les chaînes de filtre restent alignées.
fn decode_stack_request_action(
    reader: &mut ProtoReader,
) -> Result<StackRequestAction, ProtoReadError> {
    use stack_request_action_id as id;

    let action_id = stack_request_action_from_variant(reader.read_var_u32()?)?;
    let _legacy_id = reader.read_u8()?;
    let action = match action_id {
        id::TAKE | id::PLACE => {
            let count = reader.read_u8()?;
            let source = read_slot_info(reader)?;
            let destination = read_slot_info(reader)?;
            if action_id == id::TAKE {
                StackRequestAction::Take {
                    count,
                    source,
                    destination,
                }
            } else {
                StackRequestAction::Place {
                    count,
                    source,
                    destination,
                }
            }
        }
        id::SWAP => StackRequestAction::Swap {
            source: read_slot_info(reader)?,
            destination: read_slot_info(reader)?,
        },
        id::DROP => {
            let count = reader.read_u8()?;
            let source = read_slot_info(reader)?;
            let _randomly = reader.read_bool()?;
            StackRequestAction::Drop { count, source }
        }
        id::DESTROY => StackRequestAction::Destroy {
            count: reader.read_u8()?,
            source: read_slot_info(reader)?,
        },
        id::CONSUME => StackRequestAction::Consume {
            count: reader.read_u8()?,
            source: read_slot_info(reader)?,
        },
        id::CREATE => StackRequestAction::Create {
            results_slot: reader.read_u8()?,
        },
        id::LAB_TABLE_COMBINE => StackRequestAction::Unknown(action_id),
        id::BEACON_PAYMENT => {
            let _primary_effect = reader.read_var_i32()?;
            let _secondary_effect = reader.read_var_i32()?;
            StackRequestAction::Unknown(action_id)
        }
        id::MINE_BLOCK => StackRequestAction::MineBlock {
            hotbar_slot: reader.read_var_i32()?,
            predicted_durability: reader.read_var_i32()?,
            network_stack_id: reader.read_i32_le()?,
        },
        id::CRAFT_RECIPE => StackRequestAction::CraftRecipe {
            recipe_id: reader.read_var_u32()?,
            times: reader.read_u8()?,
        },
        id::CRAFT_RECIPE_AUTO => {
            let recipe_id = reader.read_var_u32()?;
            let times = reader.read_u8()?;
            let ingredient_count = read_count(reader, "auto craft ingredients", MAX_LIST_LENGTH)?;
            for _ in 0..ingredient_count {
                skip_stack_request_ingredient(reader)?;
            }
            StackRequestAction::CraftRecipeAuto { recipe_id, times }
        }
        id::CRAFT_CREATIVE => {
            let creative_item_network_id = reader.read_var_u32()?;
            let _repetitions = reader.read_u8()?;
            StackRequestAction::CraftCreative {
                creative_item_network_id,
            }
        }
        id::CRAFT_RECIPE_OPTIONAL => {
            let _recipe_id = reader.read_var_u32()?;
            let _filter_string_index = reader.read_i32_le()?;
            StackRequestAction::Unknown(action_id)
        }
        id::CRAFT_GRINDSTONE => {
            let _recipe_net_id = reader.read_i32_le()?;
            let _repetitions = reader.read_u8()?;
            let _cost = reader.read_var_i32()?;
            StackRequestAction::Unknown(action_id)
        }
        id::CRAFT_LOOM => {
            let _pattern = reader.read_string()?;
            let _repetitions = reader.read_u8()?;
            StackRequestAction::Unknown(action_id)
        }
        id::CRAFT_NON_IMPLEMENTED => StackRequestAction::Unknown(action_id),
        id::CRAFT_RESULTS_DEPRECATED => {
            let item_count = read_count(reader, "craft results", MAX_LIST_LENGTH)?;
            for _ in 0..item_count {
                skip_stack_request_item(reader)?;
            }
            let _times_crafted = reader.read_u8()?;
            StackRequestAction::CraftResultsDeprecated
        }
        other => StackRequestAction::Unknown(other),
    };
    Ok(action)
}

/// Decode an ItemStackRequest. Réf : gophertunnel `item_stack.go`
/// `ItemStackRequest.Marshal`, schéma `ItemStackRequest.json`.
fn decode_item_stack_request(reader: &mut ProtoReader) -> Result<ItemStackRequest, ProtoReadError> {
    let request_id = reader.read_var_i32()?;
    let action_count = read_count(reader, "stack request actions", MAX_ACTIONS)?;
    let mut actions = Vec::with_capacity(action_count as usize);
    for _ in 0..action_count {
        actions.push(decode_stack_request_action(reader)?);
    }

    let filter_count = read_count(reader, "stack request filter strings", MAX_LIST_LENGTH)?;
    for _ in 0..filter_count {
        let _s = reader.read_string()?;
    }
    let _filter_cause = reader.read_i32_le()?;

    Ok(ItemStackRequest {
        request_id,
        actions,
    })
}

/// Actions de bloc : position et face toujours présentes depuis 2168
/// (gophertunnel `player.go` `PlayerBlockAction.Marshal`).
fn decode_block_actions(reader: &mut ProtoReader) -> Result<Vec<BlockAction>, ProtoReadError> {
    let count = read_count(reader, "player block actions", MAX_ACTIONS)?;
    let mut block_actions = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let action_type = reader.read_var_i32()?;
        let bx = reader.read_var_i32()?;
        let by = reader.read_var_i32()?;
        let bz = reader.read_var_i32()?;
        let face = reader.read_var_i32()?;
        block_actions.push(BlockAction {
            action_type,
            position: [bx, by, bz],
            face,
        });
    }
    Ok(block_actions)
}

/// Blocs optionnels qui suivent le mouvement dans PlayerAuthInput.
struct PlayerAuthInputTail {
    item_interaction: Option<ItemInteractionData>,
    item_stack_request: Option<ItemStackRequest>,
    block_actions: Vec<BlockAction>,
}

/// Ordre 2193 (gophertunnel `packet/player_auth_input.go` `Marshal`,
/// CloudburstProtocol `PlayerAuthInputSerializer_v2193`) : cinq optionnels
/// simples (transaction UseItem, ItemStackRequest, actions de bloc, rotation
/// de véhicule, véhicule prédit), puis vecteurs analogique, orientation
/// caméra et mouvement brut (ignorés). La présence est portée par les
/// booléens, plus par les drapeaux.
fn decode_player_auth_input_tail(
    reader: &mut ProtoReader,
) -> Result<PlayerAuthInputTail, ProtoReadError> {
    let item_interaction = read_optional(reader, decode_item_interaction)?;
    let item_stack_request = read_optional(reader, decode_item_stack_request)?;
    let block_actions = read_optional(reader, decode_block_actions)?.unwrap_or_default();
    let _vehicle_rotation = read_optional(reader, |r| Ok((r.read_f32_le()?, r.read_f32_le()?)))?;
    let _client_predicted_vehicle = read_optional(reader, |r| r.read_var_i64())?;
    Ok(PlayerAuthInputTail {
        item_interaction,
        item_stack_request,
        block_actions,
    })
}

impl PlayerAuthInput {
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, ProtoReadError> {
        let pitch = reader.read_f32_le()?;
        let yaw = reader.read_f32_le()?;
        let pos_x = reader.read_f32_le()?;
        let pos_y = reader.read_f32_le()?;
        let pos_z = reader.read_f32_le()?;
        let move_vec_x = reader.read_f32_le()?;
        let move_vec_z = reader.read_f32_le()?;
        let head_yaw = reader.read_f32_le()?;

        let input_flags = read_input_flags(reader)?;

        let _input_mode = reader.read_var_u32()?;
        let _play_mode = reader.read_var_u32()?;
        let _interaction_model = reader.read_var_i32()?;
        let _interact_rot_x = reader.read_f32_le()?;
        let _interact_rot_y = reader.read_f32_le()?;
        let _tick = reader.read_var_u64()?;
        let _delta = read_vec3(reader)?;

        let (tail, trailing_error) = match decode_player_auth_input_tail(reader) {
            Ok(tail) => (tail, None),
            Err(e) => (
                PlayerAuthInputTail {
                    item_interaction: None,
                    item_stack_request: None,
                    block_actions: Vec::new(),
                },
                Some(e.to_string()),
            ),
        };

        Ok(Self {
            pitch,
            yaw,
            position: [pos_x, pos_y, pos_z],
            move_vec_x,
            move_vec_z,
            head_yaw,
            input_flags,
            block_actions: tail.block_actions,
            item_interaction: tail.item_interaction,
            item_stack_request: tail.item_stack_request,
            trailing_error,
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
        // Protocole 2168 : données de téléportation optionnelles (booléen de
        // présence), présentes ssi mode == TELEPORT (gophertunnel
        // `move_player.go` `OptionalMarshaler(TeleportData)`, CloudburstProtocol
        // `MovePlayerSerializer_v2168`).
        if self.mode == 2 {
            w.write_bool(true);
            w.write_i32_le(0); // cause = unknown
            w.write_i32_le(0); // source entity type
        } else {
            w.write_bool(false);
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
        // Protocole 2168 : GatheringsConfiguration optionnelle, absente
        // (gophertunnel `transfer.go`, CloudburstProtocol `TransferSerializer_v2168`).
        w.write_bool(false);
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
    /// Hash d'apparence calculé par le client (claim `ProfileHash` du login,
    /// protocole 2168), clé de cache des images de profil.
    pub profile_hash: String,
}

impl Default for SerializedSkin {
    fn default() -> Self {
        Self {
            skin_id: "Custom".to_string(),
            play_fab_id: String::new(),
            skin_resource_patch: "{\"geometry\":{\"default\":\"geometry.humanoid.custom\"}}"
                .to_string(),
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
            profile_hash: String::new(),
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

impl PlayerListAdd {
    /// `BuildPlatform::Unknown`. Le champ est une énumération « Enum-as-Value »
    /// dont 0 ne fait pas partie (valeurs 1 à 15 et -1). Réf : schéma
    /// `BuildPlatform.json`, PMMP `PlayerListEntry::createAdditionEntry`,
    /// dragonfly `session_list.go` (`protocol.DeviceUnknown`).
    pub const BUILD_PLATFORM_UNKNOWN: i32 = -1;
}

impl PlayerList {
    pub const ACTION_ADD: u8 = 0;
    pub const ACTION_REMOVE: u8 = 1;

    /// Protocole 2168 : plus d'octet d'action global ni de liste finale de
    /// booléens « skin vérifiée ». Chaque entrée est une variante taguée :
    /// index de variante (VarUInt32, Remove = 0, Add = 1) puis octet d'action
    /// legacy (Add = 0, Remove = 1), les deux numérotations sont inversées.
    /// Réf : gophertunnel `player.go` `playerListAction`, CloudburstProtocol
    /// `PlayerListSerializer_v2168`, schéma `PlayerListPacketPayload.json`.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(256);
        w.write_var_u32(self.entries.len() as u32);
        for entry in &self.entries {
            if self.action == Self::ACTION_ADD {
                w.write_var_u32(1); // variante Add
                w.write_u8(Self::ACTION_ADD);
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
                // Couleur du joueur, ARGB (blanc opaque).
                w.write_u32_le(0xFFFF_FFFF);
            } else {
                w.write_var_u32(0); // variante Remove
                w.write_u8(Self::ACTION_REMOVE);
                w.write_raw(&entry.uuid);
            }
        }
        w.into_bytes()
    }
}

/// Couleur de peau du claim de login (`"#rrggbb"` ou `"#aarrggbb"`) vers
/// l'entier ARGB du protocole 2168. CloudburstProtocol écrit
/// `Color.getRGB()` (0xAARRGGBB) en little-endian ; toute valeur illisible
/// (le défaut Steve `"#0"`) donne 0.
fn skin_color_argb(color: &str) -> i32 {
    let hex = color.strip_prefix('#').unwrap_or(color);
    let value = match hex.len() {
        6 => u32::from_str_radix(hex, 16).map(|rgb| 0xFF00_0000 | rgb),
        8 => u32::from_str_radix(hex, 16),
        _ => return 0,
    };
    value.map(|argb| argb as i32).unwrap_or(0)
}

/// SerializedSkin, protocole 2169 : compteurs d'animations, de pièces persona
/// et de teintes en VarUInt32, taille de bras en u8, couleur en i32 ARGB, puis
/// drapeau de confiance (chaîne `"true"`/`"false"`, déplacé depuis PlayerList)
/// et hash de profil. Réf : schéma `SerializedSkin.json` (ordinaux 0 à 21),
/// CloudburstProtocol `BedrockCodecHelper_v2168` `writeSkin`, gophertunnel
/// `skin.go`.
fn write_serialized_skin(w: &mut ProtoWriter, skin: &SerializedSkin) {
    w.write_string(&skin.skin_id);
    w.write_string(&skin.play_fab_id);
    w.write_string(&skin.skin_resource_patch);
    // Skin image data
    w.write_u32_le(skin.skin_width);
    w.write_u32_le(skin.skin_height);
    w.write_byte_array(&skin.skin_data);

    // Animations : aucune.
    w.write_var_u32(0);

    // Cape image
    w.write_u32_le(skin.cape_width);
    w.write_u32_le(skin.cape_height);
    w.write_byte_array(&skin.cape_data);

    // GeometryData est un `Json::Value` : le client rejette le paquet si la
    // chaîne n'est pas du JSON valide, et la chaîne vide n'en est pas.
    // dragonfly `skinToProtocol` envoie `{}` dans ce cas, et la version
    // courante du jeu comme version moteur de la géométrie.
    w.write_string(if skin.geometry_data.is_empty() {
        "{}"
    } else {
        &skin.geometry_data
    });
    w.write_string(if skin.geometry_data_engine_version.is_empty() {
        crate::MINECRAFT_VERSION
    } else {
        &skin.geometry_data_engine_version
    });
    w.write_string(&skin.animation_data);

    w.write_string(&skin.cape_id);
    w.write_string(&skin.full_skin_id);
    // persona::ArmSize : 0 = Slim, 1 = Wide.
    w.write_u8(if skin.arm_size.eq_ignore_ascii_case("slim") {
        0
    } else {
        1
    });
    w.write_i32_le(skin_color_argb(&skin.skin_color));

    // Pièces persona et teintes : aucune.
    w.write_var_u32(0);
    w.write_var_u32(0);

    w.write_bool(skin.premium);
    w.write_bool(skin.persona);
    w.write_bool(skin.persona_cape_on_classic);
    w.write_bool(skin.primary_user);
    w.write_bool(false); // overrides player appearance

    w.write_string("true"); // TrustedSkinFlag
    w.write_string(&skin.profile_hash);
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
    /// Réf : schéma `AddPlayerPacketPayload.json`, gophertunnel
    /// `packet/add_player.go`, PMMP `AddPlayerPacket` + `Human::sendSpawnPacket`.
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
        // Item tenu : NetworkItemStackDescriptor depuis 2168, sans net id
        // (changelog Mojang 2168, `getStrippedNetworkItem`).
        ItemStackWrapper::air().encode_descriptor(&mut w);
        w.write_var_i32(self.gamemode);
        // Métadonnées (mêmes clés que SetActorData : PMMP
        // `EntityMetadataProperties` SCALE = 38, BOUNDING_BOX_* = 53/54).
        write_actor_metadata(
            &mut w,
            &[
                (0, 7, MetadataValue::Long(0)),
                (4, 4, MetadataValue::String(self.username.clone())),
                (38, 3, MetadataValue::Float(1.0)),
                (53, 3, MetadataValue::Float(0.6)),
                (54, 3, MetadataValue::Float(1.8)),
            ],
        );
        // PropertySyncData (entiers, flottants) : vide.
        w.write_var_u32(0);
        w.write_var_u32(0);
        // AbilitiesData : identifiant, permissions joueur puis commandes.
        w.write_i64_le(self.entity_unique_id);
        w.write_u8(self.permission_level);
        w.write_u8(self.command_permission);
        // Une couche de base, capacités toutes définies et fausses, vitesses
        // nulles, comme PMMP `Human::sendSpawnPacket`.
        w.write_var_u32(1);
        w.write_u16_le(1); // layer type = Base
        w.write_u32_le(ability::ALL);
        w.write_u32_le(0);
        w.write_f32_le(0.0); // fly speed
        w.write_f32_le(0.0); // vertical fly speed
        w.write_f32_le(0.0); // walk speed
                             // Entity links (none)
        w.write_var_u32(0);
        // Device ID + OS
        w.write_string(""); // device_id
                            // Build Platform : même énumération que dans PlayerList.
        w.write_i32_le(PlayerListAdd::BUILD_PLATFORM_UNKNOWN);
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
        // Protocole 2168 : NetworkItemStackDescriptor (schéma
        // `AddItemActorPacketPayload.json`).
        self.item.encode_descriptor(&mut w);
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

// ── UpdateAbilities (S→C, 0xBB) ──

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

        // Nombre de couches en VarUInt32 (gophertunnel `ability.go` `Slice`).
        w.write_var_u32(self.layers.len() as u32);
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
        // Protocole 2168 : index de variante Cereal puis champ `Type` de la
        // charge utile, même valeur sur un octet (gophertunnel `writer.go`
        // `EntityMetadata`, CloudburstProtocol `BedrockCodecHelper_v2168`).
        w.write_var_u32(*data_type);
        w.write_u8(*data_type as u8);
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
    pub const BABY: i64 = 1 << 11; // PMMP EntityMetadataFlags::BABY = 11
    pub const SHEARED: i64 = 1 << 31; // PMMP EntityMetadataFlags::SHEARED = 31
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

    /// Encode as `NetworkItemStackDescriptor` (protocole 2168+) : `i16` id,
    /// `u16` count, VarUInt32 aux, net id optionnel (booléen + VarInt32, plus
    /// d'index de variante), VarUInt32 block runtime id, user data. L'air est
    /// entièrement à zéro (« Send fixed Id of 0 for invalid item »).
    /// Réf : gophertunnel `writer.go` `ItemInstance`, CloudburstProtocol
    /// `BedrockCodecHelper_v2168` `writeNetworkItemStackDescriptor`.
    pub fn encode_descriptor(&self, w: &mut ProtoWriter) {
        if self.item.is_air() {
            w.write_i16_le(0);
            w.write_u16_le(0);
            w.write_var_u32(0);
            w.write_bool(false);
            w.write_var_u32(0);
            w.write_byte_array(&[]);
            return;
        }
        w.write_i16_le(self.item.id as i16);
        w.write_u16_le(self.item.count);
        w.write_var_u32(self.item.meta);
        w.write_bool(self.stack_id != 0);
        if self.stack_id != 0 {
            w.write_var_i32(self.stack_id);
        }
        w.write_var_u32(self.item.block_runtime_id as u32);
        if self.item.extra_data.is_empty() {
            w.write_byte_array(&minimal_item_extra_data(self.item.id));
        } else {
            w.write_byte_array(&self.item.extra_data);
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
    use super::*;

    #[test]
    fn item_descriptor_round_trips_without_variant_index() {
        let stack = ItemStackWrapper::new(ItemStack::new(3, 1, 9853), 7);
        let mut writer = ProtoWriter::new();
        stack.encode_descriptor(&mut writer);
        let bytes = writer.into_bytes();
        // i16 id, u16 count, aux 0, présence, net id 7 (zigzag 14), sans
        // index de variante.
        assert_eq!(&bytes[..7], &[3, 0, 1, 0, 0, 1, 14]);

        let mut reader = ProtoReader::new(&bytes);
        let decoded = decode_network_item_stack_descriptor(&mut reader).unwrap();
        assert_eq!(decoded.item.id, 3);
        assert_eq!(decoded.item.count, 1);
        assert_eq!(decoded.stack_id, 7);
        assert_eq!(decoded.item.block_runtime_id, 9853);
        assert_eq!(decoded.item.extra_data, vec![0; 10]);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn air_descriptor_is_all_zero() {
        let mut writer = ProtoWriter::new();
        ItemStackWrapper::air().encode_descriptor(&mut writer);
        assert_eq!(writer.into_bytes(), vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn minimal_item_extra_data_uses_fixed_width_counts() {
        let extra = minimal_item_extra_data(3);
        assert_eq!(extra.len(), 10);
        assert_eq!(extra[0..2], [0, 0]); // no NBT
        assert_eq!(extra[2..6], [0, 0, 0, 0]); // canPlace count
        assert_eq!(extra[6..10], [0, 0, 0, 0]); // canBreak count
    }

    #[test]
    fn metadata_entry_repeats_type_byte() {
        let mut writer = ProtoWriter::new();
        write_actor_metadata(&mut writer, &[(38, 3, MetadataValue::Float(1.0))]);
        assert_eq!(
            writer.into_bytes(),
            vec![1, 38, 3, 3, 0x00, 0x00, 0x80, 0x3F]
        );
    }

    #[test]
    fn skin_colour_parses_hex_claims() {
        assert_eq!(skin_color_argb("#b37b62"), 0xFFB3_7B62_u32 as i32);
        assert_eq!(skin_color_argb("#80b37b62"), 0x80B3_7B62_u32 as i32);
        assert_eq!(skin_color_argb("#0"), 0);
    }

    #[test]
    fn player_list_add_sends_valid_json_geometry_and_known_platform() {
        let entry = PlayerListAdd {
            uuid: [9; 16],
            entity_id: 1,
            username: "a".into(),
            xuid: String::new(),
            platform_chat_id: String::new(),
            build_platform: PlayerListAdd::BUILD_PLATFORM_UNKNOWN,
            skin: SerializedSkin::default(),
            is_teacher: false,
            is_host: false,
            is_subclient: false,
        };
        let bytes = PlayerList {
            action: PlayerList::ACTION_ADD,
            entries: vec![entry],
        }
        .encode();
        let contains = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
        // BuildPlatform -1 en i32 little-endian, juste après les trois chaînes.
        assert!(contains(&[0x01, b'a', 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]));
        // GeometryData `{}` puis version moteur, chaînes préfixées par leur longueur.
        let mut geometry = vec![2, b'{', b'}', crate::MINECRAFT_VERSION.len() as u8];
        geometry.extend_from_slice(crate::MINECRAFT_VERSION.as_bytes());
        assert!(contains(&geometry));
    }

    #[test]
    fn player_list_remove_uses_tagged_entries() {
        let entry = PlayerListAdd {
            uuid: [9; 16],
            entity_id: 1,
            username: String::new(),
            xuid: String::new(),
            platform_chat_id: String::new(),
            build_platform: PlayerListAdd::BUILD_PLATFORM_UNKNOWN,
            skin: SerializedSkin::default(),
            is_teacher: false,
            is_host: false,
            is_subclient: false,
        };
        let bytes = PlayerList {
            action: PlayerList::ACTION_REMOVE,
            entries: vec![entry],
        }
        .encode();
        let mut expected = vec![1, 0, 1];
        expected.extend([9; 16]);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn inventory_transaction_normal_decodes_2193_layout() {
        let mut w = ProtoWriter::new();
        w.write_var_i32(0); // legacy request id
        w.write_bool(false); // pas de legacy slots
        w.write_var_u32(0); // Normal
        w.write_var_u32(1);
        w.write_var_u32(0); // source Container
        w.write_bool(true);
        w.write_i8(-2); // window id
        w.write_bool(false); // pas de drapeaux
        w.write_var_u32(4); // slot
        ItemStackWrapper::air().encode_descriptor(&mut w);
        ItemStackWrapper::new(ItemStack::new(3, 2, 0), 5).encode_descriptor(&mut w);
        let bytes = w.into_bytes();

        let tx = InventoryTransaction::decode(&mut ProtoReader::new(&bytes)).unwrap();
        let InventoryTransactionData::Normal { actions } = tx.data else {
            panic!("expected Normal");
        };
        assert_eq!(actions[0].window_id, Some(-2));
        assert_eq!(actions[0].source_flags, None);
        assert_eq!(actions[0].inventory_slot, 4);
        assert_eq!(actions[0].new_item.item.count, 2);
    }

    #[test]
    fn player_auth_input_reads_flag_list_and_optionals() {
        let mut w = ProtoWriter::new();
        for _ in 0..8 {
            w.write_f32_le(0.0); // rotation, position, move vector, head yaw
        }
        w.write_var_u32(2);
        w.write_var_i32(25); // START_SPRINTING
        w.write_var_i32(65); // InternalUpdate
        w.write_var_u32(0); // input mode
        w.write_var_u32(0); // play mode
        w.write_var_i32(0); // interaction model
        w.write_f32_le(0.0);
        w.write_f32_le(0.0);
        w.write_var_u64(1); // tick
        for _ in 0..3 {
            w.write_f32_le(0.0); // delta
        }
        w.write_bool(false); // pas de transaction UseItem
        w.write_bool(false); // pas d'ItemStackRequest
        w.write_bool(true); // actions de bloc
        w.write_var_u32(1);
        w.write_var_i32(18); // PREDICT_DESTROY_BLOCK
        w.write_var_i32(1);
        w.write_var_i32(64);
        w.write_var_i32(-3);
        w.write_var_i32(1);
        for _ in 0..2 {
            w.write_bool(false); // véhicule
        }
        let bytes = w.into_bytes();

        let pai = PlayerAuthInput::decode(&mut ProtoReader::new(&bytes)).unwrap();
        assert_eq!(pai.trailing_error, None);
        assert_eq!(pai.input_flags, (1u128 << 25) | (1u128 << 65));
        assert_eq!(pai.block_actions.len(), 1);
        assert_eq!(pai.block_actions[0].position, [1, 64, -3]);
    }

    #[test]
    fn stack_request_maps_variant_indices() {
        assert_eq!(stack_request_action_from_variant(6).unwrap(), 6);
        assert_eq!(stack_request_action_from_variant(7).unwrap(), 9);
        assert_eq!(stack_request_action_from_variant(17).unwrap(), 19);
        assert!(stack_request_action_from_variant(18).is_err());
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
        let air = vec![ItemStackWrapper::air(); slot_count as usize];
        Self::encode_items(window_id, &air, full_container_name)
    }

    /// Encode inventory with actual items.
    ///
    /// Layout (PMMP `InventoryContentPacket::encodePayload` 1001 = gophertunnel
    /// `packet/inventory_content.go` 1.26.45) : VarUInt32 windowId, VarUInt32
    /// count + NetworkItemStackDescriptor par slot, FullContainerName, puis
    /// descripteur de l'item de stockage (air).
    pub fn encode_items(
        window_id: u32,
        items: &[ItemStackWrapper],
        full_container_name: &FullContainerName,
    ) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(256);
        w.write_var_u32(window_id);
        w.write_var_u32(items.len() as u32);
        for item in items {
            item.encode_descriptor(&mut w);
        }
        full_container_name.encode(&mut w);
        ItemStackWrapper::air().encode_descriptor(&mut w);
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
        // Layout (PMMP `InventorySlotPacket::encodePayload`, gophertunnel
        // `packet/inventory_slot.go`) :
        //   VarU32 windowId + VarU32 inventorySlot
        //   writeOptional(containerName)  → bool + FullContainerName::write
        //   writeOptional(storage)        → bool (+ descriptor si présent)
        //   NetworkItemStackDescriptor item
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
    /// Layout (PMMP `MobEquipmentPacket::encodePayload`) : `actorRuntimeId` +
    /// `NetworkItemStackDescriptor item` + `Byte inventorySlot` + `Byte
    /// hotbarSlot` + `Byte windowId`.
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
