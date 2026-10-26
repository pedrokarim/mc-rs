# Scripts

Scripts annexes (Python, et un script PHP qui exécute PMMP). Aucun n’est nécessaire pour compiler ou lancer le serveur.

## Import de données vanilla

À relancer à chaque changement de version du jeu (voir
[`../dev-docs/31-PROTOCOL-2169.md`](../dev-docs/31-PROTOCOL-2169.md), procédure, et
[`../dev-docs/33-PROTOCOL-2193.md`](../dev-docs/33-PROTOCOL-2193.md)).

- `import_bds_recipes.py <capture> --version <bds>` : régénère `crates/mc-rs-server/data/recipes/bds.json`
  depuis une capture de BDS décodée en mode typé (`bds-capture decode -typed`) ; `--check` compare au fichier
  actuel. Remplace l'ancien import de bedrock-samples.
- `import_bds_trim_data.py <capture> --version <bds>` : garnitures d'armure (TrimData) de BDS →
  `crates/mc-rs-server/data/trim_data.json`.
- `import_bds_packets.py <capture>/s2c.bin --version <bds>` : paquets de pure donnée de BDS
  (JigsawStructureData, VoxelShapes) renvoyés tels quels → `crates/mc-rs-server/data/bds_packets/`.
- `extract_biomes.py` : régénère `crates/mc-rs-server/data/biomes.json` (climat, règles multinoise,
  surface) depuis `.reference/bedrock-samples/behavior_pack/biomes` (version du checkout courant) et les
  IDs de `biome_id_map.json` de BedrockData (PMMP, arrêté à 1.26.30).
- `pmmp/export_block_break_data.php` : exécute PMMP (avec son PHP, `.reference/PocketMine-MP/bin/php/php.exe`)
  pour exporter dureté, outil, drops et XP de chaque état de la palette, croisés avec le dump de BDS d'Allay →
  `crates/mc-rs-server/data/block_break.json`. À relancer à chaque changement de palette (voir
  [`../dev-docs/36-CASSAGE-DE-BLOCS.md`](../dev-docs/36-CASSAGE-DE-BLOCS.md)).
- `import_item_upgrade_schema.py [dossier]` : consolide les schémas de `BedrockItemUpgradeSchema` dans
  `crates/mc-rs-server/data/item_id_upgrade_schema.json` (migration des anciens identifiants d’items).

## Outils de développement (`dev/`)

- `dev/test_motd.py` : interroge un serveur (Unconnected Ping) et affiche son MOTD ; `--compare` compare
  mc-rs (19132) et BDS (19133).
- `dev/udp_proxy.py` : relais UDP 19133 → 19132, pour contourner la restriction de boucle locale des
  applications UWP quand le client tourne sur la même machine.

## Observation de BDS (`dev/`)

Voir [`../dev-docs/34-BDS-RETRO-INGENIERIE.md`](../dev-docs/34-BDS-RETRO-INGENIERIE.md).

- `dev/bds_capture/` (Go) : capture les paquets échangés avec un serveur (BDS en NetherNet, mc-rs en RakNet),
  les décode en JSON avec gophertunnel, et relève les drops de blocs calculés par BDS (`drops`).
- `dev/packet_dump.py` : résumé par type d'un enregistrement de paquets, extraction de charges utiles.
- `dev/compare_captures.py` : comparaison champ par champ de deux captures décodées.
- `dev/summarize_bds_drops.py <relevé.jsonl> [--diff] [--json bilan.json]` : bilan d'un relevé de drops de BDS
  (`bds-capture drops`), blocs aléatoires, écarts avec `data/block_break.json`.
- `dev/bds_palette.py` : palette de mc-rs vue par BDS (états typés, hash réseau, syntaxe des commandes) ;
  lancé seul, vérifie les hashes contre le dump de BDS d'Allay.
- `dev/plan_bds_drops.py --out <plan.json> [--samples N] [--only <relevé.jsonl>]` : plan du relevé des drops
  par état de la palette (états regroupés hors orientation et connexions, 16 outils).
- `dev/build_bds_drops.py --groups <plan>.groups.json <relevé.jsonl>…` : écrit
  `crates/mc-rs-server/data/block_drops_bds.json` (drops de BDS par état). Chaîne complète :
  [`../dev-docs/36-CASSAGE-DE-BLOCS.md`](../dev-docs/36-CASSAGE-DE-BLOCS.md) § 2 bis.
- `dev/chunk_biomes.py <s2c.jsonl> [--y 64] [--json carte.json] [--ascii]` : biomes des chunks d'une capture
  décodée (BDS ou mc-rs), comptes par biome et carte `{"cx,cz": [256 biomes]}`.
- `dev/deepslate_biomes.mjs --deepslate <dossier> --seed <seed> --map <carte.json>` (Node) : biomes de la même
  zone calculés par deepslate (worldgen Java) avec les données de mc-rs, et comparaison. Parité des biomes :
  [`../dev-docs/14-WORLD-GENERATION.md`](../dev-docs/14-WORLD-GENERATION.md), « Parité avec BDS ».
- `dev/world_block_classes.py extract <db> <sortie.json>` / `compare <a.json> <b.json>` : classes de blocs
  (solide, eau, lave, air) des chunks d'un monde Bedrock enregistré (BDS), au format du test ignoré
  `dump_chunk_classes` de mc-rs, et comparaison ; parité de l'eau dans `dev-docs/14-WORLD-GENERATION.md`.
- `dev/bds_load_area.py <dossier BDS> <x1> <z1> <x2> <z2>` : fait générer et enregistrer une zone à BDS
  (`tickingarea`), pour la comparer à mc-rs.
- `dev/build_block_liquid.py` : écrit `crates/mc-rs-server/data/block_liquid.json` (blocs qui peuvent contenir de
  l'eau, réaction à l'eau qui coule), d'après le dump de BDS d'Allay.
- `dev/bds_pe_index.py` : index des sections, imports et descripteurs RTTI d'un exécutable PE (peu utile sur
  BDS, qui n'a presque pas de RTTI).
- `dev/ghidra/` : scripts Ghidra pour lire le binaire de BDS (fonctions qui utilisent une chaîne, méthodes
  d'une classe), pré-script qui désactive des analyseurs, suivi d'une analyse en cours.

Les benchmarks sont dans [`../benchmarks/`](../benchmarks/).
