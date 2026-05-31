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

## Phase 2 — IA hostile (fidélité Bedrock)
- ⬜ B1 — Sun-burning : zombie/skeleton prennent feu (flag `ONFIRE`) + dégâts de feu en plein
  jour s'ils sont exposés au ciel (et pas dans l'eau / sous un bloc).
- ⬜ B2 — Ligne de vue (LOS) : ne cible que si le joueur est visible (raycast blocs).
- ⬜ B3 — Despawn des hostiles : retrait quand trop loin de tout joueur (Bedrock : >128 instantané,
  32–128 aléatoire dans le temps).
- ⬜ B4 — Regard vers le joueur à l'arrêt (LookAtPlayer quand on erre près d'un joueur).

## Phase 3 — Physique des mobs (fidélité)
- ⬜ D1 — Dégâts de chute (fall damage) pour les mobs.
- ⬜ D2 — Flottaison/nage dans l'eau (le mob remonte au lieu de couler).
- ⬜ D3 — Knockback du mob quand un joueur le frappe.

## Phase 4 — IA passive (fidélité)
- ⬜ C1 — Tempt : suivre un joueur tenant la nourriture de reproduction de l'espèce.
- 💤 C2 — Reproduction (feed 2 adultes → bébé) — nécessite l'interaction clic-droit sur entité.
- 💤 C3 — Mouton qui mange l'herbe (regagne sa laine).
- ⬜ C4 — Bébés mobs (flag baby + échelle + vitesse) via spawn.

## Hors-scope de cette passe (chantiers séparés)
- 💤 Extension du roster de mobs (spider, enderman, slime, …) — ~60 types : réseau, modèles,
  loot, règles de spawn par espèce. C'est un chantier « roster d'entités », pas « IA ».
- 💤 Redstone, fluides (autres follow-ups projet, non liés aux mobs).

## Journal
- (rempli au fil de l'eau ci-dessous : commit + ce qui a été fait)
