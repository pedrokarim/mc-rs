use mc_rs_proto::io::{ProtoReader, ProtoWriter};

/// Serialize a sub-chunk with the given palette (network format).
///
/// Format:
/// - version: u8 = 8
/// - storage_count: u8
/// - For each storage layer:
///   - header: (bits_per_block << 1) | 1 (runtime flag)
///   - If bits == 0: single VarInt32 palette value (no word array)
///   - If bits > 0: word array + VarInt32 palette count + VarInt32[] palette
pub fn serialize_sub_chunk(blocks: &[u32; 4096], palette: &[u32]) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(512);
    w.write_u8(8); // version
    w.write_u8(1); // storage_count = 1 layer

    serialize_paletted_storage(&mut w, blocks, palette);

    w.into_bytes()
}

/// Serialize an empty sub-chunk (all air).
#[allow(dead_code)]
pub fn serialize_empty_sub_chunk() -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(4);
    w.write_u8(8); // version
    w.write_u8(0); // storage_count = 0 (no layers)
    w.into_bytes()
}

/// Serialize a single biome section (4x4x4 = 64 entries).
/// For flat world: all Plains (biome ID = 1), single-value palette.
pub fn serialize_biome_section_single(biome_id: u32) -> Vec<u8> {
    serialize_biome_section(&[biome_id; 64])
}

/// Serialize a biome section from 4x4x4 cells (index `(x << 4) | (z << 2) | y`).
///
/// Sur le fil, une section de biomes est un stockage palettisé de 4 096 cases
/// (16×16×16), exactement comme les blocs : chaque cellule 4×4×4 est donc
/// étendue aux 64 blocs qu'elle couvre (index `(x << 8) | (z << 4) | y`).
/// Réf : dragonfly `chunk/decode.go` `decodePalettedStorage` (même
/// `paletteSize.uint32s()` sur 4 096 indices pour blocs et biomes).
/// Écrire seulement 64 cases ne passait que pour les sections à biome unique
/// (sans tableau d'indices) : une section mixte était 64 fois trop courte et
/// le client 1.26.45 coupait la connexion (`BadPacket`, code « Block »).
pub fn serialize_biome_section(biomes: &[u32; 64]) -> Vec<u8> {
    let mut palette = Vec::new();
    let mut cell_indices = [0u32; 64];

    for (i, &biome) in biomes.iter().enumerate() {
        let idx = if let Some(pos) = palette.iter().position(|&b| b == biome) {
            pos as u32
        } else {
            palette.push(biome);
            (palette.len() - 1) as u32
        };
        cell_indices[i] = idx;
    }

    let mut palette_indices = [0u32; 4096];
    for (i, slot) in palette_indices.iter_mut().enumerate() {
        let (x, z, y) = (i >> 8, (i >> 4) & 0xF, i & 0xF);
        *slot = cell_indices[((x >> 2) << 4) | ((z >> 2) << 2) | (y >> 2)];
    }

    let mut w = ProtoWriter::with_capacity(4);
    serialize_paletted_storage(&mut w, &palette_indices, &palette);
    w.into_bytes()
}

/// Une section de biomes relue depuis un blob sérialisé.
enum ParsedBiomeSection {
    /// Octet `0xFF` : même stockage que la section précédente.
    SameAsPrevious,
    /// Biomes des 64 cellules 4×4×4 (seulement pour le format hérité).
    Cells(Box<[u32; 64]>),
    /// Section valide au format courant (contenu non décodé).
    Current,
}

/// Relit une suite de sections de biomes dont les stockages palettisés
/// couvrent `cell_count` cases (64 : format hérité, 4 096 : format réseau).
/// Retourne `None` si le blob n'est pas consommé exactement.
fn parse_biome_sections(data: &[u8], cell_count: usize) -> Option<Vec<ParsedBiomeSection>> {
    let mut reader = ProtoReader::new(data);
    let mut sections = Vec::new();
    while reader.remaining() > 0 {
        let header = reader.read_u8().ok()?;
        if header == 0xFF {
            if sections.is_empty() {
                return None;
            }
            sections.push(ParsedBiomeSection::SameAsPrevious);
            continue;
        }
        let bits = (header >> 1) as usize;
        if bits == 0 {
            let value = reader.read_var_i32().ok()? as u32;
            sections.push(if cell_count == 64 {
                ParsedBiomeSection::Cells(Box::new([value; 64]))
            } else {
                ParsedBiomeSection::Current
            });
            continue;
        }
        if !matches!(bits, 1..=6 | 8 | 16) {
            return None;
        }
        let per_word = 32 / bits;
        let words = (0..cell_count.div_ceil(per_word))
            .map(|_| reader.read_u32_le().ok())
            .collect::<Option<Vec<u32>>>()?;
        let palette_len = reader.read_var_i32().ok()?;
        if palette_len <= 0 || palette_len as usize > (1 << bits.min(16)) {
            return None;
        }
        let palette = (0..palette_len)
            .map(|_| reader.read_var_i32().ok().map(|v| v as u32))
            .collect::<Option<Vec<u32>>>()?;
        if cell_count != 64 {
            sections.push(ParsedBiomeSection::Current);
            continue;
        }
        let mask = (1u32 << bits) - 1;
        let mut cells = [0u32; 64];
        for (k, cell) in cells.iter_mut().enumerate() {
            let index = (words[k / per_word] >> ((k % per_word) * bits)) & mask;
            *cell = *palette.get(index as usize)?;
        }
        sections.push(ParsedBiomeSection::Cells(Box::new(cells)));
    }
    Some(sections)
}

/// Convertit des biomes sauvegardés avant la correction du format (sections
/// de 64 cases au lieu de 4 096, cf. [`serialize_biome_section`]) vers le
/// format réseau. Retourne `None` si le blob est déjà au format courant ou
/// n'est lisible dans aucun des deux formats (laissé tel quel).
pub fn upgrade_legacy_biome_data(data: &[u8]) -> Option<Vec<u8>> {
    if parse_biome_sections(data, 4096).is_some() {
        return None;
    }
    let sections = parse_biome_sections(data, 64)?;
    let mut out = Vec::with_capacity(data.len());
    for section in sections {
        match section {
            ParsedBiomeSection::SameAsPrevious => out.push(0xFF),
            ParsedBiomeSection::Cells(cells) => out.extend(serialize_biome_section(&cells)),
            ParsedBiomeSection::Current => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod biome_section_tests {
    use super::{parse_biome_sections, serialize_biome_section, upgrade_legacy_biome_data};

    /// Section au format hérité (64 cases) : 1 bit par case, 2 mots.
    fn legacy_mixed_section() -> Vec<u8> {
        let mut bytes = vec![(1 << 1) | 1];
        // Case 0 → index 1 (biome 27), le reste → index 0 (biome 6).
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&[4, 12, 54]); // palette : 2 entrées, 6 et 27
        bytes
    }

    #[test]
    fn legacy_biome_data_is_upgraded_to_4096_cells() {
        let mut legacy = legacy_mixed_section();
        legacy.push(0xFF); // section suivante identique
        let upgraded = upgrade_legacy_biome_data(&legacy).expect("format hérité reconnu");
        let mut cells = [6u32; 64];
        cells[0] = 27;
        let mut expected = serialize_biome_section(&cells);
        expected.push(0xFF);
        assert_eq!(upgraded, expected);
        assert!(parse_biome_sections(&upgraded, 4096).is_some());
    }

    #[test]
    fn current_biome_data_is_left_untouched() {
        let mut cells = [6u32; 64];
        cells[5] = 188;
        let mut current = serialize_biome_section(&cells);
        current.extend(serialize_biome_section(&[6; 64]));
        assert_eq!(upgrade_legacy_biome_data(&current), None);
    }

    #[test]
    fn unreadable_biome_data_is_left_untouched() {
        assert_eq!(upgrade_legacy_biome_data(&[0x03, 0x00]), None);
    }

    #[test]
    fn mixed_biome_section_covers_4096_cells() {
        // Deux biomes : 1 bit par case, 4 096 cases → 128 mots de 32 bits.
        let mut cells = [6u32; 64];
        cells[0] = 27;
        let bytes = serialize_biome_section(&cells);
        assert_eq!(bytes[0], (1 << 1) | 1);
        // en-tête + 128 × 4 octets + taille de palette (2) + 2 entrées.
        assert_eq!(bytes.len(), 1 + 128 * 4 + 1 + 2);
        // La cellule 0 couvre les blocs x, y, z de 0 à 3 : le premier mot a ses
        // 4 bits bas (y = 0..3 pour x = z = 0) à l'index 0 (biome 27).
        assert_eq!(bytes[1] & 0x0F, 0);
    }

    #[test]
    fn single_biome_section_stays_compact() {
        let bytes = serialize_biome_section(&[6; 64]);
        assert_eq!(bytes, vec![1, 12]);
    }
}

/// Serialize all biome sections for a chunk from its 16x16 biome columns.
///
/// Bedrock uses 4x4x4 biome paletted storages per sub-chunk. We do not yet
/// model vertical biome variation, so the same 4x4 horizontal map is repeated
/// across the 4 Y slices of each section and across all vertical sections.
pub fn serialize_biome_sections_from_columns(
    biome_ids: &[[u32; 16]; 16],
    section_count: usize,
) -> Vec<u8> {
    let mut coarse = [0u32; 64];

    #[allow(clippy::needless_range_loop)]
    for coarse_x in 0..4usize {
        #[allow(clippy::needless_range_loop)]
        for coarse_z in 0..4usize {
            let mut counts: Vec<(u32, usize)> = Vec::new();
            for x in (coarse_x * 4)..(coarse_x * 4 + 4) {
                for z in (coarse_z * 4)..(coarse_z * 4 + 4) {
                    let biome = biome_ids[x][z];
                    if let Some((_, count)) = counts.iter_mut().find(|(id, _)| *id == biome) {
                        *count += 1;
                    } else {
                        counts.push((biome, 1));
                    }
                }
            }

            let mut selected = biome_ids[coarse_x * 4][coarse_z * 4];
            let mut best_count = 0usize;
            for (biome, count) in counts {
                if count > best_count {
                    selected = biome;
                    best_count = count;
                }
            }

            for coarse_y in 0..4usize {
                let idx = (coarse_x << 4) | (coarse_z << 2) | coarse_y;
                coarse[idx] = selected;
            }
        }
    }

    let section = serialize_biome_section(&coarse);
    let mut data = Vec::with_capacity(section.len() * section_count);
    for _ in 0..section_count {
        data.extend_from_slice(&section);
    }
    data
}

/// Serialize a paletted storage (blocks or biomes).
fn serialize_paletted_storage(w: &mut ProtoWriter, data: &[u32], palette: &[u32]) {
    if palette.len() <= 1 {
        // Single-value: bits_per_block = 0
        let header = 1u8; // bits=0, runtime flag
        w.write_u8(header);
        w.write_var_i32(palette.first().copied().unwrap_or(0) as i32);
        return;
    }

    // Calculate bits per block
    let bits = bits_for_palette(palette.len());
    let header = (bits << 1) | 1; // runtime flag
    w.write_u8(header);

    // Word array
    let blocks_per_word = 32 / bits as usize;
    let word_count = data.len().div_ceil(blocks_per_word);

    for word_idx in 0..word_count {
        let mut word: u32 = 0;
        for bit_idx in 0..blocks_per_word {
            let block_idx = word_idx * blocks_per_word + bit_idx;
            if block_idx >= data.len() {
                break;
            }
            let palette_idx = data[block_idx];
            word |= (palette_idx & ((1 << bits) - 1)) << (bit_idx * bits as usize);
        }
        w.write_u32_le(word);
    }

    // Palette count + entries (VarInt32 zigzag)
    w.write_var_i32(palette.len() as i32);
    for &entry in palette {
        w.write_var_i32(entry as i32);
    }
}

/// Determine the number of bits per block for a given palette size.
/// Must be one of: 1, 2, 3, 4, 5, 6, 8, 16
fn bits_for_palette(palette_size: usize) -> u8 {
    match palette_size {
        0..=2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        33..=64 => 6,
        65..=256 => 8,
        _ => 16,
    }
}

/// A sub-chunk storing 16x16x16 blocks as actual runtime IDs.
/// Index = (x << 8) | (z << 4) | y
///
/// Deux couches, comme Bedrock : `blocks` (blocs) et `liquid`, l'eau d'un bloc qui en contient
/// (waterlogging : herbe marine, escalier immergé…), absente tant qu'aucun bloc n'en porte.
#[derive(Clone)]
pub struct SubChunk {
    pub blocks: [u32; 4096],
    pub liquid: Option<Box<[u32; 4096]>>,
}

impl SubChunk {
    pub fn new_air(air_id: u32) -> Self {
        Self {
            blocks: [air_id; 4096],
            liquid: None,
        }
    }

    /// Get the block runtime ID at local coordinates.
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u32 {
        self.blocks[(x << 8) | (z << 4) | y]
    }

    /// Set the block runtime ID at local coordinates.
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, runtime_id: u32) {
        self.blocks[(x << 8) | (z << 4) | y] = runtime_id;
    }

    /// Liquide (seconde couche) aux coordonnées locales ; air s'il n'y en a pas.
    pub fn get_liquid(&self, x: usize, y: usize, z: usize) -> u32 {
        match &self.liquid {
            Some(layer) => layer[(x << 8) | (z << 4) | y],
            None => super::block_registry::BLOCKS.air,
        }
    }

    /// Fixe le liquide (seconde couche) aux coordonnées locales.
    pub fn set_liquid(&mut self, x: usize, y: usize, z: usize, runtime_id: u32) {
        let air = super::block_registry::BLOCKS.air;
        if self.liquid.is_none() && runtime_id == air {
            return;
        }
        let layer = self.liquid.get_or_insert_with(|| Box::new([air; 4096]));
        layer[(x << 8) | (z << 4) | y] = runtime_id;
    }

    /// Serialize this sub-chunk to network format (seconde couche seulement si un bloc porte du liquide).
    pub fn serialize(&self) -> Vec<u8> {
        let air = super::block_registry::BLOCKS.air;
        let liquid = self
            .liquid
            .as_deref()
            .filter(|layer| layer.iter().any(|&b| b != air));
        let mut w = ProtoWriter::with_capacity(512);
        w.write_u8(8); // version
        w.write_u8(if liquid.is_some() { 2 } else { 1 });
        for layer in std::iter::once(&self.blocks).chain(liquid) {
            let (indices, palette) = palettize(layer);
            serialize_paletted_storage(&mut w, &indices, &palette);
        }
        w.into_bytes()
    }
}

/// Palette d'une couche et indices de ses 4 096 cases.
fn palettize(layer: &[u32; 4096]) -> ([u32; 4096], Vec<u32>) {
    let mut palette: Vec<u32> = Vec::new();
    let mut indices = [0u32; 4096];
    for (i, &block) in layer.iter().enumerate() {
        indices[i] = match palette.iter().position(|&b| b == block) {
            Some(pos) => pos as u32,
            None => {
                palette.push(block);
                (palette.len() - 1) as u32
            }
        };
    }
    (indices, palette)
}

/// Lit une couche palettisée de 4 096 cases (runtime IDs).
fn read_storage(reader: &mut ProtoReader, air_id: u32) -> Option<[u32; 4096]> {
    let mut blocks = [air_id; 4096];
    let header = reader.read_u8().ok()?;
    let bits_per_block = header >> 1;
    if bits_per_block == 0 {
        // Single-value palette
        let value = reader.read_var_i32().ok()? as u32;
        blocks.fill(value);
        return Some(blocks);
    }
    let blocks_per_word = 32 / bits_per_block as usize;
    let word_count = 4096usize.div_ceil(blocks_per_word);
    let mask = (1u32 << bits_per_block) - 1;

    let mut palette_indices = [0u32; 4096];
    for word_idx in 0..word_count {
        let word = reader.read_u32_le().ok()?;
        for bit_idx in 0..blocks_per_word {
            let block_idx = word_idx * blocks_per_word + bit_idx;
            if block_idx >= 4096 {
                break;
            }
            palette_indices[block_idx] = (word >> (bit_idx * bits_per_block as usize)) & mask;
        }
    }

    let palette_count = reader.read_var_i32().ok()? as usize;
    let mut palette = Vec::with_capacity(palette_count);
    for _ in 0..palette_count {
        palette.push(reader.read_var_i32().ok()? as u32);
    }
    for i in 0..4096 {
        blocks[i] = palette
            .get(palette_indices[i] as usize)
            .copied()
            .unwrap_or(air_id);
    }
    Some(blocks)
}

/// Deserialize a single sub-chunk from network bytes.
/// Returns (SubChunk, bytes_consumed). Première couche = blocs, seconde = liquide ; les couches
/// suivantes éventuelles sont lues et ignorées (pour rester aligné sur le sous-chunk suivant).
pub fn deserialize_sub_chunk_data(data: &[u8]) -> Option<(SubChunk, usize)> {
    if data.len() < 2 {
        return None;
    }

    let mut reader = ProtoReader::new(data);
    let version = reader.read_u8().ok()?;
    if version != 8 {
        return None;
    }

    let storage_count = reader.read_u8().ok()?;
    let air_id = super::block_registry::BLOCKS.air;
    let mut sub = SubChunk::new_air(air_id);
    for layer in 0..storage_count {
        let blocks = read_storage(&mut reader, air_id)?;
        match layer {
            0 => sub.blocks = blocks,
            1 if blocks.iter().any(|&b| b != air_id) => sub.liquid = Some(Box::new(blocks)),
            _ => {}
        }
    }

    Some((sub, reader.position()))
}

/// Réécrit les entrées de palette d'un sous-chunk sérialisé (format réseau
/// version 8, toutes couches) via `remap`, sans toucher aux mots de bits.
/// Retourne `None` si les données ne sont pas un sous-chunk v8 bien formé.
pub fn remap_sub_chunk_palette(data: &[u8], remap: impl Fn(u32) -> u32) -> Option<Vec<u8>> {
    let mut reader = ProtoReader::new(data);
    let mut w = ProtoWriter::with_capacity(data.len() + 16);

    let version = reader.read_u8().ok()?;
    if version != 8 {
        return None;
    }
    w.write_u8(version);
    let storage_count = reader.read_u8().ok()?;
    w.write_u8(storage_count);

    for _ in 0..storage_count {
        let header = reader.read_u8().ok()?;
        w.write_u8(header);
        let bits_per_block = header >> 1;
        if bits_per_block == 0 {
            let value = reader.read_var_i32().ok()? as u32;
            w.write_var_i32(remap(value) as i32);
            continue;
        }
        let blocks_per_word = 32 / bits_per_block as usize;
        let word_count = 4096usize.div_ceil(blocks_per_word);
        w.write_raw(&reader.read_raw(word_count * 4).ok()?);

        let palette_count = reader.read_var_i32().ok()?;
        w.write_var_i32(palette_count);
        for _ in 0..palette_count {
            let value = reader.read_var_i32().ok()? as u32;
            w.write_var_i32(remap(value) as i32);
        }
    }

    // Octets éventuels au-delà des couches : conservés tels quels.
    w.write_raw(reader.remaining_bytes());
    Some(w.into_bytes())
}

/// Rebuild the full network payload from sub-chunks, biome data and block entities.
/// Format (réf PMMP `ChunkSerializer`) : sub-chunks, biomes, byte border-blocks (0),
/// puis les NBT réseau des block entities **concaténés sans compteur**.
pub fn rebuild_network_payload(
    sub_chunks: &[SubChunk],
    biome_data: &[u8],
    block_entities: &[crate::block_entities::BlockEntity],
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(16384);

    for sub in sub_chunks {
        payload.extend_from_slice(&sub.serialize());
    }

    payload.extend_from_slice(biome_data);

    // Border blocks count
    payload.push(0);

    // Block entities (NBT réseau concaténé, après le border byte).
    if !block_entities.is_empty() {
        payload.extend_from_slice(&crate::block_entities::serialize_network(block_entities));
    }

    payload
}

/// Parse a full chunk payload into sub-chunks and biome data.
/// Returns (Vec<SubChunk>, biome_data_bytes).
pub fn parse_chunk_payload(
    data: &[u8],
    sub_chunk_count: u32,
    air_id: u32,
) -> (Vec<SubChunk>, Vec<u8>) {
    let mut sub_chunks = Vec::with_capacity(sub_chunk_count as usize);
    let mut offset = 0;

    for _ in 0..sub_chunk_count {
        if offset >= data.len() {
            sub_chunks.push(SubChunk::new_air(air_id));
            continue;
        }
        match deserialize_sub_chunk_data(&data[offset..]) {
            Some((sub, consumed)) => {
                sub_chunks.push(sub);
                offset += consumed;
            }
            None => {
                sub_chunks.push(SubChunk::new_air(air_id));
                break;
            }
        }
    }

    // Remaining data is biome sections + border blocks
    let biome_data = if offset < data.len() {
        // Remove the trailing border blocks byte from biome data
        let remaining = &data[offset..];
        if remaining.len() > 1 {
            remaining[..remaining.len() - 1].to_vec()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    (sub_chunks, biome_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sub_chunk() {
        let data = serialize_empty_sub_chunk();
        assert_eq!(data, vec![8, 0]); // version=8, 0 layers
    }

    #[test]
    fn test_single_block_sub_chunk() {
        let blocks = [0u32; 4096]; // all palette index 0
        let palette = vec![42]; // single block type
        let data = serialize_sub_chunk(&blocks, &palette);

        assert_eq!(data[0], 8); // version
        assert_eq!(data[1], 1); // 1 storage layer
        assert_eq!(data[2], 1); // header: bits=0, runtime=1
                                // VarInt32 zigzag(42) = 84 = [0x54]
    }

    #[test]
    fn test_biome_section_plains() {
        let data = serialize_biome_section_single(1); // Plains
        assert_eq!(data[0], 1); // header: bits=0, runtime=1
                                // VarInt32 zigzag(1) = 2 = [0x02]
        assert_eq!(data[1], 0x02);
    }

    #[test]
    fn test_biome_sections_from_columns_not_empty() {
        let mut biome_ids = [[1u32; 16]; 16];
        for row in biome_ids.iter_mut().skip(8) {
            for cell in row.iter_mut() {
                *cell = 2;
            }
        }

        let data = serialize_biome_sections_from_columns(&biome_ids, 24);
        assert!(!data.is_empty());
        assert!(data.len() >= 24 * 2);
    }

    /// Sous-chunk avec herbe marine gorgée d'eau : deux couches à l'écriture, relues à l'identique, et le
    /// sous-chunk suivant reste lisible (le décodeur consomme toutes les couches).
    #[test]
    fn liquid_layer_round_trips() {
        use super::super::block_registry::BLOCKS;
        let mut sub = SubChunk::new_air(BLOCKS.air);
        sub.set_block(3, 4, 5, BLOCKS.get("minecraft:seagrass"));
        sub.set_liquid(3, 4, 5, BLOCKS.water);
        let mut data = sub.serialize();
        assert_eq!(data[1], 2, "deux couches");
        let mut next = SubChunk::new_air(BLOCKS.air);
        next.set_block(0, 0, 0, BLOCKS.stone);
        data.extend_from_slice(&next.serialize());

        let (read, consumed) = deserialize_sub_chunk_data(&data).unwrap();
        assert_eq!(read.get_block(3, 4, 5), BLOCKS.get("minecraft:seagrass"));
        assert_eq!(read.get_liquid(3, 4, 5), BLOCKS.water);
        assert_eq!(read.get_liquid(0, 0, 0), BLOCKS.air);
        let (read_next, _) = deserialize_sub_chunk_data(&data[consumed..]).unwrap();
        assert_eq!(read_next.get_block(0, 0, 0), BLOCKS.stone);
        assert!(read_next.liquid.is_none());

        // Sans eau, une seule couche.
        assert_eq!(next.serialize()[1], 1);
    }

    #[test]
    fn test_multi_palette() {
        // 3 block types → 2 bits per block
        let palette = vec![10, 20, 30];
        let mut blocks = [0u32; 4096];
        blocks[0] = 0; // palette[0] = 10
        blocks[1] = 1; // palette[1] = 20
        blocks[2] = 2; // palette[2] = 30

        let data = serialize_sub_chunk(&blocks, &palette);
        assert_eq!(data[0], 8); // version
        assert_eq!(data[1], 1); // 1 layer
        let header = data[2];
        let bits = (header >> 1) & 0x7F;
        assert_eq!(bits, 2); // 2 bits per block for 3 palette entries
    }
}
