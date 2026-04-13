# AI Handoff

Date: 2026-04-05
Repo: `mc-rs`
Target Bedrock version: `1.26.10`
Protocol: `944`

## Goal

This project is a Rust Bedrock server aiming to get closer to official Bedrock behavior while keeping a PocketMine-inspired structure where useful.

The most important medium-term goals are:

- stable survival gameplay
- correct world persistence
- a usable entity system
- better Bedrock-like world generation

## Current Repo State

There is uncommitted work in progress.

Current `git status --short` at handoff time:

```text
 M crates/mc-rs-proto/src/packets/player.rs
 M crates/mc-rs-server/src/commands.rs
 D crates/mc-rs-server/src/connection.rs
 M crates/mc-rs-server/src/entity.rs
 M crates/mc-rs-server/src/item_entities.rs
 M crates/mc-rs-server/src/item_registry.rs
 M crates/mc-rs-server/src/main.rs
 M crates/mc-rs-server/src/mob_entities.rs
?? crates/mc-rs-server/src/connection/
```

Important note:

- `crates/mc-rs-server/src/connection.rs` has been split into a folder-based module tree under `crates/mc-rs-server/src/connection/`
- this refactor is intentional and should be preserved

## What Was Completed Earlier

### Block palette / block registry

The server no longer depends on parsing `canonical_block_states.nbt` at runtime.

What was done:

- updated the Bedrock palette input to a real `1.26.10` source
- generated a static Rust registry table
- moved runtime lookup to generated Rust data instead of runtime NBT parsing
- documented the extraction workflow

Important files:

- `crates/mc-rs-server/src/world/block_registry.rs`
- `crates/mc-rs-server/src/world/block_registry_data.rs`
- `crates/mc-rs-server/src/bin/generate_block_registry.rs`
- `crates/mc-rs-server/data/canonical_block_states.nbt`
- `new-docs/25-BLOCK-PALETTE-EXTRACTION.md`

Important nuance:

- `canonical_block_states.nbt` is no longer a runtime dependency
- it is still used as a generation/reference input for rebuilding the static registry

### World persistence

World/chunk persistence was significantly improved.

What was fixed:

- modified chunks are saved
- biome data is persisted and reloaded
- chunks no longer reload as plain biome everywhere
- player inventory persistence was added

Important files:

- `crates/mc-rs-server/src/world/chunk_cache.rs`
- `crates/mc-rs-server/src/world/storage.rs`
- `crates/mc-rs-server/src/player_data.rs`
- `crates/mc-rs-server/src/inventory.rs`

### Seed handling

Seed `0` now behaves as “random seed persisted per world”.

What was done:

- active seed is stored in `worlds/world/level_seed.txt`
- deleting `worlds/` produces a different world next time
- keeping `worlds/` preserves the same world

Important files:

- `crates/mc-rs-server/src/config.rs`
- `crates/mc-rs-server/src/main.rs`

### Basic biome debug command

A `/biome` command was added to report:

- biome name / ID
- temperature
- rainfall
- related local terrain info

Important files:

- `crates/mc-rs-command/src/lib.rs`
- `crates/mc-rs-server/src/connection/`
- `crates/mc-rs-server/src/world/terrain_generator.rs`
- `crates/mc-rs-server/src/world/biome.rs`

## Entity Work Done In This Session Family

This became the main active area after worldgen frustration and item/inventory crashes.

### 1. Generic entity foundation

A generic actor base was introduced.

Important files:

- `crates/mc-rs-server/src/entity.rs`
- `crates/mc-rs-server/src/mob_entities.rs`
- `crates/mc-rs-proto/src/packets/player.rs`

This added support for:

- `AddActor`
- `MoveActorAbsolute`
- `SetActorMotion`
- entity metadata handling

### 2. Mob summoning now works

`/summon` was wired for several mobs and confirmed visible in game.

Supported mobs include:

- zombie
- skeleton
- creeper
- cow
- pig
- sheep
- chicken

Important files:

- `crates/mc-rs-server/src/commands.rs`
- `crates/mc-rs-server/src/mob_entities.rs`

### 3. `/kill` selector support improved

Selector handling was improved so these forms work more reliably:

- `@e[type=zombie]`
- `@e[type=minecraft:zombie]`

Important file:

- `crates/mc-rs-command/src/selector.rs`

### 4. Mob physics

Mobs now have a minimal gravity path and no longer just stay frozen in the air.

Important file:

- `crates/mc-rs-server/src/mob_entities.rs`

### 5. Player attacking mobs

Attacks are now processed through `InventoryTransaction::UseItemOnEntity`.

What exists:

- pending entity attack queue
- mob health reduction
- attribute updates
- entity removal on death

Important files:

- `crates/mc-rs-server/src/connection/inventory.rs`
- `crates/mc-rs-server/src/connection/mod.rs`
- `crates/mc-rs-server/src/main.rs`
- `crates/mc-rs-server/src/mob_entities.rs`

### 6. Mob drops were added

Mob deaths now create dropped items using the same item entity path used by block breaking.

Currently simple default loot only, for example:

- zombie -> rotten flesh
- skeleton -> bone + arrow
- creeper -> gunpowder
- cow -> beef + leather
- pig -> porkchop
- sheep -> white wool + mutton
- chicken -> chicken + feather

Important files:

- `crates/mc-rs-server/src/mob_entities.rs`
- `crates/mc-rs-server/src/main.rs`
- `crates/mc-rs-server/src/commands.rs`

Also:

- `/kill` on a mob now drops loot too, using the same item entity mechanism

## The Big Crash Investigation: Block Break -> Drop Item -> Client Crash

This was the most painful debugging thread.

### Symptom

Breaking a block in survival caused the Bedrock client to crash when a dropped item entity spawned.

### Important findings

Several suspected causes were investigated and partially fixed:

1. `AddItemActor` packet body was compared against PMMP / BedrockProtocol and matched closely.
2. `ItemRegistry` encoding had a real bug:
   - `component_nbt` was written raw
   - it needed a length-prefixed byte-array
   - this was fixed in `crates/mc-rs-server/src/item_registry.rs`
3. Item extra-data encoding in `ItemStack` was also corrected in `crates/mc-rs-proto/src/packets/player.rs`

Even after those fixes, the crash still happened.

### What finally stopped the crash

The crash stopped only after item entities were simplified into a static form.

Current workaround:

- no item entity movement packets
- no gravity flag for item entities
- drops spawn directly on the ground
- items are still visible and collectible

Important files:

- `crates/mc-rs-server/src/item_entities.rs`
- `crates/mc-rs-server/src/entity.rs`
- `crates/mc-rs-server/src/connection/movement.rs`

This is a stabilization patch, not a final implementation.

### Current understanding

The exact crash source was never fully isolated to one final packet-level root cause.

What is known:

- the old moving item-entity path was unsafe
- the static item-entity path does not crash the client
- movement for dropped items must be reintroduced carefully and incrementally

## Current State Of Item Entities

What currently works:

- breaking a supported block queues a dropped item entity
- dropped item entities spawn without crashing the client
- nearby pickup works
- pickup removes the entity and syncs inventory
- existing item entities are sent to newly joined players

Important files:

- `crates/mc-rs-server/src/item_entities.rs`
- `crates/mc-rs-server/src/main.rs`
- `crates/mc-rs-server/src/connection/movement.rs`

What is still intentionally limited:

- no real gravity for items
- no thrown arc
- no bouncing
- no proper motion packets for item entities
- no persistence of item entities across restart

## Inventory / UI Status

This area was extremely problematic earlier.

What improved:

- player inventory persistence exists
- item registry / network item IDs were corrected toward protocol 944 expectations
- direct “block goes immediately into inventory” behavior was replaced with world item entities

What is still not trustworthy:

- the full Bedrock inventory UI flow is not fully proven stable in all cases
- inventory-related bugs may still exist outside the now-stable item-drop path

Important files:

- `crates/mc-rs-server/src/inventory.rs`
- `crates/mc-rs-server/src/player_data.rs`
- `crates/mc-rs-server/src/connection/inventory.rs`
- `crates/mc-rs-proto/src/packets/player.rs`

## World Generation Status

World generation is still one of the weakest areas relative to the intended goal.

What was improved earlier:

- chunk persistence
- biome streaming/persistence
- seed stability
- reduced structure spam compared to a worse earlier state

What is still wrong or incomplete:

- generation is not Bedrock-accurate
- biome surface composition is still approximate
- structure placement is still approximate
- caves are not at Bedrock quality
- vegetation and biome feature logic are still heuristic

Important files:

- `crates/mc-rs-server/src/world/terrain_generator.rs`
- `crates/mc-rs-server/src/world/vegetation.rs`
- `crates/mc-rs-server/src/world/structure.rs`
- `crates/mc-rs-server/src/world/biome.rs`

Important strategic note:

- do not trust the roadmap as fully accurate completion state
- multiple items marked as “done” in spirit are not truly done at gameplay quality level

## Important Reference Material

Several reference projects were used and updated.

Main references:

- `.reference/PocketMine-MP`
- `.reference/BedrockProtocol`
- `.reference/CloudburstProtocol`
- `.reference/bedrock-protocol-docs`
- `.reference/BedrockData`
- `.reference/bedrock-rs`
- `.reference/gophertunnel`
- `.reference/dragonfly`

These repositories were `git pull --ff-only` updated earlier where possible.

Use them especially for:

- packet layout validation
- item/entity behavior comparison
- StartGame / ItemRegistry / actor metadata behavior

## Connection Refactor

`connection.rs` was split into modules.

Current folder:

- `crates/mc-rs-server/src/connection/chat.rs`
- `crates/mc-rs-server/src/connection/chunks.rs`
- `crates/mc-rs-server/src/connection/forms.rs`
- `crates/mc-rs-server/src/connection/inventory.rs`
- `crates/mc-rs-server/src/connection/login.rs`
- `crates/mc-rs-server/src/connection/mod.rs`
- `crates/mc-rs-server/src/connection/movement.rs`
- `crates/mc-rs-server/src/connection/spawn.rs`

This refactor should be kept.

## Build / Validation At Handoff

Validated recently:

- `cargo test -p mc-rs-server killing_a_mob_returns_position_and_loot`
- `cargo test -p mc-rs-server attacking_a_mob_updates_health_and_can_remove_it`
- `cargo build --release -p mc-rs-server`

At handoff time, a release server was started successfully and bound UDP `19132`.

Logs:

- `.reference/server.out.log`
- `.reference/server.err.log`
- older debugging also used `.reference/server.log`

## Recommended Next Steps

Best next steps, in order:

1. Validate the current stable entity loop in game:
   - `/summon cow`
   - kill by hand
   - verify drop appears
   - verify pickup works
   - verify `/kill @e[type=cow]` also drops loot
2. If stable, improve mob behavior before reintroducing dropped-item motion:
   - simple AI
   - wandering
   - player damage
   - death handling polish
3. Reintroduce item entity motion very carefully:
   - first only one packet type
   - validate client stability
   - then add gravity
   - then add motion updates
4. After entity stability, return to worldgen quality

## Hard Warnings For The Next AI

- Do not assume item entities are “finished”. They are currently stabilized, not complete.
- Do not aggressively reintroduce dropped-item movement all at once. That exact area caused repeated client crashes.
- Do not treat the roadmap as ground truth for real gameplay completeness.
- Preserve the `connection/` module split.
- Prefer PMMP / BedrockProtocol comparisons for packet behavior whenever the client crashes without a clear server-side error.

## Short Executive Summary

The project now has:

- a better block registry pipeline
- better chunk/world persistence
- a real seed persistence path
- a basic but real entity foundation
- working mob summon rendering
- mob attacks and deaths
- mob loot spawning through item entities
- a non-crashing static dropped-item path

The biggest remaining gameplay gaps are:

- incomplete item entity physics
- incomplete inventory/UI confidence
- weak world generation fidelity
- no real mob AI/combat loop yet
