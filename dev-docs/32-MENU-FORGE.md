# 32 – Menus Menu Forge sur Bedrock (menus coffre et formulaires)

Les menus dessinés avec le studio [Menu Forge](https://pedrokarim.github.io/menu-forge/)
s’affichent sur Bedrock grâce à un pack JSON UI exporté par le studio et à un
moteur d’exécution côté serveur. Le contrat entre les deux (fichiers, titre à
jetons, ordre des entrées, descripteur) est dans `docs/bedrock.md` du dépôt
Menu Forge ; ce document décrit l’intégration dans mc-rs.

## 1. Ce qui est en place

| Élément | Où |
|---|---|
| Moteur : descripteur, conditions, session (pile, état, pages), composition du formulaire | `crates/mc-rs-server/src/menu_forge/` |
| Formulaires Bedrock (`forms` du descripteur) : disposition de `mcrs_ui` choisie par le drapeau (`FORM_LAYOUTS`), boutons filtrés par `visibleWhen`, index d’un clic = rang parmi les boutons envoyés | `menu_forge/catalog.rs`, `menu_forge/session.rs` |
| Anciens formulaires de `/menu`, en fonctions pures (repli `/menu legacy`, référence des tests de parité) | `connection/forms.rs` (`legacy_form`, `legacy_click`) |
| Lien avec la connexion : ouverture, clic, effets (formulaire suivant, commandes, sons) | `connection/menu_forge.rs`, `PendingFormKind::MenuForge` dans `connection/forms.rs` |
| Chargement au démarrage (descripteur + pack exporté ajouté aux packs servis) | `menu_forge::load_at_startup`, `main.rs` |
| Section `[menu_forge]` | `config.rs` |
| `/mf open <menu>`, `/mf list`, `/mf reload` (opérateurs) | `commands/mf.rs` |
| `/menu` et `/menu <panneau>` passent par Menu Forge, ancien formulaire en repli | `commands/mod.rs`, `commands/menu.rs` |
| Accroche dans le pack : drapeau `§m§v`, variante `mcrs_menu_forge@menu_forge_router.main_panel`, terme du repli vanilla | `resource_packs/mcrs_ui/ui/server_form.json`, `_global_variables.json` (pack 1.4.0) |
| Espace de travail : les écrans de `/menu` en formulaires Bedrock, leurs versions en grille de coffre (`grid_*`), le menu `demo` ; et leur export | `menu_forge/workspace/`, `menu_forge/export/` (voir `menu_forge/README.md`) |

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

## 4. `/menu` : les écrans d’origine, portés comme formulaires Bedrock

Les écrans de `/menu` (hub, showcase et les huit panneaux de `mcrs_ui`) sont
des **formulaires Bedrock** du format Menu Forge (clé `form`, voir
`docs/format.md` et `docs/bedrock.md` § 9 du dépôt Menu Forge) : le formulaire
à boutons d’origine, envoyé tel quel dans la disposition `mcrs_ui` choisie par
le drapeau du titre (`§m§a` grille, `§m§b` boutons à gauche…).

| Commande | Menu Forge actif et écran exporté | Sinon |
|---|---|---|
| `/menu` | formulaire `hub` | ancien hub (`build_hub_form_batch`) |
| `/menu <panneau>` | formulaire du même nom | ancien panneau (`build_demo_panel_batch`) |
| `/menu legacy` | ancien hub, toujours | idem |
| `/mf open grid_hub` (…`grid_store`, `grid_wrapped`) | versions en grille de coffre de l’étape E2 | – |

Chaque formulaire reprend **exactement** la disposition, le titre, le contenu,
les textes, les images et l’ordre des boutons de `connection/forms.rs`, et le
comportement de chaque bouton : les commandes du hub (`tp` au point de
réapparition, `gamemode creative`, `gamemode survival`, `time set day`,
`time set night`, `biome`) suivies de la fermeture, le showcase vers chaque
panneau et le retour au hub, chaque bouton de panneau vers le showcase, la
bannière et « Rejoindre » du panneau `bottom_button` qui ferment. Seule
nouveauté : une icône vanilla sur quatre boutons du hub (Téléporter au spawn,
Mode Créatif, Régler sur Jour, UI Showcase), pour montrer la possibilité.

Les tests `menu_forge::export_tests` le vérifient sur l’export versionné :
même JSON que l’ancien formulaire (aux icônes du hub près) et même effet pour
chaque index de chaque écran, par comparaison avec `legacy_form` et
`legacy_click`.

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
- Formulaires Bedrock : ils s’appuient sur les dispositions du pack `mcrs_ui`
  (servi avec le pack Menu Forge) ; les icônes `path` vanilla du hub ne sont
  pas visibles dans le studio, seulement en jeu.

## 7. Essais en jeu

Rapatriés le 2026-10-25 des notes non commitées des worktrees `menu-forge-e0` (`E0-ESSAI.md`) et
`menu-forge-runtime` (`E2-ESSAI.md`), croisées avec leurs journaux serveur. La synthèse complète du rendu
est aussi dans `docs/bedrock.md` § 1 du dépôt Menu Forge.

### 7.1 Essai E0 (2026-09-11, client 1.26.45) : pourquoi des grilles

Disposition d’essai (fond de coffre 176 × 168) servie par une commande `/mfe0`, retirée depuis.

| Mécanisme | Résultat en jeu |
|---|---|
| Boutons de `form_buttons` posés au pixel avec un `collection_index` fixe (`§m§z`) | fond, placement au pixel, pastille par jeton de titre `(v)` et titre dynamique **marchent** ; mais les boutons affichent tous « A » et **renvoient tous l’index 0** : un `collection_index` posé à la main n’est pas honoré pour `form_buttons` |
| Fabrique vanilla, une entrée par bouton, position désignée par un jeton `(k)` du texte (`§m§y`) | **aucun bouton affiché** |
| Contrôle `grid` vanilla 9 × 3 sur `form_buttons` avec `#maximum_grid_items` (`§m§x`), 27 boutons, un par case | **index justes** au clic (journal : Diamant 0, Pomme 13, Pierre 20, Émeraude 26), soit colonne + 9 × ligne |

Conséquence, appliquée par l’exporteur et le moteur : un bouton cliquable doit être porté par une case
d’une `grid` qui génère elle-même les entrées de la collection, d’où les « 9 × lignes entrées (une par
case) » du § 3. **Point resté ouvert** : un bouton libre hors de toute grille (comme Valider ou Fermer dans
l’essai) n’a aucun mécanisme d’index qui marche.

L’essai a aussi trouvé la cause du « Rabbit » au téléchargement des packs : `ResourcePackDataInfo` doit
annoncer le type 6 (ressources), corrigé dans `connection/login.rs` (voir
[`31-PROTOCOL-2169.md`](31-PROTOCOL-2169.md) § 5).

### 7.2 Essai E2/E3 : procédure

1. Dossier d’exécution séparé (ignoré par git, par exemple `.tmp/e2-run/`) avec sa propre `server.toml` :
   port **19140** (peut tourner à côté du serveur habituel), WebUI coupée, monde plat, créatif,
   `[menu_forge] directory` pointant directement sur `menu_forge/export` (un nouvel export est pris au
   redémarrage, sans copie).
2. Console pilotable : `Get-Content -Path stdin.txt -Wait | & <chemin>\mc-rs-server.exe`, puis, depuis une
   autre fenêtre, `Add-Content stdin.txt "op <pseudo>"`. **Se reconnecter après `op`** : la liste des
   commandes n’est envoyée qu’à la connexion (`/mf` est réservée aux opérateurs, `/menu` non).
3. Au premier doute, supprimer « mc-rs UI » et « Menu Forge » du cache du client (Paramètres, Stockage) ;
   activer le journal de contenu (Paramètres, Créateur) pour voir les erreurs de JSON UI.
4. Comparer chaque écran de `/menu` avec `/menu legacy` : disposition, titre, textes, images, ordre et effet
   des boutons doivent être identiques, aux quatre icônes du hub près.

| Ce que l’on observe | Ce que ça veut dire |
|---|---|
| Liste vanilla de boutons vides à la place du dessin | pack Menu Forge ou accroche de `mcrs_ui` non appliqués : versions des deux packs, journal de contenu |
| Fond dessiné, aucun bouton ne réagit ou le mauvais | grille de la disposition : noter la case cliquée et la ligne `[menu-forge]` de la console |
| Textes figés absents ou décalés | noter le décalage et l’échelle d’interface (police Bedrock ≠ police Java) |
| `demo` : « Bonjour … » ou « Compteur » absents | étiquettes des textes dynamiques coupées hors de leur case |
| Hub sans icônes | chemin vanilla introuvable parmi `textures/items/compass_item`, `textures/blocks/grass_side_carried`, `textures/items/clock_item`, `textures/items/map_filled` |
| Store : onglets aussi en cartes de produits | attendu, l’ancien écran fait pareil |

### 7.3 Sessions E2/E3 jouées, verdict en attente

Les journaux montrent un vrai client (protocole 2169) le 2026-09-11 vers 22 h 22 et le 2026-09-12 entre
0 h 15 et 1 h 02 : ouverture de `hub`, `store`, `motd`, `demo`, `grid` et `grid_hub`. **Aucun verdict sur
le rendu n’a été noté.** Tant qu’il n’est pas donné, l’ancien chemin de `/menu` reste en place (§ 4) ; une
fois le rendu validé, retirer le repli et mettre à jour le § 6.
