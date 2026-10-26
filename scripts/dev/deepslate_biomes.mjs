// Biomes d'une seed calculés par deepslate (portage JavaScript du worldgen Java, par misode) avec les données
// de mc-rs (`crates/mc-rs-server/data/worldgen`), comparés à une carte relevée par `chunk_biomes.py --json`
// (BDS ou mc-rs). Sert d'oracle indépendant pour localiser un écart : deepslate retrouve 97 % des colonnes de
// BDS, le reste vient des frontières propres à Bedrock (dev-docs/14-WORLD-GENERATION.md, « Parité avec BDS »).
//
// deepslate se compile hors du dépôt, à la version 0.26.0 (commit 564eb02) : c'est la dernière qui lit le format
// 1.21 des données (`shifted_noise`, `flat_cache`…), et la première qui applique le facteur 4 de
// `shift_a`/`shift_b` comme vanilla (la 0.22.3 l'omet). La copie de `.reference/deepslate` ne se modifie pas :
//   git -C .reference/deepslate archive 564eb02 src tsconfig-base.json package.json package-lock.json | tar -x -C <dossier>
//   cd <dossier> && npm install --ignore-scripts && npx tsc -b src
//
// Usage :
//   node scripts/dev/deepslate_biomes.mjs --deepslate <dossier> --seed <seed> --map <carte.json> [--y 64]
//     [--out <carte deepslate.json>]
// Sortie : part des colonnes identiques et confusions les plus fréquentes (biome Java -> identifiant Bedrock lu).

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const args = Object.fromEntries(process.argv.slice(2).reduce((pairs, arg, i, all) =>
  arg.startsWith('--') ? [...pairs, [arg.slice(2), all[i + 1]]] : pairs, []))
for (const key of ['deepslate', 'seed', 'map']) {
  if (!args[key]) {
    console.error(`--${key} manquant (voir l'en-tête du script)`)
    process.exit(1)
  }
}
const y = Number(args.y ?? 64)
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')
const worldgen = path.join(repo, 'crates', 'mc-rs-server', 'data', 'worldgen')
const { Identifier, WorldgenRegistries, RandomState, MultiNoiseBiomeSource } =
  await import(pathToFileURL(path.join(path.resolve(args.deepslate), 'lib', 'index.js')).href)

// Chaque fichier JSON devient une entrée du registre, nommée d'après son chemin relatif (`overworld/continents`).
function register(dir, registry) {
  const walk = (current, prefix) => {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        walk(path.join(current, entry.name), `${prefix}${entry.name}/`)
      } else if (entry.name.endsWith('.json')) {
        const json = JSON.parse(fs.readFileSync(path.join(current, entry.name), 'utf8'))
        registry.register(Identifier.create(prefix + entry.name.slice(0, -5)), () => registry.parse(json))
      }
    }
  }
  walk(path.join(worldgen, dir), '')
}
register('noise', WorldgenRegistries.NOISE)
register('density_function', WorldgenRegistries.DENSITY_FUNCTION)
register('noise_settings', WorldgenRegistries.NOISE_SETTINGS)

const settings = WorldgenRegistries.NOISE_SETTINGS.getOrThrow(Identifier.create('overworld'))
const state = new RandomState(settings, BigInt(args.seed))
const parameters = JSON.parse(fs.readFileSync(path.join(worldgen, 'biome_parameters', 'overworld.json'), 'utf8'))
const source = MultiNoiseBiomeSource.fromJson({ biomes: parameters })
const javaToBedrock = JSON.parse(fs.readFileSync(path.join(worldgen, 'biome_parameters', 'java_to_bedrock.json'), 'utf8'))

const observed = JSON.parse(fs.readFileSync(args.map, 'utf8'))
const computed = {}
const confusion = new Map()
let same = 0
let total = 0
for (const [key, columns] of Object.entries(observed)) {
  const [cx, cz] = key.split(',').map(Number)
  const mine = []
  for (let x = 0; x < 16; x++) {
    for (let z = 0; z < 16; z++) {
      // Résolution du climat : une case de 4 × 4 × 4 blocs (coordonnées en quarts de bloc), comme vanilla.
      const biome = source.getBiome((cx * 16 + x) >> 2, y >> 2, (cz * 16 + z) >> 2, state.sampler).toString()
      const id = javaToBedrock[biome] ?? -1
      mine.push(id)
      total++
      if (id === columns[x * 16 + z]) {
        same++
      } else {
        const label = `${biome} -> ${columns[x * 16 + z]}`
        confusion.set(label, (confusion.get(label) ?? 0) + 1)
      }
    }
  }
  computed[key] = mine
}

console.log(`${Object.keys(observed).length} chunks, y = ${y}`)
console.log(`colonnes identiques : ${same}/${total} (${(100 * same / total).toFixed(2)} %)`)
for (const [label, count] of [...confusion].sort((a, b) => b[1] - a[1]).slice(0, 15)) {
  console.log(`  ${label} : ${count}`)
}
if (args.out) {
  fs.writeFileSync(args.out, JSON.stringify(computed) + '\n')
  console.log(`écrit ${args.out}`)
}
