# 18 - Mob AI — Suivi d'avancement (fidélité Bedrock)

> Doc **vivante** de cadrage/suivi. Architecture : [`18-MOB-AI.md`](18-MOB-AI.md).
> Branche `feat/mob-ai` (worktree `mc-rs-mobai`). Référence comportementale : **Allay** + vanilla
> Bedrock (PMMP n'a pas d'IA de mobs). Mise à jour au fil de l'implémentation.

Légende : ✅ fait · 🔨 en cours · ⬜ à faire · 💤 stretch/hors-scope immédiat

## Socle (déjà livré)
- ✅ Framework générique (sensors → behaviors priorisés → controllers, mémoire, A\* sol)
- ✅ Collision horizontale + step-up + `on_ground` dans la physique des mobs
- ✅ Zombie (mêlée), Skeleton (arc + entité flèche), Creeper (explosion)
- ✅ Passifs : errance (roam) + fuite si blessé (panic)
- ✅ Dégâts mob→joueur via `AiEffect` (chemin combat partagé)

## Phase 1 — Feedback audiovisuel & dégâts (fidélité combat) ✅
IDs autoritatifs (BedrockProtocol) : LevelEvent `PARTICLE_EXPLODE=2025` ; LevelSoundEvent
`AMBIENT=10`, `DEATH=14`, `HURT=17`, `ATTACK=41`, `EXPLODE=48`, `SHOOT=54`.
- ✅ A1 — Sons de mob : hurt + death (chemin PvE), ambient (périodique ~12s/mob). (attack : couvert
  par le son d'impact côté joueur ; son d'attaque dédié non émis — mineur.)
- ✅ A2 — Explosion creeper : particule `PARTICLE_EXPLODE` + son `EXPLODE`.
- ✅ A3 — Arc : son `SHOOT` au tir + son d'impact (`HIT`) à la touche.
- ✅ A4 — Swing de bras mêlée : `combat_packets::arm_swing` (AnimatePacket 0x2C) broadcast sur
  l'attaque mêlée uniquement.
- ✅ A5 — Dégâts mêlée + flèche mis à l'échelle par difficulté (peaceful 0 / easy 0.67 / normal 1 / hard 1.5).

## Phase 2 — IA hostile (fidélité Bedrock) ✅ (B2 différé)
- ✅ B1 — Sun-burning : zombie/skeleton prennent feu (flag `ONFIRE` + SetActorData) + 1 HP/s en
  plein jour s'ils sont exposés au ciel (≤64 blocs) et hors de l'eau. Dégâts via le chemin
  `apply_mob_damage_broadcast` (son + mort + drops).
- ✅ B2 — Ligne de vue (LOS) : `NearestPlayerSensor` ne cible que si le raycast voxel mob→joueur
  est dégagé (le `Sensor` reçoit désormais le `ChunkCache`).
- ✅ B3 — Despawn des hostiles : >128 blocs instantané, >32 blocs pendant ~30 s → despawn
  (pas de despawn si aucun joueur connecté).
- ✅ B4 — Regard vers le joueur à l'arrêt : le roam fixe le joueur proche ; LookController passif
  passé en (cible+route).

## Phase 3 — Physique des mobs (fidélité) ✅
- ✅ D1 — Dégâts de chute : accumulation de la descente, dégâts à l'atterrissage
  (1/bloc au-delà de 3), annulés dans l'eau. Appliqués via `damage_requests`.
- ✅ D2 — Flottaison : poussée vers le haut dans l'eau (le mob remonte/nage, ne coule plus).
- ✅ D3 — Knockback du mob frappé par un joueur (`MobEntityManager::apply_knockback`,
  non écrasé par le WalkController grâce à sa garde anti-knockback).

## Phase 4 — IA passive (fidélité)
- ✅ C1 — Tempt : le mob passif suit le joueur (≤10 blocs) tenant sa nourriture (blé/carotte/graines).
  `PlayerSnapshot.held_item` + `TemptExecutor` + `ClosureEvaluator`. Priorité Flee(4) > Tempt(3) > Roam(1).
- ✅ C2 — Reproduction : clic-droit avec la nourriture (interaction entité action 0) → mode amour
  (`feed_mob`, consomme l'item en survie) ; 2 adultes en amour proches (≤3) → bébé (`try_breed`),
  cooldown 5 min. Priorité Flee > Tempt > Roam (la reproduction se résout dans le tick manager).
- ✅ C3 — Tonte + mouton mange l'herbe : clic-droit avec des **shears** → laine + flag `SHEARED` ;
  un mouton tondu broutant un `grass_block` finit par le manger (→ `dirt`, broadcast UpdateBlock) et
  **regagne sa laine**. `shear_sheep` + `eat_grass_timer` + `TickResult.block_changes`.
- ✅ C4 — Bébés mobs : flag `BABY` (bit 11) + échelle 0.5 à la naissance ; croissance → adulte
  après ~20 min (clear flag + échelle 1.0 + SetActorData). Source : reproduction (C2).

## Hors-scope de cette passe (chantiers séparés)
- 💤 Extension du roster de mobs (spider, enderman, slime, …) — ~60 types : réseau, modèles,
  loot, règles de spawn par espèce. C'est un chantier « roster d'entités », pas « IA ».
- 💤 Redstone, fluides (autres follow-ups projet, non liés aux mobs).

## Journal
- **Phase 1** — feedback AV (sons hurt/death/ambient, explosion particule+son, arc shoot+impact)
  + dégâts mêlée/flèche par difficulté. A4 (swing-bras) différé (AnimatePacket absent).
- **Phase 2** — sun-burning zombie/skeleton (ONFIRE + 1 HP/s en plein jour exposé), despawn des
  hostiles (>128 / >32 pendant 30 s), regard joueur à l'arrêt. B2 (LOS) différé.
- **Phase 3** — fall damage, flottaison dans l'eau, knockback du mob frappé. Refactor
  `apply_mob_damage_broadcast` (PvE + feu + chute).
- **Phase 4** — tempt (suivre la nourriture). C2 reproduction / C3 mouton-herbe / C4 bébés = stretch.

**Toute la checklist IA est faite** : A1–A5, B1–B4, C1–C4, D1–D3 + tonte/mouton-herbe + spawn naturel
de bébés.

## Extension du roster (chantier « entités », par vagues)
`MobKind` est devenu une **table de descripteurs** (1 ligne/mob : id, nom, hitbox, catégorie, profil
IA, dégâts, vitesse). Santé via `mob_hp`, butin via `loot_table`, spawn via `spawn_rules_vanilla`.
Roster passé de 7 à **~51 mobs**.
- ✅ **Vague 1** — mêlée (husk, drowned, zombie_villager, spider, cave_spider, silverfish, endermite,
  wither_skeleton, vindicator, ravager, hoglin, zoglin, piglin_brute), arc (stray, bogged, pillager),
  neutres (zombified_piglin, piglin, iron_golem, snow_golem, wolf, polar_bear, goat, llama), passifs
  (rabbit, horse, donkey, mule, cat, ocelot, fox, panda, turtle, villager, wandering_trader, camel,
  armadillo, sniffer, mooshroom).
- ✅ **Vague 2** — slime + magma_cube : taille (VARIANT) + **split à la mort**.
- ✅ **Vague 3** — volants : bat, parrot, allay, bee (vol 3D, pas de gravité).
- ✅ **Vague 4** — enderman : mêlée + **téléportation quand blessé** (vers un sol proche, flag teleport).

- ✅ **Vague 5** — tireurs volants : ghast + blaze. Entité projectile généralisée (`arrow_entity`
  porte aussi les boules de feu : gravité/collage configurables) ; `FlyShootExecutor` (vol
  stationnaire + tir périodique) ; `AiEffect::ShootFireball`. Dégâts directs à l'impact (explosion
  de la fireball = follow-up).

- ✅ **Vague 6** — volants hostiles : phantom + vex (`FlyingMelee` : vole vers le joueur via
  `FlyAttackExecutor` + frappe au contact).

**Profils couverts (57 mobs)** : tous les mobs réutilisant un profil existant (Melee, Bow,
CreeperSwell, Passive, Neutral, Flying, FlyingShooter, FlyingMelee) sont faits. Les restants
demandent **chacun une mécanique bespoke** :

- ✅ **Vague 7 — Bosses** : barre de boss (`BossEventPacket` 0x4A : SHOW/HEALTH/HIDE, encodeurs
  `combat_packets::boss_*`). **Wither** (vol + wither skulls via FlyingShooter, barre violette) +
  **Ender Dragon** (vol + fonce via FlyingMelee, barre rose). PV élevés (mob_hp : 300/200). main.rs
  diffuse SHOW (porte la santé) chaque tick + HIDE à la disparition. Follow-ups : séquence de spawn
  du wither + armure, phases/end-crystals du dragon, hitbox multi-parties.

- ✅ **Vague 8 — Witch** : jet de potions (Bow généralisé → `with_projectile` émet un projectile
  potion au lieu d'une flèche). Immunité aux potions = follow-up.

### Vagues restantes (mécaniques bespoke)
- ⬜ **Shulker** : stationnaire, projectile à tête chercheuse.
- ⬜ **Guardian/Elder Guardian** : rayon laser + nage dédiée (aquatique).
- ⬜ **Breeze** (wind charge), **warden** (capteur de vibrations + sonic boom), **creaking**.
- ⬜ **Nage dédiée** pour squid/dauphin/poissons (profil passif terrestre pour l'instant).
- ⬜ **Règles de spawn par biome** (le spawner ne filtre pas par biome → husk/drowned/stray
  peuvent apparaître hors biome ; à raffiner quand le spawner lira le biome au point de spawn).
