# Scripts

Scripts Python annexes. Aucun n’est nécessaire pour compiler ou lancer le serveur.

## Import de données vanilla

À relancer à chaque changement de version du jeu (voir
[`../dev-docs/31-PROTOCOL-2169.md`](../dev-docs/31-PROTOCOL-2169.md), procédure).

- `import_recipes.py <tag>` : régénère `crates/mc-rs-server/data/recipes/vanilla.json` depuis
  `.reference/bedrock-samples` au tag donné (ex. `v1.26.40.05`) ; `--check` compare au fichier actuel.
- `import_item_upgrade_schema.py [dossier]` : consolide les schémas de `BedrockItemUpgradeSchema` dans
  `crates/mc-rs-server/data/item_id_upgrade_schema.json` (migration des anciens identifiants d’items).

## Outils de développement (`dev/`)

- `dev/test_motd.py` : interroge un serveur (Unconnected Ping) et affiche son MOTD ; `--compare` compare
  mc-rs (19132) et BDS (19133).
- `dev/udp_proxy.py` : relais UDP 19133 → 19132, pour contourner la restriction de boucle locale des
  applications UWP quand le client tourne sur la même machine.

Les benchmarks sont dans [`../benchmarks/`](../benchmarks/).
