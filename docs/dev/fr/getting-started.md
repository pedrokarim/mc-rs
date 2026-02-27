---
layout: default
title: Démarrage rapide
nav_order: 2
---

# Démarrage rapide

Créez votre premier plugin MC-RS en moins de 5 minutes.

## Prérequis

- Un binaire `mc-rs-server` compilé (ou `cargo run` depuis le workspace)
- Pour les plugins Lua : rien d'autre (Lua 5.4 est embarqué)
- Pour les plugins WASM : un toolchain Rust avec la target `wasm32-unknown-unknown`

## Structure du répertoire de plugins

Les plugins se trouvent dans le dossier `plugins/` à la racine du serveur. Chaque plugin est un sous-dossier avec un manifeste `plugin.toml` :

```
server/
├── mc-rs-server(.exe)
├── server.toml
└── plugins/
    ├── mon-plugin-lua/
    │   ├── plugin.toml
    │   └── main.lua
    └── mon-plugin-wasm/
        ├── plugin.toml
        └── plugin.wasm
```

Le serveur détecte automatiquement le type de plugin : si `plugin.toml` contient un champ `wasm_file`, il est chargé en WASM. Sinon, il est chargé en Lua.

## Votre premier plugin Lua

### 1. Créez le répertoire

```bash
mkdir -p plugins/hello-world
```

### 2. Créez `plugin.toml`

```toml
[plugin]
name = "HelloWorld"
version = "1.0.0"
author = "VotreNom"
description = "Accueille les joueurs à leur connexion"
main = "main.lua"
```

### 3. Créez `main.lua`

```lua
-- Accueillir les joueurs à leur connexion
mc.on("player_join", function(event)
    mc.broadcast("Bienvenue " .. event.player.name .. " sur le serveur !")
end)

-- Enregistrer une commande personnalisée
mc.register_command("hello", "Dire bonjour à tout le monde", function(sender, args)
    mc.broadcast(sender .. " dit bonjour !")
    return "Bonjour envoyé !"
end)

mc.log("Plugin HelloWorld chargé !")
```

### 4. Démarrez le serveur

```bash
cargo run
# ou
./mc-rs-server
```

Vous devriez voir dans les logs :
```
Loaded Lua plugin: HelloWorld v1.0.0
[HelloWorld] Plugin HelloWorld chargé !
```

## Fonctionnement du chargement des plugins

Au démarrage, le serveur :

1. Crée le dossier `plugins/` s'il n'existe pas
2. Parcourt chaque sous-dossier à la recherche d'un fichier `plugin.toml`
3. Charge les **plugins WASM** en premier (dossiers avec `wasm_file` dans le manifeste)
4. Charge les **plugins Lua** ensuite (dossiers sans `wasm_file`)
5. Appelle `on_enable` sur chaque plugin dans l'ordre de chargement
6. Envoie l'événement `ServerStarted` au premier tick de jeu

Les plugins sont déchargés dans l'ordre inverse à l'arrêt du serveur, recevant un événement `ServerStopping` suivi de `on_disable`.

## Configuration des plugins

Les plugins peuvent avoir un fichier `config.json` dans leur répertoire. Le serveur le charge automatiquement et le passe au plugin via `load_config()`.

## Étapes suivantes

- Lisez le [Guide Lua](lua-scripting) pour la référence complète de l'API `mc.*`
- Consultez la [Référence des événements](events) pour voir les 17 événements disponibles
- Voir les [Exemples](examples) pour des plugins complets
