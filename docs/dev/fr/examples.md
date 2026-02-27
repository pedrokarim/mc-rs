---
layout: default
title: Exemples
nav_order: 8
---

# Exemples de plugins

Des exemples complets de plugins prêts à l'emploi pour MC-RS.

---

## Exemple 1 : Plugin d'accueil (Lua)

Un plugin qui accueille les nouveaux joueurs, enregistre une commande et envoie des annonces périodiques.

### `plugins/accueil/plugin.toml`

```toml
[plugin]
name = "Accueil"
version = "1.0.0"
author = "Admin"
description = "Accueille les joueurs et envoie des annonces"
main = "main.lua"
```

### `plugins/accueil/main.lua`

```lua
-- Accueillir les joueurs à leur connexion
mc.on("player_join", function(event)
    mc.send_message(event.player.name, "Bienvenue sur le serveur, " .. event.player.name .. " !")

    -- Prévenir les autres
    local players = mc.online_players()
    for _, p in ipairs(players) do
        if p.name ~= event.player.name then
            mc.send_message(p.name, event.player.name .. " a rejoint le serveur !")
        end
    end
end)

-- Dire au revoir
mc.on("player_quit", function(event)
    mc.broadcast(event.player.name .. " a quitté le serveur.")
end)

-- Commande /accueil
mc.register_command("accueil", "Envoyer un message de bienvenue", function(sender, args)
    local target = args[1] or sender
    local player = mc.get_player(target)
    if player then
        mc.send_message(target, "Bienvenue sur le serveur ! Bon jeu !")
        return "Message envoyé à " .. target
    else
        return "Joueur non trouvé : " .. target
    end
end)

-- Annonce périodique toutes les 5 minutes (6000 ticks)
mc.schedule_repeating(6000, 6000, function()
    local count = #mc.online_players()
    if count > 0 then
        mc.broadcast("Il y a " .. count .. " joueurs en ligne !")
    end
end)

mc.log("Plugin Accueil chargé !")
```

---

## Exemple 2 : Filtre de chat (Lua)

Un filtre de chat basique qui bloque les messages contenant des mots interdits.

### `plugins/filtre-chat/plugin.toml`

```toml
[plugin]
name = "FiltreChat"
version = "1.0.0"
description = "Filtre les mots interdits du chat"
main = "main.lua"
```

### `plugins/filtre-chat/main.lua`

```lua
-- Mots interdits (minuscules)
local interdits = { "motinterdit1", "motinterdit2", "spam" }

mc.on("player_chat", function(event)
    local msg = event.message:lower()
    for _, mot in ipairs(interdits) do
        if msg:find(mot, 1, true) then
            event.cancelled = true
            mc.send_message(event.player.name, "Votre message a été bloqué (mot interdit).")
            mc.log_warn(event.player.name .. " a essayé de dire : " .. event.message)
            return
        end
    end
end)

mc.log("FiltreChat chargé avec " .. #interdits .. " mots interdits.")
```

---

## Exemple 3 : Protection du spawn (Lua)

Empêche la destruction et le placement de blocs près du point d'apparition.

### `plugins/protection-spawn/plugin.toml`

```toml
[plugin]
name = "ProtectionSpawn"
version = "1.0.0"
description = "Protège les blocs près du spawn"
main = "main.lua"
```

### `plugins/protection-spawn/main.lua`

```lua
local SPAWN_X = 0
local SPAWN_Z = 0
local RAYON = 50

local function est_protege(pos)
    local dx = pos.x - SPAWN_X
    local dz = pos.z - SPAWN_Z
    return (dx * dx + dz * dz) < (RAYON * RAYON)
end

mc.on("block_break", function(event)
    if est_protege(event.position) then
        event.cancelled = true
        mc.send_message(event.player.name, "Vous ne pouvez pas casser de blocs près du spawn !")
    end
end)

mc.on("block_place", function(event)
    if est_protege(event.position) then
        event.cancelled = true
        mc.send_message(event.player.name, "Vous ne pouvez pas placer de blocs près du spawn !")
    end
end)

mc.log("Protection du spawn active (rayon : " .. RAYON .. " blocs)")
```

---

## Exemple 4 : Arène de mobs (Lua)

Une arène de mobs pilotée par commande qui fait apparaître des vagues d'ennemis.

### `plugins/arene-mobs/plugin.toml`

```toml
[plugin]
name = "AreneMobs"
version = "1.0.0"
description = "Vagues de mobs avec /arene"
main = "main.lua"
```

### `plugins/arene-mobs/main.lua`

```lua
local arene_task = nil
local vague = 0
local ARENE_X, ARENE_Y, ARENE_Z = 50.5, 65.0, 50.5

local types_mobs = { "zombie", "skeleton", "zombie", "skeleton", "zombie" }

local function lancer_vague()
    vague = vague + 1
    mc.broadcast("Vague " .. vague .. " en approche !")

    local nombre = math.min(vague + 2, 10)  -- Plus de mobs à chaque vague
    for i = 1, nombre do
        local mob = types_mobs[(i % #types_mobs) + 1]
        local offset_x = (i - nombre/2) * 2
        mc.spawn_mob(mob, ARENE_X + offset_x, ARENE_Y, ARENE_Z)
    end
end

mc.register_command("arene", "Démarrer ou arrêter l'arène de mobs", function(sender, args)
    local action = args[1] or "start"

    if action == "start" then
        if arene_task then
            return "L'arène est déjà en cours !"
        end
        vague = 0
        mc.broadcast("Arène de mobs lancée par " .. sender .. " !")
        lancer_vague()
        -- Nouvelle vague toutes les 30 secondes
        arene_task = mc.schedule_repeating(600, 600, function()
            lancer_vague()
        end)
        return "Arène démarrée !"

    elseif action == "stop" then
        if arene_task then
            mc.cancel_task(arene_task)
            arene_task = nil
            vague = 0
            mc.broadcast("Arène de mobs arrêtée !")
            return "Arène arrêtée."
        else
            return "L'arène n'est pas en cours."
        end
    else
        return "Usage : /arene <start|stop>"
    end
end)

mc.log("Plugin AreneMobs chargé !")
```

---

## Exemple 5 : Plugin WASM minimal (Rust)

Un plugin WASM complet écrit en Rust qui logue un message à l'activation et diffuse un message quand un joueur rejoint.

### Structure du répertoire

```
plugins/hello-wasm/
├── plugin.toml
└── plugin.wasm        (compilé depuis le projet Rust ci-dessous)

hello-wasm-src/
├── Cargo.toml
└── src/
    └── lib.rs
```

### `plugin.toml`

```toml
[plugin]
name = "HelloWasm"
version = "1.0.0"
author = "Dev"
description = "Plugin WASM minimal"
wasm_file = "plugin.wasm"

[limits]
fuel_per_event = 1000000
max_memory_pages = 16
```

### `Cargo.toml`

```toml
[package]
name = "hello-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "s"
lto = true
```

### `src/lib.rs`

```rust
// ─── Import des fonctions hôte (module "mcrs") ───

#[link(wasm_import_module = "mcrs")]
extern "C" {
    fn broadcast_message(ptr: i32, len: i32);
    fn log(level: i32, ptr: i32, len: i32);
}

// ─── Gestion mémoire ───

static mut HEAP: [u8; 32768] = [0; 32768];
static mut HEAP_POS: usize = 0;

#[no_mangle]
pub extern "C" fn __malloc(size: i32) -> i32 {
    unsafe {
        let ptr = HEAP.as_ptr().add(HEAP_POS) as i32;
        HEAP_POS += size as usize;
        ptr
    }
}

#[no_mangle]
pub extern "C" fn __free(_ptr: i32, _size: i32) {}

// ─── Cycle de vie du plugin ───

#[no_mangle]
pub extern "C" fn __plugin_info() -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __on_enable() {
    let msg = b"Plugin HelloWasm actif !";
    unsafe { log(0, msg.as_ptr() as i32, msg.len() as i32); }
}

#[no_mangle]
pub extern "C" fn __on_disable() {}

#[no_mangle]
pub extern "C" fn __on_event(ptr: i32, len: i32) -> i32 {
    // Lire le JSON de l'événement depuis la mémoire
    let json = unsafe {
        let slice = core::slice::from_raw_parts(ptr as *const u8, len as usize);
        core::str::from_utf8_unchecked(slice)
    };

    // Vérification simple : si l'événement contient "PlayerJoin", diffuser un message
    if json.contains("PlayerJoin") {
        let msg = b"Un nouveau joueur a rejoint !";
        unsafe { broadcast_message(msg.as_ptr() as i32, msg.len() as i32); }
    }

    0 // Continuer (ne pas annuler)
}

#[no_mangle]
pub extern "C" fn __on_task(_task_id: i32) {}

#[no_mangle]
pub extern "C" fn __on_command(_ptr: i32, _len: i32) -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __default_config() -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __load_config(_ptr: i32, _len: i32) {}
```

### Compilation et déploiement

```bash
cd hello-wasm-src
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/hello_wasm.wasm ../plugins/hello-wasm/plugin.wasm
```
