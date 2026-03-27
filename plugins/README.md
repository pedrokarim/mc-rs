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

### Exemple

Voir `plugins/example-lua/`.
