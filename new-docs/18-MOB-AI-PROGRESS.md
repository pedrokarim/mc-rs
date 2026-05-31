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
- 💤 A4 — Swing de bras mêlée : **différé** (nécessite `AnimatePacket`, absent du proto ; visuel mineur).
- ✅ A5 — Dégâts mêlée + flèche mis à l'échelle par difficulté (peaceful 0 / easy 0.67 / normal 1 / hard 1.5).

## Phase 2 — IA hostile (fidélité Bedrock) ✅ (B2 différé)
- ✅ B1 — Sun-burning : zombie/skeleton prennent feu (flag `ONFIRE` + SetActorData) + 1 HP/s en
  plein jour s'ils sont exposés au ciel (≤64 blocs) et hors de l'eau. Dégâts via le chemin
  `apply_mob_damage_broadcast` (son + mort + drops).
- 💤 B2 — Ligne de vue (LOS) : **différé** — gain mineur (l'A\* bloque déjà aux murs, donc un mob
  ciblant à travers un mur reste coincé contre, sans le traverser).
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
- 💤 C2 — Reproduction (feed 2 adultes → bébé) — nécessite l'interaction clic-droit sur entité (non câblée).
- 💤 C3 — Mouton qui mange l'herbe (regagne sa laine) — nécessite l'état laine + changement de bloc.
- 💤 C4 — Bébés mobs : **différé** — flag `BABY` non défini dans le proto + pas de source de bébés
  sans reproduction (C2). À traiter avec C2.

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

### Reste (stretch / chantiers séparés)
- A4 swing-bras (AnimatePacket), B2 LOS (raycast), C2 reproduction + C4 bébés, C3 mouton-herbe.
- Extension du roster de mobs (spider, enderman, slime, …) = chantier « entités », pas « IA ».
