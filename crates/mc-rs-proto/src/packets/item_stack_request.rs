//! ItemStackRequest (0x93) — Client → Server.
//!
//! The client sends inventory manipulation requests. The server validates
//! and responds with ItemStackResponse.

use bytes::Buf;

use crate::codec::{read_string, ProtoDecode};
use crate::error::ProtoError;
use crate::types::{VarInt, VarUInt32};

/// A reference to a specific container slot.
#[derive(Debug, Clone)]
pub struct StackSlot {
    /// Container ID (0 = inventory, 119 = armor, 120 = creative output,
    /// 58 = cursor, 59 = creative menu, 124 = offhand).
    pub container_id: u8,
    /// Slot index within the container.
    pub slot: u8,
    /// Server-assigned stack network ID for the item in this slot.
    pub stack_network_id: i32,
}

impl ProtoDecode for StackSlot {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 1 {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: buf.remaining(),
            });
        }
        let container_id = buf.get_u8();
        if buf.remaining() < 1 {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: buf.remaining(),
            });
        }
        let slot = buf.get_u8();
        let stack_network_id = VarInt::proto_decode(buf)?.0;
        Ok(Self {
            container_id,
            slot,
            stack_network_id,
        })
    }
}

/// Individual action within an ItemStackRequest.
#[derive(Debug, Clone)]
pub enum StackAction {
    Take {
        count: u8,
        src: StackSlot,
        dst: StackSlot,
    },
    Place {
        count: u8,
        src: StackSlot,
        dst: StackSlot,
    },
    Swap {
        src: StackSlot,
        dst: StackSlot,
    },
    Drop {
        count: u8,
        src: StackSlot,
        randomly: bool,
    },
    Destroy {
        count: u8,
        src: StackSlot,
    },
    Consume {
        count: u8,
        src: StackSlot,
    },
    Create {
        result_slot: u8,
    },
    /// Craft a recipe by network ID (action type 12).
    CraftRecipe {
        recipe_network_id: u32,
    },
    /// Auto-craft a recipe (shift-click craft) (action type 13).
    CraftRecipeAuto {
        recipe_network_id: u32,
        /// Number of times to repeat the craft.
        times_crafted: u8,
        /// Ingredients expected by the client.
        ingredients: Vec<u8>,
    },
    CraftCreative {
        creative_item_network_id: u32,
    },
    /// Craft via enchanting table option selection (action type 15).
    CraftRecipeOptional {
        recipe_network_id: u32,
        filter_string_index: i32,
    },
    /// Craft via grindstone (action type 16).
    CraftGrindstone {
        recipe_network_id: u32,
    },
    /// Craft via loom (action type 17).
    CraftLoom {
        pattern_id: String,
    },
    /// Any action type we don't handle yet.
    Unknown {
        action_type: u8,
    },
}

/// A single request containing one or more actions.
#[derive(Debug, Clone)]
pub struct StackRequest {
    pub request_id: i32,
    pub actions: Vec<StackAction>,
    pub filter_strings: Vec<String>,
    pub filter_cause: i32,
}

/// The complete ItemStackRequest packet containing one or more requests.
pub struct ItemStackRequest {
    pub requests: Vec<StackRequest>,
}

impl ProtoDecode for ItemStackRequest {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let count = VarUInt32::proto_decode(buf)?.0;
        let mut requests = Vec::with_capacity(count as usize);
        for _ in 0..count {
            requests.push(decode_stack_request(buf)?);
        }
        Ok(Self { requests })
    }
}

fn decode_stack_request(buf: &mut impl Buf) -> Result<StackRequest, ProtoError> {
    let request_id = VarInt::proto_decode(buf)?.0;

    let action_count = VarUInt32::proto_decode(buf)?.0;
    let mut actions = Vec::with_capacity(action_count as usize);
    for _ in 0..action_count {
        actions.push(decode_stack_action(buf)?);
    }

    // Filter strings (text input from anvil, etc.)
    let filter_count = VarUInt32::proto_decode(buf)?.0;
    let mut filter_strings = Vec::with_capacity(filter_count as usize);
    for _ in 0..filter_count {
        filter_strings.push(read_string(buf)?);
    }

    let filter_cause = VarInt::proto_decode(buf)?.0;

    Ok(StackRequest {
        request_id,
        actions,
        filter_strings,
        filter_cause,
    })
}

fn decode_stack_action(buf: &mut impl Buf) -> Result<StackAction, ProtoError> {
    if buf.remaining() < 1 {
        return Err(ProtoError::BufferTooShort {
            needed: 1,
            remaining: buf.remaining(),
        });
    }
    let action_type = buf.get_u8();

    match action_type {
        0 => {
            // Take
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let count = buf.get_u8();
            let src = StackSlot::proto_decode(buf)?;
            let dst = StackSlot::proto_decode(buf)?;
            Ok(StackAction::Take { count, src, dst })
        }
        1 => {
            // Place
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let count = buf.get_u8();
            let src = StackSlot::proto_decode(buf)?;
            let dst = StackSlot::proto_decode(buf)?;
            Ok(StackAction::Place { count, src, dst })
        }
        2 => {
            // Swap
            let src = StackSlot::proto_decode(buf)?;
            let dst = StackSlot::proto_decode(buf)?;
            Ok(StackAction::Swap { src, dst })
        }
        3 => {
            // Drop
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let count = buf.get_u8();
            let src = StackSlot::proto_decode(buf)?;
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let randomly = buf.get_u8() != 0;
            Ok(StackAction::Drop {
                count,
                src,
                randomly,
            })
        }
        4 => {
            // Destroy
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let count = buf.get_u8();
            let src = StackSlot::proto_decode(buf)?;
            Ok(StackAction::Destroy { count, src })
        }
        5 => {
            // Consume
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let count = buf.get_u8();
            let src = StackSlot::proto_decode(buf)?;
            Ok(StackAction::Consume { count, src })
        }
        6 => {
            // Create
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let result_slot = buf.get_u8();
            Ok(StackAction::Create { result_slot })
        }
        12 => {
            // CraftRecipe
            let recipe_network_id = VarUInt32::proto_decode(buf)?.0;
            Ok(StackAction::CraftRecipe { recipe_network_id })
        }
        13 => {
            // CraftRecipeAuto
            let recipe_network_id = VarUInt32::proto_decode(buf)?.0;
            if buf.remaining() < 1 {
                return Err(ProtoError::BufferTooShort {
                    needed: 1,
                    remaining: buf.remaining(),
                });
            }
            let times_crafted = buf.get_u8();
            let ingredient_count = VarUInt32::proto_decode(buf)?.0;
            let mut ingredients = Vec::with_capacity(ingredient_count as usize);
            for _ in 0..ingredient_count {
                if buf.remaining() < 1 {
                    return Err(ProtoError::BufferTooShort {
                        needed: 1,
                        remaining: buf.remaining(),
                    });
                }
                ingredients.push(buf.get_u8());
            }
            Ok(StackAction::CraftRecipeAuto {
                recipe_network_id,
                times_crafted,
                ingredients,
            })
        }
        14 => {
            // CraftCreative
            let creative_item_network_id = VarUInt32::proto_decode(buf)?.0;
            Ok(StackAction::CraftCreative {
                creative_item_network_id,
            })
        }
        15 => {
            // CraftRecipeOptional (enchanting table selection)
            let recipe_network_id = VarUInt32::proto_decode(buf)?.0;
            let filter_string_index = VarInt::proto_decode(buf)?.0;
            Ok(StackAction::CraftRecipeOptional {
                recipe_network_id,
                filter_string_index,
            })
        }
        16 => {
            // CraftGrindstone
            let recipe_network_id = VarUInt32::proto_decode(buf)?.0;
            Ok(StackAction::CraftGrindstone { recipe_network_id })
        }
        17 => {
            // CraftLoom
            let pattern_id = read_string(buf)?;
            Ok(StackAction::CraftLoom { pattern_id })
        }
        _ => {
            // Unknown action — we can't skip it safely since we don't know the size.
            // Return as Unknown; the caller should stop processing this request.
            Ok(StackAction::Unknown { action_type })
        }
    }
}
