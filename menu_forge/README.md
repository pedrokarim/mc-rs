# Menus Menu Forge de mc-rs

Les écrans de `/menu` (hub et panneaux de démonstration du pack `mcrs_ui`)
dessinés comme menus [Menu Forge](https://pedrokarim.github.io/menu-forge/),
et leur export pour Bedrock, lu par le moteur `crates/mc-rs-server/src/menu_forge/`.
Documentation : [`dev-docs/32-MENU-FORGE.md`](../dev-docs/32-MENU-FORGE.md).

| Dossier | Contenu |
|---|---|
| `workspace/` | espace de travail du studio : `menus/*.menu.json`, `textures/mcrs/*.png`, `generate.py` |
| `export/` | écrit par le studio (« Exporter pour Bedrock ») : `pack/` (pack de ressources servi aux clients), `runtime.json` (logique des menus, lue par le serveur) |

Les textures sont générées par `workspace/generate.py` (Pillow), dans l’esprit
visuel de `mcrs_ui` ; aucun asset tiers. Pour modifier un menu :

1. ouvrir `workspace/` comme espace de travail dans le studio Menu Forge ;
2. dans les paramètres du studio, section « Export pour Bedrock », choisir le
   dossier `export/` ;
3. modifier, puis bouton **Bedrock** de l’éditeur : la version du pack
   s’incrémente seule ;
4. redémarrer le serveur (le pack est chargé au démarrage) et se reconnecter ;
   `/mf reload` suffit si seule la logique a changé (actions, conditions).

`python workspace/generate.py` régénère les textures ; avec `--menus`, il
réécrit aussi les menus d’origine (les retouches faites dans le studio sont
alors perdues).
