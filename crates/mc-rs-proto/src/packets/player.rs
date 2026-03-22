use crate::io::{ProtoReader, ProtoWriter};

// ── PlayerAuthInput (C→S, 0x90) ──

/// Player movement and input — decoded minimally (position + rotation only).
pub struct PlayerAuthInput {
    pub pitch: f32,
    pub yaw: f32,
    pub position: [f32; 3],
    pub move_vec_x: f32,
    pub move_vec_z: f32,
    pub head_yaw: f32,
    // inputFlags, inputMode, playMode, etc. — skipped for now
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
        // Don't read further — remaining fields are complex and not needed yet
        Ok(Self {
            pitch,
            yaw,
            position: [pos_x, pos_y, pos_z],
            move_vec_x,
            move_vec_z,
            head_yaw,
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
    pub mode: u8,           // 0=normal, 1=reset, 2=teleport, 3=rotation
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

pub struct Text {
    pub text_type: u8,
    pub needs_translation: bool,
    pub source_name: String,
    pub message: String,
    pub xuid: String,
    pub platform_chat_id: String,
}

impl Text {
    /// Decode a Text packet from the client (type=1 = chat).
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let text_type = reader.read_u8()?;
        let needs_translation = reader.read_bool()?;

        let (source_name, message) = match text_type {
            1 | 2 | 7 => {
                // CHAT, WHISPER, ANNOUNCEMENT — has source + message
                let source = reader.read_string()?;
                let msg = reader.read_string()?;
                (source, msg)
            }
            0 | 6 | 8 | 9 => {
                // RAW, JUKEBOX_POPUP, SYSTEM, OBJECT_WHISPER — message only
                let msg = reader.read_string()?;
                (String::new(), msg)
            }
            3 | 4 | 5 => {
                // TRANSLATION, POPUP, TIP — message + params
                let msg = reader.read_string()?;
                let count = reader.read_var_u32()?;
                for _ in 0..count {
                    let _ = reader.read_string()?; // skip params
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

        Ok(Self {
            text_type,
            needs_translation,
            source_name,
            message,
            xuid,
            platform_chat_id,
        })
    }

    /// Encode a chat message to broadcast.
    pub fn chat(source: &str, message: &str, xuid: &str) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_u8(1); // type = CHAT
        w.write_bool(false); // needs_translation
        w.write_string(source);
        w.write_string(message);
        w.write_string(xuid);
        w.write_string(""); // platform_chat_id
        w.into_bytes()
    }

    /// Encode a system message (no source name).
    pub fn system(message: &str) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_u8(0); // type = RAW
        w.write_bool(false);
        w.write_string(message);
        w.write_string(""); // xuid
        w.write_string(""); // platform_chat_id
        w.into_bytes()
    }
}

// ── PlayerList (S→C, 0x3F) ──

pub struct PlayerListAdd {
    pub uuid: [u8; 16],
    pub entity_id: i64,        // varint64 (actor unique ID)
    pub username: String,
    pub xuid: String,
    pub platform_chat_id: String,
    pub build_platform: i32,   // i32_le
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
    w.write_string("Custom");          // skin_id
    w.write_string("");                 // play_fab_id
    w.write_string("geometry.humanoid.custom"); // skin_resource_patch
    // Skin image data
    w.write_u32_le(64);                // width
    w.write_u32_le(64);                // height
    let skin_data = vec![0u8; 64 * 64 * 4]; // RGBA
    w.write_byte_array(&skin_data);

    // Animations — empty
    w.write_u32_le(0);

    // Cape image — empty
    w.write_u32_le(0);  // width
    w.write_u32_le(0);  // height
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
        w.write_f32_le(0.1);  // walk speed
        // Entity links (none)
        w.write_var_u32(0);
        // Device ID + OS
        w.write_string(""); // device_id
        w.write_i32_le(0);  // device_os
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
    pub walk_speed: f32,
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
            w.write_f32_le(layer.walk_speed);
        }

        w.into_bytes()
    }

    pub fn default_creative(entity_id: i64) -> Self {
        Self {
            entity_id,
            permission_level: 2,  // operator
            command_permission: 1, // game directors
            layers: vec![
                AbilitiesLayer {
                    layer_type: 1, // BASE
                    abilities_set: 0x1BFFF,    // all abilities
                    abilities_values: 0x18063,  // creative defaults
                    fly_speed: 0.05,
                    walk_speed: 0.1,
                },
            ],
        }
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
