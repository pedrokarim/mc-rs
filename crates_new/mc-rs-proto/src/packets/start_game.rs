use crate::codec::*;
use bytes::{BufMut, BytesMut};

/// Encode StartGamePacket matching PocketMine-MP exactly.
/// Every field is in the exact order from BedrockProtocol/StartGamePacket.php.
pub fn encode(
    actor_unique_id: i64,
    actor_runtime_id: u64,
    gamemode: i32,
    position: [f32; 3],
    pitch: f32,
    yaw: f32,
    world_time: i32,
    spawn_position: [i32; 3],
    world_name: &str,
) -> BytesMut {
    let mut buf = BytesMut::with_capacity(512);

    // --- StartGamePacket fields ---
    write_signed_varlong(&mut buf, actor_unique_id); // #1 actorUniqueId
    write_unsigned_varlong(&mut buf, actor_runtime_id); // #2 actorRuntimeId
    write_signed_varint32(&mut buf, gamemode); // #3 playerGamemode
    write_vec3f(&mut buf, position[0], position[1], position[2]); // #4 playerPosition
    buf.put_f32_le(pitch); // #5 pitch
    buf.put_f32_le(yaw); // #6 yaw

    // --- LevelSettings (inline) ---
    buf.put_u64_le(u64::MAX); // #7 seed (PocketMine uses -1 = u64::MAX)
    buf.put_u16_le(0); // #8 spawnSettings.biomeType
    write_string(&mut buf, ""); // #9 spawnSettings.biomeName
    write_signed_varint32(&mut buf, 0); // #10 spawnSettings.dimension (Overworld)
    write_signed_varint32(&mut buf, 1); // #11 generator (OVERWORLD=1)
    write_signed_varint32(&mut buf, gamemode); // #12 worldGamemode
    buf.put_u8(0); // #13 hardcore
    write_signed_varint32(&mut buf, 2); // #14 difficulty (Normal)
    write_block_pos(
        &mut buf,
        spawn_position[0],
        spawn_position[1] as u32,
        spawn_position[2],
    ); // #15 spawnPosition
    buf.put_u8(1); // #16 hasAchievementsDisabled
    write_signed_varint32(&mut buf, 0); // #17 editorWorldType (NON_EDITOR)
    buf.put_u8(0); // #18 createdInEditorMode
    buf.put_u8(0); // #19 exportedFromEditorMode
    write_signed_varint32(&mut buf, world_time); // #20 time
    write_signed_varint32(&mut buf, 0); // #21 eduEditionOffer
    buf.put_u8(0); // #22 hasEduFeaturesEnabled
    write_string(&mut buf, ""); // #23 eduProductUUID
    buf.put_f32_le(0.0); // #24 rainLevel
    buf.put_f32_le(0.0); // #25 lightningLevel
    buf.put_u8(0); // #26 hasConfirmedPlatformLockedContent
    buf.put_u8(1); // #27 isMultiplayerGame
    buf.put_u8(1); // #28 hasLANBroadcast
    write_signed_varint32(&mut buf, 4); // #29 xboxLiveBroadcastMode (PUBLIC=4)
    write_signed_varint32(&mut buf, 4); // #30 platformBroadcastMode (PUBLIC=4)
    buf.put_u8(1); // #31 commandsEnabled
    buf.put_u8(1); // #32 isTexturePacksRequired

    // #33 gameRules — PocketMine sends 2 rules
    write_unsigned_varint32(&mut buf, 2);
    // Rule 1: naturalregeneration = false
    write_string(&mut buf, "naturalregeneration");
    buf.put_u8(0); // isPlayerModifiable
    write_unsigned_varint32(&mut buf, 1); // type=Bool
    buf.put_u8(0); // value=false
                   // Rule 2: locatorbar = false
    write_string(&mut buf, "locatorbar");
    buf.put_u8(0);
    write_unsigned_varint32(&mut buf, 1);
    buf.put_u8(0);

    // #34 experiments
    buf.put_u32_le(0); // count (u32_le)
    buf.put_u8(0); // hasPreviouslyUsedExperiments

    buf.put_u8(0); // #35 hasBonusChestEnabled
    buf.put_u8(0); // #36 hasStartWithMapEnabled
    write_signed_varint32(&mut buf, 1); // #37 defaultPlayerPermission (MEMBER)
    buf.put_i32_le(4); // #38 serverChunkTickRadius
    buf.put_u8(0); // #39 hasLockedBehaviorPack
    buf.put_u8(0); // #40 hasLockedResourcePack
    buf.put_u8(0); // #41 isFromLockedWorldTemplate
    buf.put_u8(0); // #42 useMsaGamertagsOnly
    buf.put_u8(0); // #43 isFromWorldTemplate
    buf.put_u8(0); // #44 isWorldTemplateOptionLocked
    buf.put_u8(0); // #45 onlySpawnV1Villagers
    buf.put_u8(0); // #46 disablePersona
    buf.put_u8(0); // #47 disableCustomSkins
    buf.put_u8(0); // #48 muteEmoteAnnouncements
    write_string(&mut buf, "1.26.0"); // #49 vanillaVersion
    buf.put_i32_le(0); // #50 limitedWorldWidth
    buf.put_i32_le(0); // #51 limitedWorldLength
    buf.put_u8(1); // #52 isNewNether
    write_string(&mut buf, ""); // #53 eduSharedUriResource.buttonName
    write_string(&mut buf, ""); // #54 eduSharedUriResource.linkUri
    buf.put_u8(0); // #55 experimentalGameplayOverride = None
    buf.put_u8(0); // #56 chatRestrictionLevel (NONE)
    buf.put_u8(0); // #57 disablePlayerInteractions

    // --- End LevelSettings, back to StartGame ---

    write_string(&mut buf, ""); // #58 levelId
    write_string(&mut buf, world_name); // #59 worldName
    write_string(&mut buf, ""); // #60 premiumWorldTemplateId
    buf.put_u8(0); // #61 isTrial

    // #62-63 playerMovementSettings
    write_signed_varint32(&mut buf, 0); // rewindHistorySize
    buf.put_u8(1); // serverAuthoritativeBlockBreaking

    buf.put_u64_le(0); // #64 currentTick
    write_signed_varint32(&mut buf, 0); // #65 enchantmentSeed

    // #66 blockPalette — EMPTY (PocketMine sends [])
    write_unsigned_varint32(&mut buf, 0);

    write_string(&mut buf, ""); // #67 multiplayerCorrelationId
    buf.put_u8(1); // #68 enableNewInventorySystem
    write_string(&mut buf, "mc-rs 0.1.0"); // #69 serverSoftwareVersion

    // #70 playerActorProperties — empty NBT Compound (network LE)
    // TAG_Compound(0x0A) + name_len(0x00, 0x00 as VarInt) + TAG_End(0x00)
    buf.put_u8(0x0A); // Compound tag
    buf.put_u8(0x00); // name length (VarUInt = 0)
    buf.put_u8(0x00); // TAG_End

    buf.put_u64_le(0); // #71 blockPaletteChecksum

    // #72 worldTemplateId (UUID, 16 bytes of zeros)
    buf.put_slice(&[0u8; 16]);

    buf.put_u8(0); // #73 enableClientSideChunkGeneration
    buf.put_u8(0); // #74 blockNetworkIdsAreHashes = FALSE

    // #75 networkPermissions
    buf.put_u8(1); // disableClientSounds = true

    // #76 serverJoinInformation = None
    buf.put_u8(0);

    // #77-80 serverTelemetryData (4 empty strings)
    write_string(&mut buf, ""); // serverId
    write_string(&mut buf, ""); // scenarioId
    write_string(&mut buf, ""); // worldId
    write_string(&mut buf, ""); // ownerId

    buf
}
