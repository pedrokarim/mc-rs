//! Règles d'attachement / support des blocs — port simplifié des
//! `Block::canBeSupportedAt()` / `onNearbyBlockChange()` de PMMP.
//!
//! Quand un bloc support est cassé, les blocs « attachés » au-dessus ou à côté
//! qui ne peuvent plus rester doivent popper (bambou, canne à sucre, herbe,
//! fleurs, torches, échelles, snow layer, etc.).

/// Règle d'attachement d'un bloc : où doit se trouver son support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentRule {
    /// Nécessite un bloc solide juste en dessous (fleurs, herbe haute, crops,
    /// saplings, torches, rails, pressure plates, signs, snow layer, etc.).
    NeedsBlockBelow,
    /// Nécessite bloc en dessous ET type correct (bambou → dirt/sand/bamboo).
    Bamboo,
    /// Canne à sucre : sugar_cane OU (dirt/grass/sand/podzol + eau adjacente).
    SugarCane,
    /// Cactus : sand OU cactus en dessous + pas de bloc latéral.
    Cactus,
    /// Nécessite un bloc solide au-dessus (hanging_roots, spore_blossom).
    NeedsBlockAbove,
    /// Nécessite au moins un voisin solide (vine, ladder, button, lever —
    /// blocs directionnels dont on ne track pas le facing côté serveur).
    NeedsAnyAdjacentSolid,
    /// Bloc placé sur eau (water_lily / lily_pad).
    OnWater,
    /// Cocoa bean : attaché à un jungle_log adjacent (horizontal).
    Cocoa,
    /// Chorus plant : base sur end_stone OU autre chorus au-dessus/dessous.
    ChorusPlant,
}

/// Retourne la règle d'attachement d'un bloc, ou None s'il n'en a pas
/// (bloc solide standard, pas de check nécessaire).
pub fn attachment_rule(block_name: &str) -> Option<AttachmentRule> {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    // Plantes qui ont juste besoin d'un bloc solide en dessous.
    let below = matches!(
        short,
        // Fleurs + plantes basses
        "red_flower" | "yellow_flower" | "double_plant"
        | "tallgrass" | "tall_grass" | "short_grass"
        | "deadbush" | "fern" | "large_fern"
        | "seagrass"
        | "poppy" | "dandelion" | "blue_orchid" | "allium" | "azure_bluet"
        | "oxeye_daisy" | "cornflower" | "lily_of_the_valley" | "wither_rose"
        | "sunflower" | "lilac" | "rose_bush" | "peony" | "pitcher_plant"
        | "torchflower" | "pink_petals" | "cactus_flower"
        | "wildflowers" | "leaf_litter"
        // Azalea / mushrooms / flower pot
        | "azalea" | "flowering_azalea"
        | "red_mushroom" | "brown_mushroom"
        | "flower_pot"
        // Saplings
        | "sapling"
        | "oak_sapling" | "spruce_sapling" | "birch_sapling"
        | "jungle_sapling" | "acacia_sapling" | "dark_oak_sapling"
        | "mangrove_propagule" | "cherry_sapling" | "pale_oak_sapling"
        | "bamboo_sapling"
        // Crops
        | "wheat" | "potatoes" | "carrots" | "beetroot"
        | "torchflower_crop" | "pitcher_crop"
        | "pumpkin_stem" | "melon_stem"
        | "sweet_berry_bush" | "nether_wart"
        // Nether plants
        | "crimson_fungus" | "warped_fungus"
        | "crimson_roots" | "warped_roots" | "nether_sprouts"
        // Dripleaf
        | "big_dripleaf" | "small_dripleaf_block" | "dripleaf"
        // Candles / cake / campfire / cauldron / bell
        | "candle" | "white_candle" | "orange_candle" | "magenta_candle"
        | "light_blue_candle" | "yellow_candle" | "lime_candle" | "pink_candle"
        | "gray_candle" | "light_gray_candle" | "cyan_candle" | "purple_candle"
        | "blue_candle" | "brown_candle" | "green_candle" | "red_candle" | "black_candle"
        | "cake" | "candle_cake"
        | "white_candle_cake" | "orange_candle_cake" | "magenta_candle_cake"
        | "light_blue_candle_cake" | "yellow_candle_cake" | "lime_candle_cake"
        | "pink_candle_cake" | "gray_candle_cake" | "light_gray_candle_cake"
        | "cyan_candle_cake" | "purple_candle_cake" | "blue_candle_cake"
        | "brown_candle_cake" | "green_candle_cake" | "red_candle_cake" | "black_candle_cake"
        | "campfire" | "soul_campfire"
        | "cauldron" | "lava_cauldron"
        | "bell"
        | "sea_pickle"
        // Attachements solides
        | "torch" | "soul_torch" | "redstone_torch" | "colored_torch_rg"
        | "colored_torch_bp" | "underwater_torch"
        | "snow_layer"
        | "rail" | "golden_rail" | "detector_rail" | "activator_rail"
        | "wooden_pressure_plate" | "stone_pressure_plate"
        | "light_weighted_pressure_plate" | "heavy_weighted_pressure_plate"
        | "polished_blackstone_pressure_plate"
        | "oak_pressure_plate" | "spruce_pressure_plate" | "birch_pressure_plate"
        | "jungle_pressure_plate" | "acacia_pressure_plate" | "dark_oak_pressure_plate"
        | "mangrove_pressure_plate" | "cherry_pressure_plate" | "bamboo_pressure_plate"
        | "crimson_pressure_plate" | "warped_pressure_plate"
        | "unpowered_repeater" | "powered_repeater"
        | "unpowered_comparator" | "powered_comparator"
        | "redstone_wire"
        | "wooden_door" | "iron_door"
        | "spruce_door" | "birch_door" | "jungle_door" | "acacia_door"
        | "dark_oak_door" | "mangrove_door" | "cherry_door" | "bamboo_door"
        | "crimson_door" | "warped_door" | "copper_door"
        | "standing_sign" | "wall_sign" | "hanging_sign"
        | "spruce_standing_sign" | "birch_standing_sign" | "jungle_standing_sign"
        | "acacia_standing_sign" | "darkoak_standing_sign"
        | "mangrove_sign" | "cherry_sign" | "bamboo_sign"
        | "crimson_sign" | "warped_sign"
        | "standing_banner" | "wall_banner"
        | "end_rod" | "lightning_rod"
        | "carpet" | "white_carpet" | "black_carpet" | "red_carpet" | "blue_carpet"
        | "green_carpet" | "yellow_carpet" | "orange_carpet" | "purple_carpet"
        | "pink_carpet" | "light_blue_carpet" | "light_gray_carpet" | "gray_carpet"
        | "brown_carpet" | "cyan_carpet" | "lime_carpet" | "magenta_carpet"
        | "moss_carpet" | "pale_moss_carpet"
        // Beds (2 halves, each needs ground)
        | "bed"
        | "white_bed" | "orange_bed" | "magenta_bed" | "light_blue_bed"
        | "yellow_bed" | "lime_bed" | "pink_bed" | "gray_bed"
        | "light_gray_bed" | "cyan_bed" | "purple_bed" | "blue_bed"
        | "brown_bed" | "green_bed" | "red_bed" | "black_bed"
        // Coral fans (floor) — les "coral_fan" sans "wall_" sont floor
        | "coral_fan" | "coral_fan_dead"
        // Frosted ice — tombe si support en dessous retiré
        | "frosted_ice"
    );
    if below {
        return Some(AttachmentRule::NeedsBlockBelow);
    }

    match short {
        "bamboo" => Some(AttachmentRule::Bamboo),
        "reeds" | "sugar_cane" => Some(AttachmentRule::SugarCane),
        "cactus" => Some(AttachmentRule::Cactus),
        // Blocs directionnels : on n'a pas le facing côté serveur, on se
        // contente de vérifier qu'au moins UN voisin est solide.
        "vine" | "cave_vines" | "cave_vines_body_with_berries"
        | "cave_vines_head_with_berries"
        | "weeping_vines" | "twisting_vines" | "nether_vines"
        | "ladder" | "tripwire_hook"
        | "wooden_button" | "stone_button" | "polished_blackstone_button"
        | "oak_button" | "spruce_button" | "birch_button" | "jungle_button"
        | "acacia_button" | "dark_oak_button" | "mangrove_button" | "cherry_button"
        | "bamboo_button" | "crimson_button" | "warped_button"
        | "lever"
        | "coral_fan_wall" | "coral_fan_hang" | "coral_fan_hang2" | "coral_fan_hang3"
        | "amethyst_cluster" | "small_amethyst_bud" | "medium_amethyst_bud"
        | "large_amethyst_bud"
        | "item_frame" | "glow_item_frame"
        | "painting"
        // Lantern : center-support dessus ou dessous (hanging ou posé).
        // En l'absence du bit `hanging` on tolère l'un ou l'autre support.
        | "lantern" | "soul_lantern"
            => Some(AttachmentRule::NeedsAnyAdjacentSolid),

        // Blocs suspendus (attach au-dessus).
        "spore_blossom" | "hanging_roots" | "pointed_dripstone" => {
            Some(AttachmentRule::NeedsBlockAbove)
        }

        // Blocs posés sur l'eau.
        "waterlily" | "lily_pad" => Some(AttachmentRule::OnWater),

        // Cocoa bean sur jungle_log adjacent horizontal.
        "cocoa" => Some(AttachmentRule::Cocoa),

        // Chorus plant / flower — support multi-face (end_stone OU chorus).
        "chorus_plant" | "chorus_flower" => Some(AttachmentRule::ChorusPlant),

        _ => None,
    }
}

/// Teste si un bloc peut servir de support (solide) — approx PMMP
/// `Block::isSolid()`. On considère l'air et les plantes comme non-solide.
pub fn is_solid_support(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    if short.is_empty() || short == "air" {
        return false;
    }
    // Liste non-exhaustive d'éléments non-solides (pour ne pas gêner les
    // attachements). Un bloc inconnu est supposé solide par défaut.
    !matches!(
        short,
        "air"
            | "water"
            | "flowing_water"
            | "lava"
            | "flowing_lava"
            | "fire"
            | "soul_fire"
            | "torch"
            | "soul_torch"
            | "redstone_torch"
            | "red_flower"
            | "yellow_flower"
            | "sapling"
            | "tallgrass"
            | "deadbush"
            | "double_plant"
            | "bamboo"
            | "reeds"
            | "sugar_cane"
            | "cactus"
            | "snow_layer"
            | "wheat"
            | "carrots"
            | "potatoes"
            | "beetroot"
            | "vine"
            | "ladder"
            | "rail"
    )
}

/// Pour un bloc avec `SugarCane`, sand/dirt/podzol/grass_block/mud suffit
/// si une eau adjacente est présente (détection au call site).
pub fn is_valid_sugar_cane_ground(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    matches!(
        short,
        "dirt"
            | "coarse_dirt"
            | "rooted_dirt"
            | "grass_block"
            | "podzol"
            | "mud"
            | "sand"
            | "red_sand"
            | "reeds"
            | "sugar_cane"
    )
}

/// Pour un bloc bambou, le support valide (cf Allay
/// `BlockBambooSaplingBaseComponentImpl::canSupportBamboo`) : dirt-tag,
/// sand-tag, bamboo[_sapling], moss_block, podzol, gravel.
pub fn is_valid_bamboo_ground(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    matches!(
        short,
        "bamboo"
            | "bamboo_sapling"
            | "dirt"
            | "coarse_dirt"
            | "rooted_dirt"
            | "grass_block"
            | "podzol"
            | "mud"
            | "sand"
            | "red_sand"
            | "moss_block"
            | "gravel"
    )
}

/// Pour le cactus : sand/red_sand OU cactus en dessous.
pub fn is_valid_cactus_ground(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    matches!(short, "sand" | "red_sand" | "cactus")
}

/// Water (still ou flowing) pour water_lily.
pub fn is_water(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    matches!(short, "water" | "flowing_water")
}

/// Jungle log (toutes variantes) pour cocoa.
pub fn is_jungle_log(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    matches!(
        short,
        "jungle_log" | "stripped_jungle_log" | "jungle_wood" | "stripped_jungle_wood"
    )
}

/// End stone ou chorus plant (pour base chorus).
pub fn is_chorus_support(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    matches!(short, "end_stone" | "chorus_plant")
}

/// Blocs marqués `minecraft:replaceable` dans Allay / PMMP : ils
/// peuvent être overridés quand on place un autre bloc dessus.
/// Source canonique : `.reference/Allay/data/resources/block_tags_custom.json`.
pub fn is_replaceable(block_name: &str) -> bool {
    let short = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    matches!(
        short,
        "air"
            | "bubble_column"
            | "crimson_roots"
            | "deadbush"
            | "fern"
            | "fire"
            | "flowing_lava"
            | "flowing_water"
            | "glow_lichen"
            | "large_fern"
            | "lava"
            | "leaf_litter"
            | "nether_sprouts"
            | "pink_petals"
            | "resin_clump"
            | "sculk_vein"
            | "seagrass"
            | "short_grass"
            | "snow_layer"
            | "soul_fire"
            | "tall_grass"
            | "tallgrass"
            | "vine"
            | "warped_roots"
            | "water"
            | "wildflowers"
            | "light_block_0"
            | "light_block_1"
            | "light_block_2"
            | "light_block_3"
            | "light_block_4"
            | "light_block_5"
            | "light_block_6"
            | "light_block_7"
            | "light_block_8"
            | "light_block_9"
            | "light_block_10"
            | "light_block_11"
            | "light_block_12"
            | "light_block_13"
            | "light_block_14"
            | "light_block_15"
    )
}

/// Vérifie si le bloc à (x,y,z) a toujours son support selon la règle.
/// Le cache est fourni pour lire les blocs voisins.
pub fn check_support(
    cache: &mut crate::world::chunk_cache::ChunkCache,
    x: i32,
    y: i32,
    z: i32,
    rule: AttachmentRule,
) -> bool {
    let name_at =
        |c: &mut crate::world::chunk_cache::ChunkCache, x: i32, y: i32, z: i32| -> String {
            let id = c.get_block(x, y, z);
            crate::world::block_registry::BLOCKS
                .name_for(id)
                .unwrap_or("")
                .to_string()
        };

    match rule {
        AttachmentRule::NeedsBlockBelow => {
            let below = name_at(cache, x, y - 1, z);
            is_solid_support(&below)
        }
        AttachmentRule::Bamboo => {
            let below = name_at(cache, x, y - 1, z);
            is_valid_bamboo_ground(&below)
        }
        AttachmentRule::SugarCane => {
            let below = name_at(cache, x, y - 1, z);
            if below == "minecraft:reeds" || below == "minecraft:sugar_cane" {
                true
            } else if is_valid_sugar_cane_ground(&below) {
                // Eau dans les 4 directions horizontales du bloc en dessous.
                [
                    (x + 1, y - 1, z),
                    (x - 1, y - 1, z),
                    (x, y - 1, z + 1),
                    (x, y - 1, z - 1),
                ]
                .iter()
                .any(|(wx, wy, wz)| {
                    let wname = name_at(cache, *wx, *wy, *wz);
                    is_water(&wname) || wname == "minecraft:frosted_ice"
                })
            } else {
                false
            }
        }
        AttachmentRule::Cactus => {
            let below = name_at(cache, x, y - 1, z);
            if !is_valid_cactus_ground(&below) {
                return false;
            }
            // Pas de bloc solide latéral (sinon cactus push out).
            ![(x + 1, y, z), (x - 1, y, z), (x, y, z + 1), (x, y, z - 1)]
                .iter()
                .any(|(lx, ly, lz)| {
                    let name = name_at(cache, *lx, *ly, *lz);
                    is_solid_support(&name)
                })
        }
        AttachmentRule::NeedsBlockAbove => {
            let above = name_at(cache, x, y + 1, z);
            is_solid_support(&above)
        }
        AttachmentRule::NeedsAnyAdjacentSolid => [
            (x, y + 1, z),
            (x, y - 1, z),
            (x + 1, y, z),
            (x - 1, y, z),
            (x, y, z + 1),
            (x, y, z - 1),
        ]
        .iter()
        .any(|(ax, ay, az)| {
            let name = name_at(cache, *ax, *ay, *az);
            is_solid_support(&name)
        }),
        AttachmentRule::OnWater => {
            let below = name_at(cache, x, y - 1, z);
            is_water(&below)
        }
        AttachmentRule::Cocoa => {
            // Jungle log horizontalement adjacent.
            [(x + 1, y, z), (x - 1, y, z), (x, y, z + 1), (x, y, z - 1)]
                .iter()
                .any(|(jx, jy, jz)| {
                    let name = name_at(cache, *jx, *jy, *jz);
                    is_jungle_log(&name)
                })
        }
        AttachmentRule::ChorusPlant => {
            // Support = end_stone dessous OU chorus adjacent (haut/bas/horiz).
            let below = name_at(cache, x, y - 1, z);
            if below == "minecraft:end_stone" {
                return true;
            }
            [
                (x, y + 1, z),
                (x, y - 1, z),
                (x + 1, y, z),
                (x - 1, y, z),
                (x, y, z + 1),
                (x, y, z - 1),
            ]
            .iter()
            .any(|(cx, cy, cz)| {
                let name = name_at(cache, *cx, *cy, *cz);
                is_chorus_support(&name)
            })
        }
    }
}
