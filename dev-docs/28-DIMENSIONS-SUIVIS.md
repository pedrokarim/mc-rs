# Dimensions — éléments à faire après (suivis)

Liste des éléments restants pour finir les dimensions (Nether / End), après le gros
chantier génération + forteresse + block entities (fait). Réf chantier :
[`26-DIMENSIONS-CADRAGE.md`](26-DIMENSIONS-CADRAGE.md) ·
spec génération : [`27-DIMENSIONS-GENERATION-SPEC.md`](27-DIMENSIONS-GENERATION-SPEC.md).

## ✅ Déjà fait
- Terrain + biomes Nether/End (calibré Bedrock via Nukkit ; densités, surfaces, ores,
  végétation, fungi, deltas, glowstone…).
- End : piliers vanilla + cristaux, chorus, carte complète (centre/anneau/îles externes).
- **Forteresse du Nether** (générateur procédural) : ponts, couloirs, salles, nether wart.
- **Sous-système block entities** : persistance LevelDB + sync client.
- **Coffres de forteresse** : butin `nether_bridge` (consommé à l'ouverture).
- **Spawners de forteresse** : tick → apparition des blazes.

## 🔜 À faire après

### A. Structures du Nether
- [ ] **Bastion remnant** — piglins, blocs dorés / gilded blackstone, ponts, coffres de
  butin (`bastion_*`). Géométrie complexe (plusieurs variantes de salles).
- [ ] **Ruined portals** — portails en ruine (Nether **et** Overworld) : cadre d'obsidienne
  partiel, crying obsidian, or, lave, netherrack, coffre de butin enterré.
- [ ] **Nether fossils** — squelettes en `bone_block` dans les soul sand valleys.

### B. Block entities — consolidation
- [ ] **Persistance du `chest_manager`** : actuellement runtime-only → les coffres
  (forteresse incluse) se **re-remplissent au redémarrage** et les modifs joueur sont
  perdues. Persister les inventaires de coffres (clé LevelDB ou via les block entities).
- [ ] **Spawner générique** : mapping `EntityIdentifier` → `MobKind` (au lieu du blaze
  codé en dur) pour réutiliser les spawners avec d'autres mobs / structures.
- [ ] (Option) **Délai par spawner** : registre runtime du compte à rebours (aujourd'hui
  approximé par proba/tick) pour coller exactement à `MinSpawnDelay`/`MaxSpawnDelay`.

### C. Mobs
- [ ] **Vérifier en jeu** le spawn des mobs du Nether (piglin, zombified piglin, ghast,
  magma cube, blaze, hoglin, strider, enderman) et de l'End.

### D. End — fin de la dimension
- [ ] **End gateways** : après la mort du dragon, anneau de gateways (r ≈ 96) → TP vers
  les îles externes + retour (`end_gateway_return`, exit `[100,50,0]`).
- [ ] **End city + end ship** : structures des îles externes (butin, elytra, shulkers).
- [ ] **end_islands seed-exact Java** (LegacyRandom + SimplexNoise vanilla) si on veut la
  parité de graine exacte (actuel : forme correcte, pas bit-identique).

### E. Polish génération (optionnel)
- [ ] Surface rules détaillées 100 % data-driven si on veut s'éloigner des passes codées.
- [ ] Lanternes / chaînes (viennent avec les structures).

---
**Règle de travail rappelée** : pour tout format binaire / NBT / wire, **recouper PMMP ou
Nukkit avant d'implémenter** — ne jamais écrire de mémoire (cf. divergence `DisplayEntity*`
du MobSpawner attrapée en re-vérifiant).
