# bds-capture

Observe un serveur Bedrock de référence (BDS) avec gophertunnel, et produit des enregistrements comparables
à ceux de mc-rs (`MCRS_DUMP_PACKETS=full`). Méthode et constats : [`dev-docs/34-BDS-RETRO-INGENIERIE.md`](../../../dev-docs/34-BDS-RETRO-INGENIERIE.md).

## Construire

Go 1.25 ou plus récent :

```powershell
cd scripts\dev\bds_capture
$env:GOARCH = "amd64"; go build -o bds-capture.exe .
```

## Utiliser

```text
bds-capture capture -network nethernet -addr http://127.0.0.1:19150 -out <préfixe> -stay 20s
bds-capture capture -network raknet -addr 127.0.0.1:19132 -out <préfixe> -stay 20s
bds-capture decode -in <préfixe>_s2c.bin -direction s2c -out <préfixe>_s2c.jsonl
bds-capture drops -bds <chemin>/bedrock_server.exe -plan <plan.json> -out <relevé.jsonl>
```

- `capture` se connecte comme un client et écrit `<préfixe>_s2c.bin` (serveur → client) et `<préfixe>_c2s.bin`
  (client → serveur), au format `[u64 ms][u32 id][u32 longueur][charge utile]` little-endian.
- `decode` décode chaque paquet avec gophertunnel et écrit une ligne JSON par paquet ; `unread` signale des
  octets restants (format divergent), `error` un échec de décodage. Avec `-typed`, le NBT garde ses types
  (`{"int": 0}`, `{"byte_array": [...]}`…) et les champs optionnels montrent leur contenu
  (`{"set": true, "val": ...}`), que le JSON standard affiche `{}` : à utiliser pour comparer des optionnels ou
  importer des données (recettes).
- `drops` lance BDS lui-même (console sur son entrée standard), connecte le bot, puis fait calculer par BDS les
  drops de chaque bloc du plan avec chaque outil : `/give` et `/enchant` de l'outil, blocs posés en grille sur de
  la bedrock, puis `execute as @a at @s run loot spawn <case> mine <bloc> mainhand` (logique de drops de BDS,
  sans casser le bloc ni gérer de durée). Chaque tirage fait apparaître ses items dans sa propre case, ce qui
  permet de les attribuer par la position des `AddItemActor`. Plan : `{"samples": 4, "tools": [{}, {"tool":
  "iron_pickaxe", "enchants": {"fortune": 3}}], "blocks": ["minecraft:stone", "minecraft:wheat [\"growth\"=7]"],
  "checks": [...]}` ; `checks`, facultatif, donne pour chaque bloc l'état vérifié après la pose (les seules
  propriétés qui comptent : BDS recalcule orientation et connexions). Plans par état de la palette :
  `scripts/dev/plan_bds_drops.py`. Sortie : une ligne JSON par tirage (`placed` faux si le bloc n'était pas en
  place au moment du tirage). L'XP n'est pas relevée (`/loot` ne la produit pas). Bilan et comparaison :
  `scripts/dev/summarize_bds_drops.py` ; données de mc-rs : `scripts/dev/build_bds_drops.py`.
- **Rythme** : BDS n'exécute qu'une commande de console par tick (20 par seconde). `drops` écrit donc, avant de
  lancer BDS, un pack de comportement (`development_behavior_packs/mcrs_drops/`) dont chaque fonction traite un
  lot entier en un seul tick : items précédents tués, jusqu'à 225 blocs posés, présence vérifiée par `execute
  if block … run say mcrs-ok-<i>` (messages reçus par le bot), puis jusqu'à 4 096 tirages. Les fonctions ne
  dépendent pas de l'outil : une seule copie par liste de blocs (un pack de 46 Mo retardait la connexion du bot
  au-delà de son délai). Relevé de 2 049 états × 16 outils × 16 tirages (524 896 tirages) : 9 minutes. Le pack
  est appliqué au monde (`world_behavior_packs.json`) le temps du relevé puis retiré, y compris sur erreur, pour
  ne pas changer la pile de packs des autres captures ; si l'outil est tué, il reste appliqué jusqu'au relevé
  suivant, qui le reconnaît et le retire. BDS rejette une fonction entière dès qu'une ligne est invalide
  (« Function … not found ») : un lot rejeté repasse par la console, commande par commande, et les noms que
  BDS refuse (« Syntax error: Unexpected … », à `setblock` comme à `testforblock` : `end_gateway` passe l'un
  et pas l'autre) sont retenus dans `<relevé>.rejected.json`, puis écartés des fonctions aux relevés suivants.
  Sur la console, la sortie de BDS arrive en différé : l'outil attend une réponse par commande au lieu de lire
  après un délai fixe (sinon les derniers blocs d'un lot passent pour non posés).
- **Grille** : une case par bloc sur ±32 blocs autour du joueur. Mesures : BDS fait apparaître les items de
  `loot spawn x y z` en (x - 0,5, z - 0,5), sans exception sur 3 000 items ; au-delà d'environ 3 chunks, il
  n'envoie pas les items au bot ; au début, un chunk peut manquer tant que le bot ne l'a pas reçu. D'où un
  **étalonnage** au démarrage : terre cassée à la main sur toute la grille, répété jusqu'à ce que chaque case
  reçoive son item.
- La première connexion demande un code sur https://www.microsoft.com/link (compte Microsoft du jeu) : BDS
  exige une identité Xbox Live en NetherNet, même avec `online-mode=false`. Le jeton est enregistré dans
  `%APPDATA%\mc-rs\bds-capture-token.json`, hors du dépôt.
