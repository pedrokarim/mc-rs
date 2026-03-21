use crate::io::ProtoWriter;

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
