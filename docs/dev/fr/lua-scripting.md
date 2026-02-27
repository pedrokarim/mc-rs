---
layout: default
title: Scripting Lua
nav_order: 6
---

# Guide de scripting Lua

MC-RS embarque **Lua 5.4** pour le scripting de plugins. Les plugins Lua sont simples, légers et ne nécessitent aucune compilation.

## Manifeste (`plugin.toml`)

```toml
[plugin]
name = "MonPlugin"
version = "1.0.0"
author = "VotreNom"
description = "Une courte description"
main = "main.lua"          # Script d'entrée (par défaut : main.lua)

[limits]
memory_mb = 16             # Mémoire max en Mo (par défaut : 16)
instruction_limit = 1000000 # Instructions max par callback (par défaut : 1 000 000)
```

## L'API `mc`

Toutes les interactions serveur passent par la table globale `mc`. Les fonctions sont groupées par catégorie.

### Gestion des événements

#### `mc.on(event_name, handler)`

Enregistre un gestionnaire d'événement. Le handler reçoit une table d'événement.

```lua
mc.on("player_join", function(event)
    mc.broadcast("Bienvenue " .. event.player.name .. " !")
end)
```

Pour **annuler** un événement annulable, définissez `event.cancelled = true` :

```lua
mc.on("player_chat", function(event)
    if event.message:find("spam") then
        event.cancelled = true
    end
end)
```

Voir la [Référence des événements](events) pour les 17 événements et leurs champs.

---

### Fonctions joueur

#### `mc.send_message(player_name, message)`

Envoie un message à un joueur spécifique.

```lua
mc.send_message("Steve", "Salut Steve !")
```

#### `mc.broadcast(message)`

Envoie un message à tous les joueurs connectés.

```lua
mc.broadcast("Annonce du serveur !")
```

#### `mc.kick(player_name, reason)`

Déconnecte un joueur.

```lua
mc.kick("Steve", "Vous avez été expulsé")
```

#### `mc.set_health(player_name, health)`

Définit la santé d'un joueur (0.0 à 20.0).

```lua
mc.set_health("Steve", 20.0)  -- Santé maximale
```

#### `mc.set_food(player_name, food)`

Définit le niveau de faim d'un joueur (0 à 20).

```lua
mc.set_food("Steve", 20)  -- Faim maximale
```

#### `mc.teleport(player_name, x, y, z)`

Téléporte un joueur aux coordonnées données.

```lua
mc.teleport("Steve", 0.5, 65.0, 0.5)
```

---

### Requêtes joueur

#### `mc.online_players()`

Retourne une table (tableau) de tous les joueurs connectés. Chaque joueur est une table avec les champs :

| Champ | Type | Description |
|-------|------|-------------|
| `name` | string | Nom du joueur |
| `uuid` | string | UUID du joueur |
| `runtime_id` | number | Runtime ID de l'entité |
| `x` | number | Coordonnée X |
| `y` | number | Coordonnée Y |
| `z` | number | Coordonnée Z |
| `gamemode` | number | 0=survie, 1=créatif, 2=aventure, 3=spectateur |
| `health` | number | Santé actuelle (0.0–20.0) |

```lua
local players = mc.online_players()
for _, p in ipairs(players) do
    mc.log(p.name .. " à " .. p.x .. ", " .. p.y .. ", " .. p.z)
end
```

#### `mc.get_player(name)`

Récupère un joueur spécifique par son nom. Retourne une table joueur ou `nil` si non trouvé.

```lua
local player = mc.get_player("Steve")
if player then
    mc.log("Santé de Steve : " .. player.health)
end
```

---

### Fonctions monde

#### `mc.get_time()`

Récupère l'heure actuelle du monde (0–24000).

```lua
local time = mc.get_time()
mc.log("Heure actuelle : " .. time)
```

#### `mc.set_time(time)`

Définit l'heure du monde.

```lua
mc.set_time(6000)  -- Midi
```

#### `mc.is_raining()`

Vérifie s'il pleut.

```lua
if mc.is_raining() then
    mc.broadcast("Il pleut !")
end
```

---

### Fonctions entité

#### `mc.spawn_mob(mob_type, x, y, z)`

Fait apparaître un mob aux coordonnées. Types disponibles : `zombie`, `skeleton`, `cow`, `pig`, `chicken`.

```lua
mc.spawn_mob("zombie", 10.5, 65.0, 10.5)
```

#### `mc.remove_mob(runtime_id)`

Supprime un mob par son runtime ID.

```lua
mc.remove_mob(42)
```

---

### Fonctions serveur

#### `mc.get_tick()`

Récupère le tick serveur actuel (20 ticks = 1 seconde).

```lua
mc.log("Tick serveur : " .. mc.get_tick())
```

---

### Logging

#### `mc.log(message)`

Log au niveau INFO.

#### `mc.log_warn(message)`

Log au niveau WARN.

#### `mc.log_error(message)`

Log au niveau ERROR.

#### `mc.log_debug(message)`

Log au niveau DEBUG.

Tous les messages de log sont préfixés avec le nom du plugin :
```
[MonPlugin] Votre message ici
```

---

### Commandes

#### `mc.register_command(name, description, handler)`

Enregistre une commande personnalisée. Le handler reçoit le nom de l'expéditeur et une table d'arguments. Retournez une chaîne pour envoyer une réponse, ou `nil` pour aucune réponse.

```lua
mc.register_command("spawn", "Se téléporter au spawn", function(sender, args)
    mc.teleport(sender, 0.5, 65.0, 0.5)
    return "Téléporté au spawn !"
end)

mc.register_command("heal", "Soigner un joueur", function(sender, args)
    local target = args[1] or sender
    mc.set_health(target, 20.0)
    mc.set_food(target, 20)
    return "Soigné " .. target
end)
```

---

### Planificateur de tâches

#### `mc.schedule(delay_ticks, callback)`

Planifie une tâche one-shot. Retourne un ID de tâche. (20 ticks = 1 seconde)

```lua
-- Exécuter après 5 secondes
local task_id = mc.schedule(100, function()
    mc.broadcast("5 secondes se sont écoulées !")
end)
```

#### `mc.schedule_repeating(delay_ticks, interval_ticks, callback)`

Planifie une tâche répétitive. Retourne un ID de tâche.

```lua
-- Diffuser toutes les 60 secondes, en commençant après 60 secondes
local task_id = mc.schedule_repeating(1200, 1200, function()
    mc.broadcast("Annonce périodique !")
end)
```

#### `mc.cancel_task(task_id)`

Annule une tâche planifiée.

```lua
local task_id = mc.schedule_repeating(200, 200, function()
    mc.broadcast("Répétition...")
end)

-- L'annuler plus tard
mc.cancel_task(task_id)
```

---

## Restrictions du sandbox

Pour la sécurité, les globales Lua suivantes sont **supprimées** et indisponibles :

| Supprimé | Raison |
|----------|--------|
| `os` | Accès au système de fichiers et aux processus |
| `io` | Opérations d'entrée/sortie fichier |
| `debug` | Hooks de debug et accès aux internes |
| `loadfile` | Chargement dynamique de fichiers |
| `dofile` | Exécution dynamique de fichiers |

**Disponibles** : `table`, `string`, `math`, `ipairs`, `pairs`, `type`, `tostring`, `tonumber`, `select`, `unpack`, `pcall`, `xpcall`, `error`, `assert`, `rawget`, `rawset`, `setmetatable`, `getmetatable`.

## Limites de ressources

Chaque plugin Lua s'exécute dans sa propre VM Lua isolée avec des limites configurables :

- **Mémoire** : Contrôlée par `memory_mb` dans `plugin.toml` (par défaut : 16 Mo)
- **Instructions** : Contrôlées par `instruction_limit` par callback (par défaut : 1 000 000)

Si un plugin dépasse ses limites, le callback est interrompu et une erreur est loguée.
