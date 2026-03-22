use crate::io::ProtoWriter;

// ── LevelChunk (S→C, 0x3A) ──

pub struct LevelChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub dimension_id: i32,
    pub sub_chunk_count: u32,
    pub cache_enabled: bool,
    pub payload: Vec<u8>, // serialized sub-chunks + biomes + borders + tiles
}

impl LevelChunk {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(self.payload.len() + 32);
        w.write_var_i32(self.chunk_x);
        w.write_var_i32(self.chunk_z);
        w.write_var_i32(self.dimension_id);
        w.write_var_u32(self.sub_chunk_count);
        w.write_bool(self.cache_enabled);
        w.write_byte_array(&self.payload);
        w.into_bytes()
    }
}

// ── RequestChunkRadius (C→S, 0x45) ──

pub struct RequestChunkRadius {
    pub radius: i32,
}

impl RequestChunkRadius {
    pub fn decode(
        reader: &mut crate::io::ProtoReader,
    ) -> Result<Self, crate::io::reader::ProtoReadError> {
        let radius = reader.read_var_i32()?;
        Ok(Self { radius })
    }
}

// ── ChunkRadiusUpdated (S→C, 0x46) ──

pub struct ChunkRadiusUpdated {
    pub radius: i32,
}

impl ChunkRadiusUpdated {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(4);
        w.write_var_i32(self.radius);
        w.into_bytes()
    }
}

// ── NetworkChunkPublisherUpdate (S→C, 0x79) ──

pub struct NetworkChunkPublisherUpdate {
    pub position: [i32; 3], // BlockPos (varint32 x3)
    pub radius: u32,        // varuint32 (in blocks)
}

impl NetworkChunkPublisherUpdate {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32);
        w.write_var_i32(self.position[0]);
        w.write_var_u32(self.position[1] as u32);
        w.write_var_i32(self.position[2]);
        w.write_var_u32(self.radius);
        // Saved chunks — empty list
        w.write_u32_le(0);
        w.into_bytes()
    }
}
