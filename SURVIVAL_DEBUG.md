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

## Prochaine action
Essayer : Envoyer SetActorData UNE seule fois dans PreSpawn SANS NO_AI. Pas de SetActorData post-spawn. Ça devrait garder le skin et avoir la gravité.
