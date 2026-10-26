## Lua Plugins

Les plugins chargés actuellement par `mc-rs` sont des plugins `Lua`.

Chaque plugin vit dans son propre dossier sous `plugins/` :

```text
plugins/
  my-plugin/
    plugin.yml
    main.lua
    config.yml   # optionnel
```

### `plugin.yml`

Champs supportés actuellement :

- `name`
- `version`
- `main`
- `api`
- `description`
- `author` / `authors`
- `website`
- `load` (`STARTUP` ou `POSTWORLD`)
- `enabled` (extension de mc-rs, `true` par défaut) : `false` laisse le plugin de côté au démarrage sans
  retirer son dossier
- `prefix`
- `depend`
- `softdepend`
- `loadbefore`
- `commands`
- `permissions`

### Script Lua

Le champ `main` doit pointer vers un script Lua, par exemple `main.lua`.

Hooks optionnels :

- `on_load()`
- `on_enable()`
- `on_disable()`

API globale disponible :

- `register_command(name, handler)`
- `run_command(ligne)` : lance une commande du serveur (`/fill`, `/setblock`, `/give`…) au nom de l'expéditeur,
  après le gestionnaire de commande, dans l'ordre des appels ; une erreur est renvoyée à l'expéditeur. Les
  commandes d'un plugin ne peuvent pas être lancées ainsi.
- `log(message)`
- `warn(message)`
- `error_log(message)`
- `broadcast(message)`
- `save_default_config()`
- `load_config()`
- `get_data_folder()`
- `get_config_path()`
- `get_plugin_name()`

Signature d'un handler de commande :

```lua
register_command("hello", function(sender, args, raw_args, label)
  local target = args[1] or sender.name
  return "Hello, " .. target .. "!"
end)
```

Le `sender` expose :

- `sender.name`
- `sender.is_player`
- `sender.is_op`
- `sender.x`, `sender.y`, `sender.z` (position, pour un joueur seulement)

### Exemples

- `plugins/example-lua/` : commande, configuration, hooks.
- `plugins/test-zones/` : `/testzone <zone>` construit devant le joueur un terrain de test (zone `drops` :
  minerais, plantes, cultures mûres, blocs multiples, outils donnés), `/testzone clear` l'efface,
  `/testzone list` liste les zones. Une zone = une entrée du tableau `ZONES` de `main.lua`. Se désactive
  par `enabled: false` dans son `plugin.yml`, se retire en supprimant le dossier.

Les blocs des commandes acceptent leurs états, syntaxe de Bedrock : `minecraft:wheat["growth"=7]` ou
`minecraft:wheat[growth=7]`.
