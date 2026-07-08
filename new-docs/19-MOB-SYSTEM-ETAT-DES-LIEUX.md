# 19 — Système de mobs : état des lieux & ce qui manque

> **Rapport d'audit** produit le 2026-07-08 par lecture exhaustive du code (4 audits parallèles :
> roster, spawn, combat/loot/cycle de vie, physique/pathfinding/sync). Chaque constat est sourcé
> `fichier:ligne`. Périmètre = **système de mobs** (IA, spawn, entités, combat, butin, cycle de vie,
> physique). Complète [`18-MOB-AI.md`](18-MOB-AI.md) (architecture) et son inventaire des comportements.
>
> Légende : ✅ fait · ⚠️ partiel/approximatif · ❌ manquant · 🐛 bug de correction.

---

## 0. Résumé exécutif — les points les plus importants

**🐛 P0 — CRITIQUE (à corriger en priorité) :**
1. **Le tick physique/IA des mobs tourne à 100 TPS au lieu de 20 TPS.** Le tick (`main.rs:2198`,
   dans la boucle `for tick_dim in active_dimensions`) est **hors** du gate `world_gametick_fired`
   (fermé `main.rs:1999`). La gravité/drag SONT scalés pour 100 TPS, mais **la vitesse horizontale,
   l'impulsion de saut, la friction et l'intervalle d'errance ne le sont PAS** → mobs **~5× trop
   rapides**, sautent **~5× trop haut** (apex ~5.5 blocs vs 1.1), re-tirent l'errance 5× trop souvent,
   timers (repro/despawn/sun-burn) 5× trop rapides, jusqu'à 100 paquets de mouvement/s par mob.
   **C'est la cause de fond du « les mobs bougent dans tous les sens / trop vite » au-delà des fixes
   NO_AI + errance.** Probable régression du refactor multi-dimension (le spawner, lui, est resté à
   20 TPS `main.rs:1932`). **Fix : gater le tick mob par `world_gametick_fired` (20 TPS), OU finir le
   scaling 100 TPS partout.**
2. **Sync client contradictoire** : on envoie `MoveActorAbsolute` **+** `SetActorMotion` à chaque tick
   sans le diff-gating de PMMP (`mob_entities.rs:1967-1982`, `main.rs:3088-3091`). Atténué par `NO_AI`
   mais reste incorrect (le motion fait extrapoler le client, l'absolute le corrige).

**⚠️ P1 — écarts majeurs de fidélité :**
- **Spawn** : light level **ignoré** (docstring mensonger), pas de spawn grotte/eau/structure, pas de
  `herd`/`density_limit`/`spawns_on_block`, mob cap approximatif (2 catégories).
- **Roster** : beaucoup de mobs = **reskins sans mécanique propre** (Warden sans sonic boom/sniff,
  Allay/Bee/Sniffer/Villager sans leur mécanique cœur, aucun taming/riding/trading).
- **Combat** : dégâts joueur→mob **hardcodés à 4.0** (pas d'arme), **pas d'XP à la mort**, pas d'armure
  mob, **aucun effet de statut** (poison/wither… modules écrits mais morts).

**Beaucoup de code écrit mais NON câblé** (mort) : `witch.rs`, `spider.rs`, `zombie.rs`, `effects.rs`,
`mob_xp.rs`, `experience_orb.rs`, `slime_spawning.rs`, `frog_spawn.rs`, `trial_spawner.rs`,
`spawner.rs`, `despawn_rules.rs`, `spawn_rules.rs`, `biome_spawning.rs`.

---

## 1. Physique / pathfinding / sync client

### ✅ Fait
- **A\* sol** : 8 voisins, heuristique octile, anti-corner-cutting, chemin partiel si inatteignable
  (`route.rs:25-34,71-75,167-187,213-217`). Step-up 1 / chute 3 / cap 100 nœuds (`route.rs:37-38,94-95`).
- **Gravité + air drag** correctement scalés 100 TPS (`mob_entities.rs:13-14`).
- **Dégâts de chute** conformes vanilla (`mob_entities.rs:1893-1907`).
- **Flag `NO_AI`** posé sur tous les mobs (`entity.rs:167-178`) — coupe l'IA client.

### ⚠️ Approximatif / faux
- 🐛 **Vitesse horizontale ~5× trop rapide** et **saut ~5× trop haut** à 100 TPS (`controller.rs:73,91-116,10`) — voir P0.
- **Friction sol/air** = valeurs 20-TPS appliquées à 100 TPS (`mob_entities.rs:18-19`) — surtout visible sur le knockback/coasting.
- **Collision = colonne centrale unique** (`mob_entities.rs:2077-2108`, `// suffisante pour la v1`) → faux pour les gros mobs au sol (ravager 1.95, spider 1.4…) qui coupent les coins.
- **Nage** : test `in_water` = **bloc des pieds seulement** (`mob_entities.rs:2051-2058`), ignore la colonne/profondeur.
- **Route recalculée 5× trop souvent** (cycle 20 ticks à 100 TPS = 0.2 s) ; waypoint `max_dist=0.75` constante (`mod.rs:393-395`).
- 🐛 **Double paquet de mouvement** (absolute + motion) chaque tick, sans gating PMMP — voir P0 #2.

### ❌ Manquant
- **Lissage de chemin** (funnel/LOS) — waypoints = centres de blocs bruts (`route.rs:242`).
- **Pathing** : portes (fermées = bloquées, pas d'ouverture), eau (pas de swim-path), escalade (spider ne grimpe pas), vol (les volants foncent en ligne droite **sans évitement d'obstacle**, `controller.rs:144-167`).
- **Collision entité-entité** (poussée entre mobs) — **totalement absente**.
- **Montures / riding** — absent.
- **Hitbox multi-parties** (ender dragon / wither) — une seule AABB (`mob_entities.rs:601-622`).

---

## 2. Spawn naturel

### ✅ Fait (le mieux couvert)
- **Poids** (tirage pondéré) `mob_spawner.rs:72,127-136`.
- **Habitat / dimension** (Nether/End ≠ overworld) `mob_spawner.rs:62-75`.
- **Filtre de biome** via tags du biome **réellement généré** (`biome_identifier_at`) + évaluateur
  `all_of`/`any_of`/`none_of`/OR-of-AND (`mob_spawner.rs:187-191`, `spawn_rules_vanilla.rs:89-140`).
- **Sol solide + 2 blocs d'air** `mob_spawner.rs:162-178`. **Nuit** (monstres, overworld) `mob_spawner.rs:148`.

### ⚠️ Partiel / faux
- 🐛 **Light level IGNORÉ** : `brightness_range` défini mais **jamais appelé** ; le docstring
  (`mob_spawner.rs:10-11`, `spawn_rules_vanilla.rs:6-8`) **prétend le contraire**. Les mobs spawnent
  sous les torches la nuit, et pas dans les grottes sombres le jour.
- **Surface seulement** : `is_surface_spawner`/`is_underground_spawner` jamais appelés ; scan = 1er bloc solide depuis Y=320.
- **Mob cap** global-par-dimension × nb joueurs, **2 catégories** (monster/animal), valeurs 12/6 loin du modèle vanilla (~70/10 par joueur, multi-catégories) `mob_spawner.rs:29-30,101-114`.
- **Despawn** : hostiles seulement (`mob_entities.rs:1707`) → **les passifs ne despawnent JAMAIS** (le cap animal sature à vie) ; **les mobs nommés/apprivoisés despawnent** (seul `is_boss` exempté) — bug vanilla.

### ❌ Manquant
- `minecraft:herd` (spawn en **groupe/pack**), `density_limit` (cap local), `spawns_on_block_filter`
  (vache→herbe, chauve-souris→pierre, axolotl→argile), `difficulty_filter`, `height_filter`.
- **Spawn aquatique** (aquatiques exclus `mob_spawner.rs:69`, eau sautée `:164`) → pas de squid/poissons/dauphin/drowned naturels.
- **Spawn en grotte/souterrain**, **spawn par structure** (village→chat/villageois, monument→guardian, forteresse→wither skeleton, bastion→piglin). Seul le blaze est hardcodé depuis un spawner block (`mob_spawner.rs:288-292`).
- **Slime chunks / slimes de marais** (module `slime_spawning.rs` mort → slimes spawnent partout), **frogs/tadpoles**, **trial spawners** (modules morts).

---

## 3. Combat / butin / attributs

### ✅ Fait
- **Mob→joueur** : dégâts + i-frames + knockback + hurt anim + mort/respawn (`main.rs:2741-2842`, port PvP `combat.rs`).
- **Difficulté** : multiplicateur global peaceful 0 / easy 0.67 / normal 1 / hard 1.5 (`main.rs:2663-2670`), sur mêlée + flèches.
- **Explosion creeper** : blocs + dégâts radiaux + particule/son (`main.rs:2847-2933`).
- **Butin data-driven** : `loot_table.rs` (7 tables vanilla, conditions + fonctions partielles) + fallback `default_loot`.
- **PV max par mob** (`mob_hp.rs`, ~90 entités) + **dégâts d'attaque de base** par mob (`desc()`).
- **Sons** ambient/hurt/death/shoot/explode (`main.rs:2415-2438,2700-2708,2862-2870`).

### ⚠️ Partiel
- **Joueur→mob** : dégâts **hardcodés 4.0** (pas d'arme/enchant/critique) `main.rs:3922` ; knockback OK ; **pas d'armure/défense mob** (soustraction brute `mob_entities.rs:1588`).
- **Loot** : `looting_level` hardcodé 0 (`mob_entities.rs:1620`), `killed_by_player:true` toujours vrai même sur mort environnementale ; conditions/fonctions non couvertes → certains loots ratés.
- **Difficulté** : un seul multiplicateur global, pas les courbes de difficulté régionale par mob.

### ❌ Manquant
- **Mob→mob** (golem de fer / loup vs hostile) — **aucune infrastructure de ciblage mob** (seuls les joueurs sont ciblés).
- **Effets de statut** (poison araignée, faim husk, wither, darkness warden…) — **aucun** ; modules `witch.rs`/`spider.rs`/`effects.rs` écrits mais **morts** (`combat.rs:120` `// TODO`).
- **XP à la mort** — `mob_xp.rs` + `spawn_xp_orb` existent mais **zéro appelant**.
- **Drops d'équipement** (armure zombie/skeleton), particules de hurt/mort, sons d'attaque dédiés.

---

## 4. Roster & comportements par mob

**73 `MobKind`** (`mob_entities.rs:79-239`), 11 profils d'IA. Voir le détail par profil et la
correspondance vanilla dans [`18-MOB-AI.md`](18-MOB-AI.md).

### ✅ Mécaniques bespoke présentes
Creeper (explosion), Skeleton/Stray/Bogged/Pillager (arc balistique + kite), Witch/Breeze/Shulker
(projectiles, bullet à tête chercheuse), Enderman (téléport blessé), Slime/MagmaCube (taille + split),
Sheep (tonte + brout), Creaking (fige si regardé), EnderDragon (phases), Guardian (rayon),
Ghast/Blaze/Wither (boules de feu). Sun-burn : **Zombie/Skeleton uniquement**.

### ⚠️ / ❌ Reskins sans mécanique propre (échantillon)
- **Warden** = simple `Melee` → **pas de sonic boom, sniff, darkness, dig/emerge, anger par vibration**.
- **Allay** (pas de ramassage/livraison d'items), **Bee** (pas de pollinisation/ruche/dard), **Sniffer**
  (pas de fouille de graines) — leur **mécanique cœur** manque.
- **Villager** (pas de métiers/commerce/réputation), **WanderingTrader** (pas de trades), **Piglin**
  (pas de bartering/aggro or), **Ravager** (pas de roar/raid), **IronGolem** (pas de défense de village).
- **Aucun taming** (loup/chat/cheval/perroquet), **aucun riding/saddle** (cochon/cheval/strider/llama),
  **aucun variant** (couleurs de mouton → toujours blanc `mob_entities.rs:1279-1282`, cat/rabbit/tropical fish…).
- Husk/Drowned/ZombieVillager (pas de spécificités : trident, conversion, hunger), CaveSpider (pas de poison), Silverfish (pas d'infestation).
- **Descs copiés-collés** : Husk=Drowned=ZombieVillager=Zombie, Hoglin=Zoglin, Stray=Bogged=Skeleton, Horse=Donkey=Mule, Cat=Ocelot, Squid=GlowSquid (`mob_entities.rs:259-302,380-401,446-478,885-939`).

### 🐛 Bugs de breeding
- **Marqués breedables à tort** (stériles en vanilla) : **Mule, Armadillo, Parrot** (`mob_entities.rs:907-917,1028-1038,1006-1016`).
- **Non-breedables alors qu'ils devraient l'être** : **Wolf, Goat, Llama** (catégorie `Neutral` → `is_breedable()` faux malgré leur `breeding_food`) ; **Hoglin, Villager** (food vide).
- **Tempt inopérant hors Passive** : les mobs `Neutral`/`Flying` (loup, chèvre, perroquet, abeille…) ne sont **jamais séduits** par leur nourriture (seul le profil Passive câble le tempt).

### ❌ Transformations manquantes
Zombie→drowned (eau), Pig→zombified piglin (foudre), Villager↔zombie_villager (kill/cure),
Mooshroom→cow (tonte), Skeleton→stray (poudreuse). **Aucune** n'est implémentée.

---

## 5. Code écrit mais MORT (à câbler ou supprimer)

Déclaré `pub mod` dans `main.rs` mais **jamais appelé** au runtime (souvent une meilleure version que le
code live) :

| Module | Contenu | Verdict |
|--------|---------|---------|
| `despawn_rules.rs` | Despawn vanilla (persistence/nom/tamed + 1/800) | À câbler (corrige le bug despawn) |
| `slime_spawning.rs` | Slime chunks (RNG Java) + marais | À câbler |
| `frog_spawn.rs` | Éclosion tadpole | À câbler |
| `spawner.rs` | Spawner block vanilla (delays/spawn_count) | À câbler dans `tick_spawners` |
| `trial_spawner.rs` | Machine à états trial spawner | À câbler |
| `witch.rs`, `spider.rs`, `zombie.rs`, `effects.rs`, etc. | Effets de statut par mob | À câbler dans le combat |
| `mob_xp.rs`, `experience_orb.rs` | XP à la mort | À câbler |
| `spawn_rules.rs` | « port conceptuel PMMP (pas implémenté) » | À supprimer (remplacé par `spawn_rules_vanilla`) |
| `biome_spawning.rs` | (en fait worldgen, pas mob) | Hors-sujet mob |

---

## 6. Backlog priorisé « ce qui manque »

### P0 — bugs de correction (à faire d'abord)
1. **Gater le tick mob à 20 TPS** (ou finir le scaling 100 TPS de la vitesse/saut/friction/intervalles). → règle le « trop rapide / bouge de partout ».
2. **Sync mouvement façon PMMP** : diff position/motion séparément, n'envoyer que le composant changé, à 20 TPS.
3. **Despawn** : câbler `despawn_rules.rs` (passifs despawnent, nommés/tamed persistent).

### P1 — fidélité à fort impact
4. **Light level au spawn** (brightness_range) + spawn grotte/souterrain.
5. **`herd`** (spawn en groupe) + **`spawns_on_block_filter`** + `density_limit`.
6. **XP orbs à la mort** (câbler `mob_xp` + `spawn_xp_orb`).
7. **Dégâts joueur→mob selon l'arme** (au lieu de 4.0) + armure/défense mob.
8. **Spawn aquatique** (squid/poissons/dauphin/drowned) + **spawn par structure** (dépend des structures : village, monument, forteresse, bastion).
9. **Effets de statut** (câbler witch/spider/wither…).

### P2 — comportements & polish
10. Behaviors vanilla manquants (voir tableau `18-MOB-AI.md`) : `breed`/`follow_parent` en pathfinding, `hurt_by_target` (riposte), `avoid_mob_type`, `look_at_player` dédié, `leap_at_target`.
11. **Mob→mob** (golem/loup vs hostile) — infra de ciblage d'entités.
12. **Taming** (loup/chat/cheval), **riding/saddle**, **variants** (couleurs mouton, etc.), **transformations**.
13. Corriger les **bugs de breeding** (Mule/Armadillo/Parrot stériles ; Wolf/Goat/Llama breedables).
14. Collision entité-entité, hitbox multi-parties (dragon/wither), lissage de chemin, pathing eau/porte/escalade.
15. Différencier les **descs copiés-collés** (hitbox/vitesse/HP par mob réels).

---

## 8. Journal de réparation (mise à jour au fil des corrections)

> Chaque entrée = un bloc corrigé + commit. Sert à ne pas perdre le fil.

- **[FAIT] P0 #1 — Scaling 100 TPS de la locomotion des mobs.** Le tick mob tourne au tick
  serveur (100 TPS) **volontairement** (comme items/flèches : `SERVER_TICKS_PER_SECOND=100`).
  Seules gravité/drag étaient scalées ; vitesse/saut/friction/timers ne l'étaient pas.
  Corrigé **sans toucher `main.rs`** :
  - `controller.rs` : vélocité Walk/Fly × `TICK_SCALE` (0.2) ; `JUMP_IMPULSE` 0.42→0.2 (apex ~1.25) ;
    `JUMP_COOL_DOWN` ×5.
  - `mob_entities.rs` : `GROUND_FRICTION`/`AIR_FRICTION` → puissance 1/5 (0.9029/0.9813) ;
    `DESPAWN_TIMER_MAX`/`LOVE_TICKS`/`BREED_COOLDOWN`/`BABY_GROW_TICKS` ×5 ; `WATER_BUOYANCY`/
    `WATER_MAX_RISE` re-tunés ; sun-burn (×5) ; brout mouton (×5).
  - `species.rs` : `MELEE_COOLDOWN`/`FIREBALL_COOLDOWN`/`GUARDIAN_CHARGE_TICKS`/`CREEPER_FUSE_TICKS`/
    `BOW_COOLDOWN` (+ variante projectile) ×5.
  - `executor.rs` : `MAX_STUCK_TICKS`/`ROAM_INTERVAL`/`DRAGON_CIRCLE_TICKS`/`DRAGON_STRAFE_TICKS`/
    `DRAGON_SHOOT_CD` ×5 ; `DRAGON_ANGULAR` ×0.2.
  - Facteur `SERVER_TICKS_PER_GAME_TICK = 5` documenté dans chaque fichier. Reste à valider en jeu.
  - **NON scalé encore** : vitesses des projectiles (flèche `BOW_ARROW_SPEED`, boule de feu
    `FIREBALL_SPEED`) — à traiter avec l'audit des projectiles (P1).

- **[FAIT] P0 #3 — Despawn.** Ajout de `MobKind::can_despawn()` (hostiles + aquatiques + bat
  despawnent ; animaux de ferme, neutres et boss **persistent**, conforme à la persistance vanilla).
  Le gate live passe de `is_hostile()` à `can_despawn()`. Persistance nommé/tamed = quand ces états
  existeront (taming non implémenté). `despawn_rules.rs` reste un module de référence non câblé.
- **[FAIT] P1 (partiel) — Spawn : bloc-support + groupes.** `spawn_rules_vanilla` : `spawns_on_block`
  (vache→grass_block, chauve-souris→pierres…) + `herd_size` (min/max). `mob_spawner` : sol extrait en
  helper `find_spawn_ground`, filtre bloc-support, et **apparition en groupe** (`herd`) en grappe
  autour du point validé (respecte le cap). Docstring mensonger corrigé.
- **[BLOQUÉ] P1 — Light level au spawn.** Nécessite un **moteur de lumière** (light stocké/requêtable)
  absent. Gate nuit global conservé pour les monstres ; `brightness_range` reste défini mais non
  utilisable tant que le light engine n'existe pas. Documenté honnêtement dans le docstring du spawner.

- **[FAIT] P0 #1 (suite) — Projectiles scalés 100 TPS.** Flèches & boules de feu tickent aussi au
  tick serveur. `arrow_entity.rs` : `ARROW_GRAVITY` ×0.2, `ARROW_DRAG` ^1/5 (0.998), `ARROW_MAX_LIFE`
  ×5, grâce anti-auto-collision ×5, `HOMING_STEER` 0.25→0.056. `executor.rs` : `ARROW_GRAVITY_COMP`
  ×0.2 (visée). `species.rs` : `BOW_ARROW_SPEED` 1.4→0.28, `FIREBALL_SPEED` 0.8→0.16. Trajectoire
  balistique préservée (5× plus de ticks × gravité/5). Projectiles ~5× moins rapides = réaliste.

- **[FAIT] P2 #13 — Bugs de breeding.** `is_breedable()` exclut désormais les stériles vanilla
  (mule/tatou/perroquet). (Wolf/Goat/Llama breedables = encore bloqué par leur catégorie `Neutral` ;
  à traiter avec le refactor breedable data-driven.)
- **[FAIT] Sun-burn fidèle.** `MobKind::burns_in_daylight()` : zombie, zombie_villager, drowned (au
  sec), skeleton, stray, bogged brûlent au soleil ; husk & wither_skeleton exclus (avant : seulement
  zombie/skeleton).

- **[FAIT] P1 #7 (partiel) — Dégâts joueur→mob selon l'arme.** Plus de `4.0` en dur :
  `weapon_stats::player_attack_damage(held_item_id)` (poing 1.0, épée/hache/pioche/pelle/houe via les
  tables PMMP). Câblé au site d'attaque dans `main.rs`. **Reste** : enchant/critique/cooldown d'attaque,
  armure/défense côté mob (soustraction encore brute), trident/masse.

## 7. Méthode & limites de ce rapport

- Établi par **lecture du code** (pas d'exécution en jeu). Les constats « trop rapide / trop haut » sont
  déduits des constantes + de la cadence de tick, **non encore validés manette en main**.
- Périmètre = mobs. N'inclut pas : worldgen (autre chantier), structures (dont dépendent certains spawns),
  redstone, fluides.
- Voir aussi [`18-MOB-AI.md`](18-MOB-AI.md), [`18-MOB-AI-PROGRESS.md`](18-MOB-AI-PROGRESS.md),
  [`99-ROADMAP.md`](99-ROADMAP.md).
