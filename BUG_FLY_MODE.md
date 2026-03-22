# BUG CRITIQUE : Le joueur fly en mode Survival

## Symptôme
Le joueur est censé être en mode Survival (gamemode=0) mais peut voler en appuyant sur Espace. Double-clic sur Espace active le fly comme en Créatif. Le joueur monte indéfiniment et ne retombe jamais.

## Ce qui a été essayé (TOUT échoué)

1. **StartGame gamemode=0** (survival) — Envoyé, mais le client reste en créatif
2. **UpdateAbilities avec FLYING=false, ALLOW_FLIGHT=false** — Abilities correctes (identiques à PMMP, 0xFFFFF/0x3F), mais le client les ignore
3. **SetPlayerGameType(0)** envoyé après spawn — Aucun effet
4. **SetActorData avec HAS_GRAVITY flag** — Aucun effet sur le fly (fixe les bulles d'eau)
5. **Bit 19 VERTICAL_FLY_SPEED ajouté** — Aucun effet
6. **Ordre des paquets réorganisé** comme PMMP — Aucun effet
7. **Paquets d'inventaire** (InventoryContent + MobEquipment) envoyés — Aucun effet
8. **Anti-fly clamp serveur** — Patch symptomatique, pas un fix
9. **enableNewInventorySystem true/false** — Aucun effet

## Ce qui fonctionne
- Le gamemode dans StartGame est 0 (survival)
- Les abilities_set = 0x000FFFFF (identique PMMP)
- Les abilities_values = 0x0000003F (identique PMMP)
- FLYING=false, ALLOW_FLIGHT=false dans les values
- Les paquets sont envoyés dans le bon ordre PMMP
- UpdateAttributes envoyé (health, hunger, etc.)

## Piste non explorée
- **Comparer les bytes exacts** de notre UpdateAbilities avec un dump réseau de PMMP (packet sniffer)
- **Le format de l'AbilitiesLayer** a peut-être un champ qu'on n'encode pas (comme `layerId` qui pourrait être `0` au lieu de `1`)
- **Le client Bedrock Windows** a peut-être un comportement différent de ce qu'on attend
- **Un paquet qu'on n'envoie pas** du tout et qui est requis pour que le client respecte le survival mode

## Contournement temporaire
Le serveur clamp la position Y quand le joueur monte trop vite (>1.5 blocs/tick). C'est un hack, pas un fix.

## Priorité
CRITIQUE — rend le jeu injouable en survival.
