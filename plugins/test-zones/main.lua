-- Zones de test : /testzone <zone> construit, devant le joueur, un terrain prêt pour tester une
-- fonctionnalité ; /testzone clear l'efface ; /testzone (ou list) liste les zones.
--
-- Tout passe par les commandes du serveur (run_command : /fill, /setblock, /give…), lancées au nom du
-- joueur : le plugin ne touche à rien d'autre. Blocs avec états : syntaxe de Bedrock,
-- « minecraft:wheat[growth=7] ».
--
-- Ajouter une zone : une entrée dans ZONES, avec
--   description  texte affiché par /testzone list
--   size         { x, y, z } : emprise, vidée avant la construction et effacée par /testzone clear
--   build(zone)  pose les blocs avec les fonctions de `zone` (coordonnées relatives au coin de la zone,
--                y = 0 au niveau du sol) : zone.fill, zone.set, zone.give, zone.command, zone.note.

local ZONES = {}
local ORDER = {}

local function define(name, zone)
  ZONES[name] = zone
  table.insert(ORDER, name)
end

-- Rangée de blocs isolés (un bloc sur deux) le long de x, à la profondeur z, sur un sol donné.
local function row(zone, z, floor, blocks)
  for i, block in ipairs(blocks) do
    local x = (i - 1) * 2
    zone.set(x, -1, z, floor)
    if type(block) == "table" then
      -- Bloc sur deux hauteurs : { bas, haut }.
      zone.set(x, 0, z, block[1])
      zone.set(x, 1, z, block[2])
    else
      zone.set(x, 0, z, block)
    end
  end
end

-- ── Zone « drops » : cassage, outils, Toucher de soie, Fortune, drops aléatoires ──
define("drops", {
  description = "Cassage et drops : minerais, terre et pierre, plantes, cultures mûres, blocs multiples",
  size = { 30, 4, 11 },
  build = function(zone)
    zone.fill(0, -1, 0, 29, -1, 10, "minecraft:stone")

    -- z = 0 : minerais.
    row(zone, 0, "minecraft:stone", {
      "minecraft:coal_ore", "minecraft:iron_ore", "minecraft:copper_ore", "minecraft:gold_ore",
      "minecraft:redstone_ore", "minecraft:lapis_ore", "minecraft:diamond_ore", "minecraft:emerald_ore",
      "minecraft:deepslate_diamond_ore", "minecraft:nether_gold_ore", "minecraft:ancient_debris",
      "minecraft:obsidian",
    })
    -- z = 2 : pierre, terre, blocs qui changent de drop selon l'outil.
    row(zone, 2, "minecraft:stone", {
      "minecraft:stone", "minecraft:deepslate", "minecraft:grass_block", "minecraft:gravel",
      "minecraft:clay", "minecraft:glowstone", "minecraft:sea_lantern", "minecraft:glass",
      "minecraft:bookshelf", "minecraft:melon_block", "minecraft:ice", "minecraft:iron_bars",
      "minecraft:web", "minecraft:beehive", "minecraft:dragon_egg",
    })
    -- z = 4 : plantes, sur de l'herbe.
    row(zone, 4, "minecraft:grass_block", {
      "minecraft:oak_leaves", "minecraft:birch_leaves", "minecraft:short_grass", "minecraft:fern",
      { "minecraft:tall_grass[upper_block_bit=false]", "minecraft:tall_grass[upper_block_bit=true]" },
      { "minecraft:sunflower[upper_block_bit=false]", "minecraft:sunflower[upper_block_bit=true]" },
      { "minecraft:rose_bush[upper_block_bit=false]", "minecraft:rose_bush[upper_block_bit=true]" },
      "minecraft:deadbush", "minecraft:brown_mushroom_block", "minecraft:sweet_berry_bush[growth=3]",
    })
    -- z = 6 : cultures mûres, sur de la terre labourée.
    row(zone, 6, "minecraft:farmland", {
      "minecraft:wheat[growth=7]", "minecraft:carrots[growth=7]", "minecraft:potatoes[growth=7]",
      "minecraft:beetroot[growth=7]", "minecraft:wheat[growth=0]",
    })
    zone.set(10, -1, 6, "minecraft:soul_sand")
    zone.set(10, 0, 6, "minecraft:nether_wart[age=3]")
    -- z = 8 : blocs qui rendent plusieurs items.
    row(zone, 8, "minecraft:stone", {
      "minecraft:red_candle[candles=3]", "minecraft:sea_pickle[cluster_count=3]",
      "minecraft:snow_layer[height=7]", "minecraft:pink_petals[growth=3]",
      "minecraft:turtle_egg[turtle_egg_count=four_egg]",
    })

    zone.give("minecraft:wooden_pickaxe")
    zone.give("minecraft:stone_pickaxe")
    zone.give("minecraft:iron_pickaxe")
    zone.give("minecraft:diamond_pickaxe")
    zone.give("minecraft:netherite_pickaxe")
    zone.give("minecraft:iron_axe")
    zone.give("minecraft:iron_shovel")
    zone.give("minecraft:iron_hoe")
    zone.give("minecraft:shears")
    zone.command("gamemode survival @s")
    zone.note("Rangées : minerais, pierre et terre, plantes, cultures mûres, blocs multiples.")
    zone.note("Enchanter l'outil tenu : /enchant @s fortune 3, ou /enchant @s silk_touch 1.")
  end,
})

-- ── Moteur ──

-- Dernière zone construite par joueur, pour /testzone clear.
local built = {}

local function floor(v)
  return math.floor(v)
end

local function quoted(name)
  return '"' .. name .. '"'
end

local function build(sender, name, zone)
  -- Coin de la zone : deux blocs devant le joueur en x, sol au niveau de ses pieds - 1.
  local ox, oy, oz = floor(sender.x) + 2, floor(sender.y), floor(sender.z)
  local notes = {}
  local api = {}

  local function at(x, y, z)
    return string.format("%d %d %d", ox + x, oy + y, oz + z)
  end
  function api.fill(x1, y1, z1, x2, y2, z2, block)
    run_command(string.format("fill %s %s %s", at(x1, y1, z1), at(x2, y2, z2), block))
  end
  function api.set(x, y, z, block)
    run_command(string.format("setblock %s %s", at(x, y, z), block))
  end
  function api.give(item, count)
    run_command(string.format("give %s %s %d", quoted(sender.name), item, count or 1))
  end
  function api.command(line)
    run_command(line)
  end
  function api.note(text)
    table.insert(notes, text)
  end

  local sx, sy, sz = zone.size[1], zone.size[2], zone.size[3]
  -- Emprise vidée (sol compris) avant la construction.
  api.fill(0, -1, 0, sx - 1, sy - 1, sz - 1, "minecraft:air")
  zone.build(api)
  built[sender.name] = { ox = ox, oy = oy, oz = oz, size = zone.size, name = name }

  local lines = { string.format("Zone « %s » construite en %d %d %d.", name, ox, oy, oz) }
  for _, note in ipairs(notes) do
    table.insert(lines, note)
  end
  return table.concat(lines, "\n")
end

local function clear(sender)
  local last = built[sender.name]
  if last == nil then
    return "Aucune zone à effacer."
  end
  local s = last.size
  run_command(string.format("fill %d %d %d %d %d %d minecraft:air",
    last.ox, last.oy - 1, last.oz, last.ox + s[1] - 1, last.oy + s[2] - 1, last.oz + s[3] - 1))
  built[sender.name] = nil
  return string.format("Zone « %s » effacée.", last.name)
end

local function list()
  local lines = { "Zones de test (/testzone <zone>, /testzone clear) :" }
  for _, name in ipairs(ORDER) do
    table.insert(lines, string.format("  %s : %s", name, ZONES[name].description))
  end
  return table.concat(lines, "\n")
end

register_command("testzone", function(sender, args)
  local name = args[1]
  if name == nil or name == "list" then
    return list()
  end
  if not sender.is_player then
    return "Commande réservée aux joueurs : la zone se construit à leur position."
  end
  if name == "clear" then
    return clear(sender)
  end
  local zone = ZONES[name]
  if zone == nil then
    return "Zone inconnue : " .. name .. "\n" .. list()
  end
  return build(sender, name, zone)
end)

function on_enable()
  log(string.format("%d zone(s) de test : /testzone list", #ORDER))
end
