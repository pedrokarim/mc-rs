# Documentation interne de MC-RS

Notes de conception, spécifications, audits et suivis de chantier. Le site public (GitHub Pages) est
dans [`../docs/`](../docs/) ; ce dossier-ci est la documentation de travail des développeurs.

Les fichiers sont numérotés par domaine. Un numéro partagé (`18-…`) regroupe un document d’architecture et
son suivi.

## Vue d’ensemble

- [`00-PROJECT-OVERVIEW.md`](00-PROJECT-OVERVIEW.md) : présentation du projet
- [`98-RUST-ARCHITECTURE.md`](98-RUST-ARCHITECTURE.md) : architecture Rust actuelle
- [`99-ROADMAP.md`](99-ROADMAP.md) : feuille de route, phase par phase

## Réseau et protocole

- [`02-NETWORK-RAKNET.md`](02-NETWORK-RAKNET.md) : RakNet
- [`03-PROTOCOL-PACKETS.md`](03-PROTOCOL-PACKETS.md) : paquets
- [`04-LOGIN-FLOW.md`](04-LOGIN-FLOW.md) : séquence de connexion
- [`24-CHUNK-LOADING.md`](24-CHUNK-LOADING.md) : chargement et envoi des chunks
- [`31-PROTOCOL-2169.md`](31-PROTOCOL-2169.md) : protocole 2169 (1.26.45), références après la fin de
  support de PMMP, **causes de déconnexion trouvées avec le vrai client** et outils de diagnostic
- [`33-PROTOCOL-2193.md`](33-PROTOCOL-2193.md) : protocole 2193 (1.26.51), blocs vanilla data-driven,
  propriétés de connexion et migration des mondes
- [`34-BDS-RETRO-INGENIERIE.md`](34-BDS-RETRO-INGENIERIE.md) : journal d'observation de BDS (captures de
  paquets, décompilation, comparaison avec mc-rs) et écarts trouvés
- [`35-CODES-ERREUR-CLIENT.md`](35-CODES-ERREUR-CLIENT.md) : mot-code de chaque écran d'erreur du client
  (« Block », « Boat »…) et cause correspondante, d'après BDS

## Cœur du serveur

- [`01-SERVER-CORE.md`](01-SERVER-CORE.md) : cœur du serveur
- [`10-EVENT-SYSTEM.md`](10-EVENT-SYSTEM.md) : événements
- [`11-PLUGIN-SYSTEM.md`](11-PLUGIN-SYSTEM.md) : plugins
- [`12-COMMAND-SYSTEM.md`](12-COMMAND-SYSTEM.md) : commandes
- [`13-SCHEDULER-ASYNC.md`](13-SCHEDULER-ASYNC.md) : planificateur et tâches asynchrones

## Monde et génération

- [`05-WORLD-SYSTEM.md`](05-WORLD-SYSTEM.md) : monde
- [`06-BLOCK-SYSTEM.md`](06-BLOCK-SYSTEM.md) : blocs
- [`14-WORLD-GENERATION.md`](14-WORLD-GENERATION.md) : génération
- [`21-ANALYSER-MC-WORLD.md`](21-ANALYSER-MC-WORLD.md) : analyser un monde Bedrock
- [`22-GENERATION-PARAMS-FROM-WORLD.md`](22-GENERATION-PARAMS-FROM-WORLD.md) : paramètres extraits d’un
  monde réel
- [`23-BIOME-FEATURES-EXHAUSTIVE.md`](23-BIOME-FEATURES-EXHAUSTIVE.md) : contenu de chaque biome
- [`25-BLOCK-PALETTE-EXTRACTION.md`](25-BLOCK-PALETTE-EXTRACTION.md) : extraction de la palette de blocs
- [`26-DIMENSIONS-CADRAGE.md`](26-DIMENSIONS-CADRAGE.md) : cadrage Nether et End
- [`27-DIMENSIONS-GENERATION-SPEC.md`](27-DIMENSIONS-GENERATION-SPEC.md) : génération Nether et End
- [`28-DIMENSIONS-SUIVIS.md`](28-DIMENSIONS-SUIVIS.md) : suivis des dimensions

## Items, inventaire et artisanat

- [`08-ITEM-SYSTEM.md`](08-ITEM-SYSTEM.md) : items
- [`09-INVENTORY-SYSTEM.md`](09-INVENTORY-SYSTEM.md) : inventaire
- [`15-CRAFTING-SYSTEM.md`](15-CRAFTING-SYSTEM.md) : artisanat
- [`20-INVENTORY-ITEMS.md`](20-INVENTORY-ITEMS.md) : état réel et plan de portage inventaire, items et
  items au sol
- [`30-VANILLA-DATA-IMPORT.md`](30-VANILLA-DATA-IMPORT.md) : import des données vanilla

## Entités et mobs

- [`07-ENTITY-SYSTEM.md`](07-ENTITY-SYSTEM.md) : entités
- [`18-MOB-AI.md`](18-MOB-AI.md) : framework d’IA des mobs
- [`18-MOB-AI-PROGRESS.md`](18-MOB-AI-PROGRESS.md) : suivi d’avancement de l’IA
- [`19-MOB-SYSTEM-ETAT-DES-LIEUX.md`](19-MOB-SYSTEM-ETAT-DES-LIEUX.md) : état des lieux du système de mobs

## Packs de ressources et interface

- [`16-RESOURCE-PACK-PIPELINE.md`](16-RESOURCE-PACK-PIPELINE.md) : chaîne des packs et UI pilotée par le
  serveur
- [`17-UI-LAYOUTS.md`](17-UI-LAYOUTS.md) : catalogue des mises en page
- [`32-MENU-FORGE.md`](32-MENU-FORGE.md) : menus du studio Menu Forge (moteur, `/mf`, `/menu`, export)

## Archives

Documents périmés, gardés pour l’historique. Ne pas s’en servir comme référence d’implémentation.

- [`archive/ARCHIVE-legacy-code.md`](archive/ARCHIVE-legacy-code.md) : ce que contenaient `old_crates/` et
  `docs_initial/`, supprimés le 2026-05-28
- [`archive/HANDOFF-FREEZE-BUG.md`](archive/HANDOFF-FREEZE-BUG.md) : enquête sur le gel par connexion
  (résolu le 2026-05-19)
- [`archive/NEW-SYSTEMS.md`](archive/NEW-SYSTEMS.md) : bilan de la session de portage du 2026-04-14
- [`archive/INVENTORY-NEW-CHAT-CONTEXT.md`](archive/INVENTORY-NEW-CHAT-CONTEXT.md) : consigne de démarrage
  d’une ancienne session sur l’inventaire
