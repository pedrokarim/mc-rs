---
layout: default
title: Référence des événements
nav_order: 5
---

# Référence des événements

MC-RS envoie 17 événements aux plugins. **10 événements sont annulables** — retourner `Cancelled` depuis un événement annulable empêche l'action par défaut du serveur et arrête la propagation aux plugins restants.

## Résumé

| Événement | Catégorie | Annulable | Champs |
|-----------|-----------|:---------:|--------|
| `PlayerJoin` | Joueur | Non | player |
| `PlayerQuit` | Joueur | Non | player |
| `PlayerChat` | Joueur | **Oui** | player, message |
| `PlayerCommand` | Joueur | **Oui** | player, command, args |
| `PlayerMove` | Joueur | **Oui** | player, from, to |
| `PlayerDeath` | Joueur | Non | player, message |
| `PlayerDamage` | Joueur | **Oui** | player, damage, cause |
| `PlayerRespawn` | Joueur | Non | player |
| `BlockBreak` | Bloc | **Oui** | player, position, block_id |
| `BlockPlace` | Bloc | **Oui** | player, position, block_id |
| `MobSpawn` | Entité | **Oui** | mob_type, runtime_id, position |
| `MobDeath` | Entité | Non | mob_type, runtime_id, killer_runtime_id |
| `EntityDamage` | Entité | **Oui** | runtime_id, damage, attacker_runtime_id |
| `WeatherChange` | Monde | **Oui** | raining, thundering |
| `TimeChange` | Monde | **Oui** | new_time |
| `ServerStarted` | Serveur | Non | *(aucun)* |
| `ServerStopping` | Serveur | Non | *(aucun)* |

---

## Événements joueur

### PlayerJoin

Déclenché quand un joueur rejoint le serveur, après la séquence de connexion.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur qui se connecte |

**Nom Lua :** `player_join`

```lua
mc.on("player_join", function(event)
    mc.broadcast(event.player.name .. " a rejoint le serveur !")
end)
```

---

### PlayerQuit

Déclenché quand un joueur se déconnecte.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur qui part |

**Nom Lua :** `player_quit`

```lua
mc.on("player_quit", function(event)
    mc.broadcast(event.player.name .. " a quitté le serveur.")
end)
```

---

### PlayerChat (Annulable)

Déclenché quand un joueur envoie un message dans le chat. Annuler empêche la diffusion du message.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | L'expéditeur |
| `message` | String | Le message |

**Nom Lua :** `player_chat`

```lua
mc.on("player_chat", function(event)
    if event.message:find("motinterdit") then
        event.cancelled = true
        mc.send_message(event.player.name, "Surveillez votre langage !")
    end
end)
```

---

### PlayerCommand (Annulable)

Déclenché quand un joueur exécute une commande. Annuler empêche l'exécution.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur |
| `command` | String | Nom de la commande (sans `/`) |
| `args` | String[] | Arguments de la commande |

**Nom Lua :** `player_command`

```lua
mc.on("player_command", function(event)
    mc.log("Commande: /" .. event.command .. " par " .. event.player.name)
end)
```

---

### PlayerMove (Annulable)

Déclenché quand un joueur se déplace. Annuler le téléporte à la position `from`.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur |
| `from` | (f32, f32, f32) | Position précédente (x, y, z) |
| `to` | (f32, f32, f32) | Nouvelle position (x, y, z) |

**Nom Lua :** `player_move`

En Lua, `from` et `to` sont des tables avec les champs `x`, `y`, `z` :

```lua
mc.on("player_move", function(event)
    -- Empêcher de dépasser X=100
    if event.to.x > 100 then
        event.cancelled = true
    end
end)
```

---

### PlayerDeath

Déclenché quand un joueur meurt.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur décédé |
| `message` | String | Message de mort |

**Nom Lua :** `player_death`

```lua
mc.on("player_death", function(event)
    mc.broadcast(event.message)
end)
```

---

### PlayerDamage (Annulable)

Déclenché quand un joueur subit des dégâts. Annuler empêche les dégâts.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur touché |
| `damage` | f32 | Montant des dégâts (en demi-cœurs) |
| `cause` | DamageCause | Cause des dégâts |

**Nom Lua :** `player_damage`

Valeurs DamageCause en Lua : `"Attack"`, `"Fall"`, `"Drowning"`, `"Lava"`, `"Fire"`, `"Suffocation"`, `"Starvation"`, `"Void"`, `"Other"`

```lua
mc.on("player_damage", function(event)
    -- Désactiver les dégâts de chute
    if event.cause == "Fall" then
        event.cancelled = true
    end
end)
```

---

### PlayerRespawn

Déclenché quand un joueur réapparaît après sa mort.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur qui réapparaît |

**Nom Lua :** `player_respawn`

```lua
mc.on("player_respawn", function(event)
    mc.send_message(event.player.name, "Bon retour parmi nous !")
end)
```

---

## Événements bloc

### BlockBreak (Annulable)

Déclenché quand un joueur casse un bloc. Annuler empêche la destruction.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur |
| `position` | PluginBlockPos | Coordonnées du bloc (x, y, z) |
| `block_id` | u32 | Runtime ID du bloc (hash FNV-1a) |

**Nom Lua :** `block_break`

```lua
mc.on("block_break", function(event)
    local pos = event.position
    mc.log("Bloc cassé à " .. pos.x .. ", " .. pos.y .. ", " .. pos.z)
end)
```

---

### BlockPlace (Annulable)

Déclenché quand un joueur place un bloc. Annuler empêche le placement.

| Champ | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | Le joueur |
| `position` | PluginBlockPos | Coordonnées du bloc (x, y, z) |
| `block_id` | u32 | Runtime ID du bloc (hash FNV-1a) |

**Nom Lua :** `block_place`

```lua
mc.on("block_place", function(event)
    -- Empêcher de placer des blocs au-dessus de Y=200
    if event.position.y > 200 then
        event.cancelled = true
        mc.send_message(event.player.name, "Limite de construction atteinte !")
    end
end)
```

---

## Événements entité

### MobSpawn (Annulable)

Déclenché quand un mob apparaît (naturellement ou via commande). Annuler empêche l'apparition.

| Champ | Type | Description |
|-------|------|-------------|
| `mob_type` | String | Identifiant du mob (ex : `"zombie"`) |
| `runtime_id` | u64 | Runtime ID de l'entité |
| `position` | (f32, f32, f32) | Coordonnées d'apparition (x, y, z) |

**Nom Lua :** `mob_spawn`

```lua
mc.on("mob_spawn", function(event)
    -- Pas de zombies !
    if event.mob_type == "zombie" then
        event.cancelled = true
    end
end)
```

---

### MobDeath

Déclenché quand un mob meurt.

| Champ | Type | Description |
|-------|------|-------------|
| `mob_type` | String | Identifiant du mob |
| `runtime_id` | u64 | Runtime ID de l'entité |
| `killer_runtime_id` | Option\<u64\> | Runtime ID du tueur (si applicable) |

**Nom Lua :** `mob_death`

En Lua, `killer_runtime_id` est `nil` s'il n'y a pas de tueur :

```lua
mc.on("mob_death", function(event)
    if event.killer_runtime_id then
        mc.log(event.mob_type .. " tué par l'entité " .. event.killer_runtime_id)
    end
end)
```

---

### EntityDamage (Annulable)

Déclenché quand une entité subit des dégâts. Annuler empêche les dégâts.

| Champ | Type | Description |
|-------|------|-------------|
| `runtime_id` | u64 | Runtime ID de l'entité touchée |
| `damage` | f32 | Montant des dégâts |
| `attacker_runtime_id` | Option\<u64\> | Runtime ID de l'attaquant (si applicable) |

**Nom Lua :** `entity_damage`

```lua
mc.on("entity_damage", function(event)
    mc.log("Entité " .. event.runtime_id .. " a subi " .. event.damage .. " dégâts")
end)
```

---

## Événements monde

### WeatherChange (Annulable)

Déclenché quand la météo change. Annuler conserve la météo actuelle.

| Champ | Type | Description |
|-------|------|-------------|
| `raining` | bool | S'il va pleuvoir |
| `thundering` | bool | S'il va y avoir de l'orage |

**Nom Lua :** `weather_change`

```lua
mc.on("weather_change", function(event)
    -- Garder le beau temps !
    if event.raining then
        event.cancelled = true
    end
end)
```

---

### TimeChange (Annulable)

Déclenché quand l'heure du monde change (via commande ou cycle naturel). Annuler empêche le changement.

| Champ | Type | Description |
|-------|------|-------------|
| `new_time` | i64 | La nouvelle heure |

**Nom Lua :** `time_change`

```lua
mc.on("time_change", function(event)
    mc.log("Heure qui change vers " .. event.new_time)
end)
```

---

## Événements serveur

### ServerStarted

Déclenché une seule fois au premier tick de jeu, après le chargement de tous les plugins.

**Nom Lua :** `server_started`

```lua
mc.on("server_started", function(event)
    mc.log("Le serveur est prêt !")
end)
```

---

### ServerStopping

Déclenché pendant l'arrêt du serveur, avant la désactivation des plugins. Les actions mises en file pendant cet événement ne sont **pas appliquées** (le serveur s'arrête).

**Nom Lua :** `server_stopping`

```lua
mc.on("server_stopping", function(event)
    mc.log("Le serveur s'arrête...")
end)
```

---

## Format JSON WASM

Pour les plugins WASM, les événements sont sérialisés en JSON et passés à `__on_event(ptr, len)`. Le JSON utilise le format enum tagué par défaut de serde :

```json
{
  "PlayerChat": {
    "player": {
      "name": "Steve",
      "uuid": "...",
      "runtime_id": 1,
      "position": [0.5, 65.62, 0.5],
      "gamemode": 0,
      "health": 20.0
    },
    "message": "Hello world"
  }
}
```

Retournez `1` depuis `__on_event` pour annuler, `0` pour continuer.
