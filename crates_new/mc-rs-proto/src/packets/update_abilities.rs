use bytes::{BufMut, BytesMut};

// Layer IDs
const LAYER_BASE: u16 = 1;

// Ability bit positions
const BUILD: u32 = 1 << 0;
const MINE: u32 = 1 << 1;
const DOORS_AND_SWITCHES: u32 = 1 << 2;
const OPEN_CONTAINERS: u32 = 1 << 3;
const ATTACK_PLAYERS: u32 = 1 << 4;
const ATTACK_MOBS: u32 = 1 << 5;
const OPERATOR: u32 = 1 << 6;
const TELEPORT: u32 = 1 << 7;
const INVULNERABLE: u32 = 1 << 8;
const FLYING: u32 = 1 << 9;
const ALLOW_FLIGHT: u32 = 1 << 10;
const INFINITE_RESOURCES: u32 = 1 << 11;
const LIGHTNING: u32 = 1 << 12;
const FLY_SPEED: u32 = 1 << 13;
const WALK_SPEED: u32 = 1 << 14;
const MUTED: u32 = 1 << 15;
const WORLD_BUILDER: u32 = 1 << 16;
const NO_CLIP: u32 = 1 << 17;
const VERTICAL_FLY_SPEED: u32 = 1 << 19;

// Permission levels
const PLAYER_PERMISSION_MEMBER: u8 = 1;
const COMMAND_PERMISSION_NORMAL: u8 = 0;

/// UpdateAbilities packet matching PocketMine syncAbilities() for a normal creative player:
/// one BASE layer, with all ability bits set and values toggled per PMMP defaults.
pub fn encode_default_creative(actor_unique_id: i64) -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_i64_le(actor_unique_id); // targetActorUniqueId (signed long LE)
    buf.put_u8(PLAYER_PERMISSION_MEMBER);
    buf.put_u8(COMMAND_PERMISSION_NORMAL);
    buf.put_u8(1); // layer count

    // PMMP sets all ability bits (0..19) in this BASE layer.
    let base_set = 0x000F_FFFF;
    let base_values = BUILD
        | MINE
        | DOORS_AND_SWITCHES
        | OPEN_CONTAINERS
        | ATTACK_PLAYERS
        | ATTACK_MOBS
        | INVULNERABLE
        | ALLOW_FLIGHT
        | INFINITE_RESOURCES;
    write_layer(&mut buf, LAYER_BASE, base_set, base_values, 0.05, 1.0, 0.1);

    buf
}

/// UpdateAbilities packet matching PocketMine syncAbilities() for a normal survival player.
/// Uses a single BASE layer with PMMP-like defaults.
pub fn encode_default_survival(actor_unique_id: i64) -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_i64_le(actor_unique_id); // targetActorUniqueId (signed long LE)
    buf.put_u8(PLAYER_PERMISSION_MEMBER);
    buf.put_u8(COMMAND_PERMISSION_NORMAL);
    buf.put_u8(1); // layer count

    // PMMP still sets all bits in BASE; false values are carried in abilityValues.
    let base_set = 0x000F_FFFF;
    let base_values = BUILD
        | MINE
        | DOORS_AND_SWITCHES
        | OPEN_CONTAINERS
        | ATTACK_PLAYERS
        | ATTACK_MOBS;
    write_layer(&mut buf, LAYER_BASE, base_set, base_values, 0.05, 1.0, 0.1);

    buf
}

fn write_layer(
    buf: &mut BytesMut,
    layer_id: u16,
    abilities_set: u32,
    ability_values: u32,
    fly_speed: f32,
    vertical_fly_speed: f32,
    walk_speed: f32,
) {
    buf.put_u16_le(layer_id);
    buf.put_u32_le(abilities_set);
    buf.put_u32_le(ability_values);
    buf.put_f32_le(fly_speed);
    buf.put_f32_le(vertical_fly_speed);
    buf.put_f32_le(walk_speed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_pmmp_single_base_layer_layout() {
        let buf = encode_default_creative(1);
        assert_eq!(buf.len(), 33, "unexpected UpdateAbilities wire size");

        // Header: i64 + playerPerm + cmdPerm + layerCount
        assert_eq!(buf[8], PLAYER_PERMISSION_MEMBER);
        assert_eq!(buf[9], COMMAND_PERMISSION_NORMAL);
        assert_eq!(buf[10], 1);

        let set = u32::from_le_bytes([buf[13], buf[14], buf[15], buf[16]]);
        let values = u32::from_le_bytes([buf[17], buf[18], buf[19], buf[20]]);
        assert_eq!(set, 0x000F_FFFF);
        assert_eq!(values, 0x0000_0D3F);

        // Layer 0 starts at 11. Fly speed triplet is at offsets 21..32.
        let fly = f32::from_le_bytes([buf[21], buf[22], buf[23], buf[24]]);
        let vertical = f32::from_le_bytes([buf[25], buf[26], buf[27], buf[28]]);
        let walk = f32::from_le_bytes([buf[29], buf[30], buf[31], buf[32]]);
        assert_eq!(fly, 0.05);
        assert_eq!(vertical, 1.0);
        assert_eq!(walk, 0.1);
    }

    #[test]
    fn encodes_survival_values() {
        let buf = encode_default_survival(1);
        assert_eq!(buf.len(), 33, "unexpected UpdateAbilities wire size");

        let set = u32::from_le_bytes([buf[13], buf[14], buf[15], buf[16]]);
        let values = u32::from_le_bytes([buf[17], buf[18], buf[19], buf[20]]);
        assert_eq!(set, 0x000F_FFFF);
        assert_eq!(values, 0x0000_003F);
    }
}
