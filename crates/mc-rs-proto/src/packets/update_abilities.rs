//! UpdateAbilities (0xBB) — Server → Client.
//!
//! Updates the player's ability flags (fly, instabuild, etc.).
//! Sent when gamemode changes or when operator status changes.

use bytes::BufMut;

use crate::codec::ProtoEncode;

/// Update a player's ability data.
pub struct UpdateAbilities {
    pub command_permission_level: u8,
    pub permission_level: u8,
    pub entity_unique_id: i64,
    /// Used to compute ability values bitmask.
    pub gamemode: i32,
}

impl ProtoEncode for UpdateAbilities {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        // AbilitiesData wire format:
        // targetActorUniqueId (i64 LE), playerPermission (u8), commandPermission (u8), layerCount (u8), layers...
        buf.put_i64_le(self.entity_unique_id);
        buf.put_u8(self.permission_level);
        buf.put_u8(self.command_permission_level);

        // One base layer.
        buf.put_u8(1);
        buf.put_u16_le(1); // LAYER_BASE

        // Abilities bit indices (BedrockProtocol AbilitiesLayer constants).
        const ABILITY_BUILD: u32 = 0;
        const ABILITY_MINE: u32 = 1;
        const ABILITY_DOORS_AND_SWITCHES: u32 = 2;
        const ABILITY_OPEN_CONTAINERS: u32 = 3;
        const ABILITY_ATTACK_PLAYERS: u32 = 4;
        const ABILITY_ATTACK_MOBS: u32 = 5;
        const ABILITY_OPERATOR: u32 = 6;
        const ABILITY_TELEPORT: u32 = 7;
        const ABILITY_INVULNERABLE: u32 = 8;
        const ABILITY_FLYING: u32 = 9;
        const ABILITY_ALLOW_FLIGHT: u32 = 10;
        const ABILITY_INFINITE_RESOURCES: u32 = 11;
        const ABILITY_LIGHTNING: u32 = 12;
        const ABILITY_FLY_SPEED: u32 = 13;
        const ABILITY_WALK_SPEED: u32 = 14;
        const ABILITY_MUTED: u32 = 15;
        const ABILITY_WORLD_BUILDER: u32 = 16;
        const ABILITY_NO_CLIP: u32 = 17;
        const ABILITY_PRIVILEGED_BUILDER: u32 = 18;
        const ABILITY_VERTICAL_FLY_SPEED: u32 = 19;

        let is_creative_or_spectator = matches!(self.gamemode, 1 | 3);
        let is_spectator = self.gamemode == 3;
        let can_build = !is_spectator;
        let is_op = self.permission_level >= 2 || self.command_permission_level >= 1;

        let mut set_abilities = 0u32;
        let mut set_values = 0u32;
        let set_bool = |set_abilities: &mut u32, set_values: &mut u32, bit: u32, value: bool| {
            *set_abilities |= 1 << bit;
            if value {
                *set_values |= 1 << bit;
            }
        };

        // Mirror PMMP-style base layer fields so the client gets a complete ability set.
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_ALLOW_FLIGHT,
            is_creative_or_spectator,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_FLYING,
            is_spectator,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_NO_CLIP,
            is_spectator,
        );
        set_bool(&mut set_abilities, &mut set_values, ABILITY_OPERATOR, is_op);
        set_bool(&mut set_abilities, &mut set_values, ABILITY_TELEPORT, is_op);
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_INVULNERABLE,
            is_creative_or_spectator,
        );
        set_bool(&mut set_abilities, &mut set_values, ABILITY_MUTED, false);
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_WORLD_BUILDER,
            false,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_INFINITE_RESOURCES,
            is_creative_or_spectator,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_LIGHTNING,
            false,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_BUILD,
            can_build,
        );
        set_bool(&mut set_abilities, &mut set_values, ABILITY_MINE, can_build);
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_DOORS_AND_SWITCHES,
            can_build,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_OPEN_CONTAINERS,
            can_build,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_ATTACK_PLAYERS,
            can_build,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_ATTACK_MOBS,
            can_build,
        );
        set_bool(
            &mut set_abilities,
            &mut set_values,
            ABILITY_PRIVILEGED_BUILDER,
            false,
        );

        // Speed flags are represented by dedicated bits + float payloads.
        set_abilities |= 1 << ABILITY_FLY_SPEED;
        set_abilities |= 1 << ABILITY_WALK_SPEED;
        set_abilities |= 1 << ABILITY_VERTICAL_FLY_SPEED;

        buf.put_u32_le(set_abilities);
        buf.put_u32_le(set_values);
        buf.put_f32_le(0.05); // fly speed
        buf.put_f32_le(1.0); // vertical fly speed
        buf.put_f32_le(0.1); // walk speed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_creative_abilities() {
        let pkt = UpdateAbilities {
            command_permission_level: 0,
            permission_level: 1,
            entity_unique_id: 1,
            gamemode: 1,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // i64 + player_perm + cmd_perm + layer_count + layer(u16+u32+u32+f32+f32+f32)
        assert_eq!(buf.len(), 33);
        let values = u32::from_le_bytes([buf[17], buf[18], buf[19], buf[20]]);
        // Creative should at least allow build + mayfly + instabuild.
        assert_ne!(values & (1 << 0), 0); // build
        assert_ne!(values & (1 << 10), 0); // allow flight
        assert_ne!(values & (1 << 11), 0); // infinite resources
    }

    #[test]
    fn encode_survival_abilities() {
        let pkt = UpdateAbilities {
            command_permission_level: 0,
            permission_level: 1,
            entity_unique_id: 1,
            gamemode: 0,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let values = u32::from_le_bytes([buf[17], buf[18], buf[19], buf[20]]);
        assert_ne!(values & (1 << 0), 0); // build
        assert_eq!(values & (1 << 10), 0); // allow flight off
        assert_eq!(values & (1 << 11), 0); // infinite resources off
    }
}
