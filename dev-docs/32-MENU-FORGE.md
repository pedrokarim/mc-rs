# 32 – Menus Menu Forge (rendu Bedrock « formulaire dessiné »)

Les menus dessinés avec le studio [Menu Forge](https://pedrokarim.github.io/menu-forge/)
s’affichent sur Bedrock grâce à un pack JSON UI exporté par le studio et à un
moteur d’exécution côté serveur. Le contrat entre les deux (fichiers, titre à
jetons, ordre des entrées, descripteur) est dans `docs/bedrock.md` du dépôt
Menu Forge ; ce document décrit l’intégration dans mc-rs.

## 1. Ce qui est en place

| Élément | Où |
|---|---|
| Moteur : descripteur, conditions, session (pile, état, pages), composition du formulaire | `crates/mc-rs-server/src/menu_forge/` |
| Lien avec la connexion : ouverture, clic, effets (formulaire suivant, commandes, sons) | `connection/menu_forge.rs`, `PendingFormKind::MenuForge` dans `connection/forms.rs` |
| Chargement au démarrage (descripteur + pack exporté ajouté aux packs servis) | `menu_forge::load_at_startup`, `main.rs` |
| Section `[menu_forge]` | `config.rs` |
| `/mf open <menu>`, `/mf list`, `/mf reload` (opérateurs) | `commands/mf.rs` |
| `/menu` et `/menu <panneau>` passent par Menu Forge, ancien formulaire en repli | `commands/mod.rs`, `commands/menu.rs` |
| Accroche dans le pack : drapeau `§m§v`, variante `mcrs_menu_forge@menu_forge_router.main_panel`, terme du repli vanilla | `resource_packs/mcrs_ui/ui/server_form.json`, `_global_variables.json` (pack 1.4.0) |
| Espace de travail des écrans de `/menu` et leur export | `menu_forge/workspace/`, `menu_forge/export/` (voir `menu_forge/README.md`) |

## 2. Configuration

```toml
[menu_forge]
enabled = true                   # charger le descripteur et servir le pack exportés
directory = "menu_forge/export"  # dossier d’export du studio (runtime.json et pack/)
menu_command = true              # /menu ouvre les menus Menu Forge quand ils existent
```

Section absente : ces valeurs par défaut. Un descripteur absent ou invalide est
journalisé (`[menu-forge] …`) et laisse un catalogue vide : le serveur démarre,
`/menu` garde ses anciens formulaires.

## 3. Déroulé

1. **Démarrage** : `runtime.json` est validé (format, version de contrat,
   drapeau identique à `§m§v`, cases dans la grille, entrées des textes) ; le
   pack `pack/` est chargé comme les autres packs et envoyé à la connexion.
   Un pack et un descripteur d’exports différents (uuid ou version) sont
   signalés.
2. **Ouverture** (`/menu`, `/mf open`) : une `Session` part de l’état initial
   du menu et compose le formulaire : titre `§m§v[mf:<id>]<jetons>§r<nom>`,
   `9 × lignes` entrées (une par case, texte vide pour une case vide), puis une
   entrée par texte dynamique.
3. **Clic** (`ModalFormResponse`) : l’index désigne la case ; le slot mémorisé
   à la composition exécute ses actions (`open`, `back`, `close`, `setState`,
   `nextPage`, `prevPage`, `sound`, `command`, `custom`). Sauf fermeture, le
   menu du haut de la pile est recomposé et renvoyé (le client ferme toujours
   le formulaire au clic).
4. **Effets** : commande `player` → `pending_commands` ; commande `console` →
   `pending_console_commands` (exécutée par `main.rs` avec la source
   `Console`) ; son → paquet `PlaySound` à la position du joueur ; `custom` →
   journalisé (aucun gestionnaire sur mc-rs pour l’instant).
5. **Fermeture** par le joueur (`UserClosed`) ou refus (`UserBusy`) : la
   session est oubliée.

Variables fournies par mc-rs : `{viewer.name}`, `{viewer.uuid}`,
`{mcrs.spawn}` (point de réapparition, `x y z`, comme l’ancien bouton « Téléporter
au spawn ») ; drapeaux : `viewer.bedrock` (toujours), `viewer.op`.

## 4. `/menu` : Menu Forge d’abord, ancien chemin en repli

| Commande | Menu Forge actif et menu exporté | Sinon |
|---|---|---|
| `/menu` | menu `hub` | ancien hub (`build_hub_form_batch`) |
| `/menu <panneau>` | menu du même nom | ancien panneau (`build_demo_panel_batch`) |
| `/menu legacy` | ancien hub, toujours | idem |

Les menus reprennent chaque bouton de l’ancien chemin (inventaire complet dans
`docs/bedrock.md` § 8 du dépôt Menu Forge) : les commandes du hub (`tp`
au point de réapparition, `gamemode creative`, `gamemode survival`,
`time set day`, `time set night`, `biome`) suivies de la fermeture, le
showcase vers chaque panneau et le retour au hub, chaque bouton de panneau vers
le showcase, « Rejoindre » du panneau `bottom_button` qui ferme. Les tests
`menu_forge::export_tests` vérifient cette parité sur l’export versionné.

**L’ancien chemin reste en place tant que le rendu n’a pas été validé en jeu**
(`[menu_forge] menu_command = false` pour y revenir sans recompiler).

## 5. Modifier les menus

Voir `menu_forge/README.md` : ouvrir `menu_forge/workspace/` dans le studio,
dossier d’export Bedrock = `menu_forge/export/`, bouton **Bedrock**. La version
du pack s’incrémente seule (le client garde un pack en cache par uuid +
version). Nouveau visuel : redémarrer le serveur et se reconnecter ; logique
seule (actions, conditions) : `/mf reload`.

## 6. Limites

- Listes (`kind: list`) : l’interface `Lists` existe mais mc-rs n’enregistre
  encore aucune source ; les cases restent vides.
- Actions `custom` : journalisées, sans gestionnaire.
- Un seul formulaire en attente par joueur (`pending_form`) : ouvrir un autre
  formulaire abandonne le menu en cours.
- Chaque changement d’état referme puis rouvre le formulaire.
- Rendu à vérifier en jeu : étiquettes des textes dynamiques qui débordent de
  leur case, bouton de fermeture vanilla du routeur, icônes d’items.
