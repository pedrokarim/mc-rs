//! Seaux d'eau et de lave : PMMP `item/Bucket.php` (remplir) et `item/LiquidBucket.php` (vider), avec le
//! waterlogging de dragonfly (`item/bucket.go`) que PMMP ne gère pas : un seau d'eau vidé sur un bloc qui peut
//! contenir de l'eau la met dans sa seconde couche, un seau vide reprend l'eau d'un bloc gorgé d'eau.
//! L'écoulement qui suit est l'affaire de `liquid_flow` (le changement de bloc le déclenche).

use tracing::info;

use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::{ItemInteractionData, ItemStack};
use mc_rs_proto::packets::world::LevelSoundEvent;

use crate::liquid_flow::{Kind, Liquid};
use crate::world::block_registry::BLOCKS;

use super::Connection;

/// Contenu d'un seau.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Content {
    Empty,
    Liquid(Kind),
}

fn content_of(item_name: &str) -> Option<Content> {
    match item_name {
        "minecraft:bucket" => Some(Content::Empty),
        "minecraft:water_bucket" => Some(Content::Liquid(Kind::Water)),
        "minecraft:lava_bucket" => Some(Content::Liquid(Kind::Lava)),
        _ => None,
    }
}

fn bucket_name(content: Content) -> &'static str {
    match content {
        Content::Empty => "minecraft:bucket",
        Content::Liquid(Kind::Water) => "minecraft:water_bucket",
        Content::Liquid(Kind::Lava) => "minecraft:lava_bucket",
    }
}

impl Connection {
    /// Vrai si l'objet tenu est un seau géré ici (il a alors été utilisé, ou l'action est sans effet).
    pub(super) fn try_use_bucket(
        &mut self,
        interaction: &ItemInteractionData,
        responses: &mut Vec<Vec<u8>>,
    ) -> bool {
        let held = &self.inventory.slots[self.inventory.held_slot as usize].item;
        let Some(content) = crate::item_registry::item_name_by_id(held.id).and_then(content_of)
        else {
            return false;
        };
        let clicked = interaction.block_position;
        match content {
            Content::Empty => self.fill_bucket(clicked, responses),
            Content::Liquid(kind) => self.empty_bucket(interaction, kind, responses),
        }
        true
    }

    /// PMMP `Bucket::onInteractBlock` : une source de liquide (ou l'eau d'un bloc gorgé d'eau) passe dans le
    /// seau.
    fn fill_bucket(&mut self, pos: [i32; 3], responses: &mut Vec<Vec<u8>>) {
        let Ok(mut cache) = self.chunk_cache.lock() else {
            return;
        };
        let block = cache.get_block(pos[0], pos[1], pos[2]);
        let waterlogged = Liquid::from_id(cache.get_liquid(pos[0], pos[1], pos[2]));
        let (kind, layer) = match (Liquid::from_id(block), waterlogged) {
            (Some(l), _) if l.is_source() => (l.kind, 0),
            (None, Some(l)) if l.is_source() => (l.kind, 1),
            _ => return,
        };
        if layer == 0 {
            cache.set_block(pos[0], pos[1], pos[2], BLOCKS.air);
        } else {
            cache.set_liquid(pos[0], pos[1], pos[2], BLOCKS.air);
        }
        drop(cache);
        self.broadcast_block_layer(pos, BLOCKS.air, layer, responses);
        let sound = match kind {
            Kind::Water => LevelSoundEvent::BUCKET_FILL_WATER,
            Kind::Lava => LevelSoundEvent::BUCKET_FILL_LAVA,
        };
        self.broadcast_bucket_sound(sound, pos, responses);
        self.swap_held_bucket(Content::Liquid(kind));
        info!("[{}] Seau rempli ({:?}) en {:?}", self.addr, kind, pos);
    }

    /// PMMP `LiquidBucket::onInteractBlock` : le liquide est posé (forme qui coule, source) à la place du bloc
    /// cliqué s'il est remplaçable, sinon à côté selon la face ; l'eau entre dans un bloc qui peut la contenir.
    fn empty_bucket(
        &mut self,
        interaction: &ItemInteractionData,
        kind: Kind,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let clicked = interaction.block_position;
        let side = match interaction.face {
            0 => [0, -1, 0],
            1 => [0, 1, 0],
            2 => [0, 0, -1],
            3 => [0, 0, 1],
            4 => [-1, 0, 0],
            5 => [1, 0, 0],
            _ => return,
        };
        let adjacent = [
            clicked[0] + side[0],
            clicked[1] + side[1],
            clicked[2] + side[2],
        ];
        let source = Liquid {
            kind,
            still: false,
            decay: 0,
            falling: false,
        }
        .id();
        let Ok(mut cache) = self.chunk_cache.lock() else {
            return;
        };
        let accepts_water = |cache: &mut crate::world::chunk_cache::ChunkCache, p: [i32; 3]| {
            kind == Kind::Water
                && crate::block_liquid::can_contain_liquid(cache.get_block(p[0], p[1], p[2]))
                && cache.get_block(p[0], p[1], p[2]) != BLOCKS.air
                && cache.get_liquid(p[0], p[1], p[2]) == BLOCKS.air
        };
        let replaceable = |cache: &mut crate::world::chunk_cache::ChunkCache, p: [i32; 3]| {
            let id = cache.get_block(p[0], p[1], p[2]);
            crate::block_attachment::is_replaceable(BLOCKS.name_for(id).unwrap_or(""))
        };
        // Ordre de dragonfly : bloc cliqué (gorgé d'eau ou remplacé), puis bloc voisin.
        let (pos, layer) = if accepts_water(&mut cache, clicked) {
            (clicked, 1)
        } else if replaceable(&mut cache, clicked) {
            (clicked, 0)
        } else if accepts_water(&mut cache, adjacent) {
            (adjacent, 1)
        } else if replaceable(&mut cache, adjacent) {
            (adjacent, 0)
        } else {
            return;
        };
        if layer == 1 {
            cache.set_liquid(pos[0], pos[1], pos[2], source);
        } else {
            cache.set_block(pos[0], pos[1], pos[2], source);
        }
        drop(cache);
        self.broadcast_block_layer(pos, source, layer, responses);
        let sound = match kind {
            Kind::Water => LevelSoundEvent::BUCKET_EMPTY_WATER,
            Kind::Lava => LevelSoundEvent::BUCKET_EMPTY_LAVA,
        };
        self.broadcast_bucket_sound(sound, pos, responses);
        self.swap_held_bucket(Content::Empty);
        info!(
            "[{}] Seau vidé ({:?}) en {:?}, couche {}",
            self.addr, kind, pos, layer
        );
    }

    fn broadcast_bucket_sound(
        &mut self,
        sound: &'static str,
        pos: [i32; 3],
        responses: &mut Vec<Vec<u8>>,
    ) {
        let center = [
            pos[0] as f32 + 0.5,
            pos[1] as f32 + 0.5,
            pos[2] as f32 + 0.5,
        ];
        let payload = LevelSoundEvent::world_sound(sound, center).encode();
        responses.push(self.encode_compressed_packet(packet_id::LEVEL_SOUND_EVENT, &payload));
        self.broadcasts
            .push((packet_id::LEVEL_SOUND_EVENT, payload));
    }

    /// Survie : un seau du tas tenu devient `content` (PMMP `pop()` + objet rendu) ; en créatif, rien ne change.
    fn swap_held_bucket(&mut self, content: Content) {
        if self.gamemode != 0 {
            return;
        }
        let Some(new_id) = crate::item_registry::network_id(bucket_name(content)) else {
            return;
        };
        let slot = self.inventory.held_slot as usize;
        let held = self.inventory.slots[slot].item.clone();
        let filled = ItemStack {
            id: new_id,
            count: 1,
            meta: 0,
            ..held.clone()
        };
        if held.count <= 1 {
            self.inventory_manager.set_slot(
                &mut self.inventory,
                crate::inventory_manager::InvKey::Main,
                slot,
                filled,
            );
            return;
        }
        let mut rest = held;
        rest.count -= 1;
        self.inventory_manager.set_slot(
            &mut self.inventory,
            crate::inventory_manager::InvKey::Main,
            slot,
            rest,
        );
        if !self
            .inventory_manager
            .add_item_to_main(&mut self.inventory, filled.clone())
        {
            // Inventaire plein : l'objet tombe aux pieds du joueur (PMMP `dropItem`).
            self.pending_item_spawns.push(
                crate::item_entities::PendingItemEntitySpawn::with_scatter(filled, self.position),
            );
        }
    }
}
