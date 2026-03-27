use mc_rs_proto::packets::player::{
    entity_flags, AddActor, AddActorAttribute, MetadataValue, MoveActorAbsolute, RemoveEntity,
    SetActorData, SetActorMotion,
};

use crate::player_registry;

#[derive(Clone)]
pub struct EntityBase {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub actor_type: String,
    pub selector_type: String,
    pub display_name: String,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub body_yaw: f32,
    pub attributes: Vec<AddActorAttribute>,
    pub metadata: Vec<(u32, u32, MetadataValue)>,
}

impl EntityBase {
    pub fn new(
        actor_type: impl Into<String>,
        selector_type: impl Into<String>,
        display_name: impl Into<String>,
        position: [f32; 3],
        attributes: Vec<AddActorAttribute>,
        metadata: Vec<(u32, u32, MetadataValue)>,
    ) -> Self {
        let entity_unique_id = player_registry::next_entity_id();
        Self {
            entity_unique_id,
            entity_runtime_id: entity_unique_id as u64,
            actor_type: actor_type.into(),
            selector_type: selector_type.into(),
            display_name: display_name.into(),
            position,
            velocity: [0.0, 0.0, 0.0],
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            body_yaw: 0.0,
            attributes,
            metadata,
        }
    }

    pub fn add_actor_packet(&self) -> Vec<u8> {
        AddActor {
            entity_unique_id: self.entity_unique_id,
            entity_runtime_id: self.entity_runtime_id,
            actor_type: self.actor_type.clone(),
            position: self.position,
            velocity: self.velocity,
            pitch: self.pitch,
            yaw: self.yaw,
            head_yaw: self.head_yaw,
            body_yaw: self.body_yaw,
            attributes: self.attributes.clone(),
            metadata: self.metadata.clone(),
        }
        .encode()
    }

    pub fn remove_packet(&self) -> Vec<u8> {
        RemoveEntity {
            entity_unique_id: self.entity_unique_id,
        }
        .encode()
    }

    pub fn actor_data_packet(&self) -> Vec<u8> {
        SetActorData {
            runtime_entity_id: self.entity_runtime_id,
            metadata: self.metadata.clone(),
            tick: 0,
        }
        .encode()
    }

    pub fn move_absolute_packet(&self, on_ground: bool, teleport: bool) -> Vec<u8> {
        let mut flags = 0;
        if on_ground {
            flags |= MoveActorAbsolute::FLAG_GROUND;
        }
        if teleport {
            flags |= MoveActorAbsolute::FLAG_TELEPORT;
        }
        MoveActorAbsolute {
            runtime_entity_id: self.entity_runtime_id,
            position: self.position,
            pitch: self.pitch,
            yaw: self.yaw,
            head_yaw: self.head_yaw,
            flags,
        }
        .encode()
    }

    pub fn motion_packet(&self) -> Vec<u8> {
        SetActorMotion {
            runtime_entity_id: self.entity_runtime_id,
            motion: self.velocity,
        }
        .encode()
    }
}

pub fn health_attributes(max_health: f32) -> Vec<AddActorAttribute> {
    vec![AddActorAttribute {
        name: "minecraft:health".to_string(),
        min: 0.0,
        current: max_health,
        max: max_health,
    }]
}

pub fn living_metadata(
    width: f32,
    height: f32,
    name_tag: Option<&str>,
) -> Vec<(u32, u32, MetadataValue)> {
    let name_tag = name_tag.unwrap_or_default();
    let mut flags =
        entity_flags::BREATHING | entity_flags::HAS_GRAVITY | entity_flags::HAS_COLLISION;
    if !name_tag.is_empty() {
        flags |= entity_flags::CAN_SHOW_NAMETAG;
    }

    vec![
        (0, 7, MetadataValue::Long(flags)),
        (4, 4, MetadataValue::String(name_tag.to_string())),
        (5, 7, MetadataValue::Long(-1)),
        (6, 7, MetadataValue::Long(0)),
        (37, 7, MetadataValue::Long(-1)),
        (38, 3, MetadataValue::Float(1.0)),
        (53, 3, MetadataValue::Float(width)),
        (54, 3, MetadataValue::Float(height)),
        (81, 0, MetadataValue::Byte(0)),
        (84, 4, MetadataValue::String(String::new())),
    ]
}

pub fn item_metadata() -> Vec<(u32, u32, MetadataValue)> {
    let flags = entity_flags::HAS_GRAVITY | entity_flags::HAS_COLLISION;
    vec![
        (0, 7, MetadataValue::Long(flags)),
        (3, 0, MetadataValue::Byte(0)),
        (4, 4, MetadataValue::String(String::new())),
        (5, 7, MetadataValue::Long(-1)),
        (6, 7, MetadataValue::Long(0)),
        (37, 7, MetadataValue::Long(-1)),
        (38, 3, MetadataValue::Float(1.0)),
        (53, 3, MetadataValue::Float(0.25)),
        (54, 3, MetadataValue::Float(0.25)),
        (81, 0, MetadataValue::Byte(0)),
        (84, 4, MetadataValue::String(String::new())),
    ]
}
