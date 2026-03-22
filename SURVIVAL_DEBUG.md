# Survival Mode Debug Tracker

## État actuel (dernière mise à jour)
- Connexion : OK
- Bulles d'eau : FIXÉ (flag BREATHING bit 35)
- Fly/flottement : PARTIELLEMENT FIXÉ (gravité fonctionne quand NO_AI=false)
- Skin du joueur : CASSÉ (disparaît quand SetActorData envoyé post-spawn)
- Block breaking : CASSÉ (ne marche plus en survival)
- Position spawn : OK (pieds sur le sol)

## Tests effectués

| Test | Résultat | Notes |
|------|----------|-------|
| Sans SetActorData | Skin OK, fly, bulles | Pas de gravité, bulles |
| SetActorData dans PreSpawn (NO_AI=true) | Pas de crash, gelé au sol, pas de bulles | OK mais immobile |
| SetActorData post-spawn (NO_AI=false) | Skin disparaît | Le skin du joueur local disparaît |
| UpdateAdventureSettings | CRASH | Format incorrect — NE PAS ENVOYER |
| SetActorData + UpdateAdventureSettings | CRASH | UpdateAdventureSettings cause le crash |

## Bugs à résoudre

### 1. Skin disparaît après SetActorData post-spawn
- **Cause probable** : SetActorData avec les metadata réinitialise les flags du joueur local côté client, et le client perd la référence au skin
- **Solution à tester** : Ne pas envoyer SetActorData au joueur local pour ses propres metadata ? Ou envoyer un paquet différent pour enlever NO_AI ?
- **Alternative** : Ne PAS utiliser NO_AI pendant le pre-spawn. Envoyer SetActorData UNE seule fois avec les bons flags (sans NO_AI) dès le début.

### 2. UpdateAdventureSettings crash
- **Cause** : Format incorrect
- **Status** : DÉSACTIVÉ — pas critique pour le moment

### 3. Block breaking en survival
- **Cause** : Probablement lié aux abilities ou au gamemode
- **Status** : À investiguer après fix du skin

## Analyse approfondie (2026-03-22)

### Abilities values CORRECTES
- abilities_set = 0x000FFFFF (bits 0-19) — identique à PMMP
- abilities_values = 0x0000003F (bits 0-5) — identique à PMMP
- FLYING=false, ALLOW_FLIGHT=false dans les values

### Paquets manquants (par rapport à PMMP)
- **InventoryContent** — PMMP envoie l'inventaire complet pendant PreSpawn
- **MobEquipment** — PMMP envoie l'item en main
- **CreativeContent** avec les vrais items — on envoie vide

### Théorie actuelle
Le client Bedrock reste en mode créatif parce qu'on n'envoie pas les paquets d'inventaire.
Sans inventaire, le client ne peut pas déterminer le gamemode correctement et fallback en créatif.

### Prochaine action
Implémenter les paquets d'inventaire basiques (InventoryContent vide pour les 36 slots + armor).
