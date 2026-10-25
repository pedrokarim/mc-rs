# Scripts

Scripts Python annexes. Aucun n’est nécessaire pour compiler ou lancer le serveur.

## Import de données vanilla

À relancer à chaque changement de version du jeu (voir
[`../dev-docs/31-PROTOCOL-2169.md`](../dev-docs/31-PROTOCOL-2169.md), procédure, et
[`../dev-docs/33-PROTOCOL-2193.md`](../dev-docs/33-PROTOCOL-2193.md)).

- `import_recipes.py <tag>` : régénère `crates/mc-rs-server/data/recipes/vanilla.json` depuis
  `.reference/bedrock-samples` au tag donné (ex. `v1.26.50.4`) ; `--check` compare au fichier actuel.
- `extract_biomes.py` : régénère `crates/mc-rs-server/data/biomes.json` (climat, règles multinoise,
  surface) depuis `.reference/bedrock-samples/behavior_pack/biomes` (version du checkout courant) et les
  IDs de `biome_id_map.json` de BedrockData (PMMP, arrêté à 1.26.30).
- `import_item_upgrade_schema.py [dossier]` : consolide les schémas de `BedrockItemUpgradeSchema` dans
  `crates/mc-rs-server/data/item_id_upgrade_schema.json` (migration des anciens identifiants d’items).

## Outils de développement (`dev/`)

- `dev/test_motd.py` : interroge un serveur (Unconnected Ping) et affiche son MOTD ; `--compare` compare
  mc-rs (19132) et BDS (19133).
- `dev/udp_proxy.py` : relais UDP 19133 → 19132, pour contourner la restriction de boucle locale des
  applications UWP quand le client tourne sur la même machine.

## Observation de BDS (`dev/`)

Voir [`../dev-docs/34-BDS-RETRO-INGENIERIE.md`](../dev-docs/34-BDS-RETRO-INGENIERIE.md).

- `dev/bds_capture/` (Go) : capture les paquets échangés avec un serveur (BDS en NetherNet, mc-rs en RakNet)
  et les décode en JSON avec gophertunnel.
- `dev/packet_dump.py` : résumé par type d'un enregistrement de paquets, extraction de charges utiles.
- `dev/compare_captures.py` : comparaison champ par champ de deux captures décodées.
- `dev/ghidra/` : scripts Ghidra pour lire le binaire de BDS (fonctions qui utilisent une chaîne, méthodes
  d'une classe).

Les benchmarks sont dans [`../benchmarks/`](../benchmarks/).
