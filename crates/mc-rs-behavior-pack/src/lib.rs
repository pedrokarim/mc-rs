//! Bedrock behavior pack parser.
//!
//! Parses behavior pack JSON files (manifest, entities, items, blocks, recipes,
//! loot tables) and provides a loader that scans a pack directory.

pub mod block;
pub mod entity;
pub mod item;
pub mod loader;
pub mod loot_table;
pub mod manifest;
pub mod recipe;

pub use loader::{load_all_packs, LoadedBehaviorPack};
