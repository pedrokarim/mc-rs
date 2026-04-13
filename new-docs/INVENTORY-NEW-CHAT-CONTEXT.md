# Contexte de démarrage pour la session "Port complet du système inventaire PMMP"

> **Texte à coller au début d'une nouvelle session Claude Code pour démarrer le port complet.**

---

## Mission

Tu travailles sur **mc-rs** (`c:\Users\karim\Desktop\programming-laboratory\mc-rs\`), une réécriture de PocketMine-MP en Rust. Le serveur cible le protocol Bedrock 944 (Minecraft 1.26.10).

**Ta mission unique pour cette session : porter intégralement le système d'inventaire de PocketMine-MP en Rust.** Le système actuel ne fonctionne pas (3 bugs critiques bloquants depuis des semaines). Bricoler les paquets individuellement n'a JAMAIS fonctionné — il faut porter le système entier.

## Règle absolue : ne PAS s'arrêter

Tu **NE T'ARRÊTES PAS** tant que les 10 tests E2E listés dans `new-docs/INVENTORY-ITEMS-SYSTEM.md` §4 Phase F ne passent **PAS TOUS** sur un vrai client Bedrock.

- Pas de "je propose un plan" puis stop.
- Pas de "j'ai fait la phase A, à toi de tester".
- Pas de "j'ai du mal à isoler, dis-moi".
- Tu fais **toute** la phase A, **toute** la phase B, **toute** la phase C, etc.
- Quand tu butes : tu ouvres le code PMMP, tu lis intégralement, tu portes ligne par ligne. Pas de devinette.
- Quand un test échoue : tu lis les logs serveur (`.reference/server.log`), tu trouves la cause, tu fixes, tu retestes.
- L'utilisateur testera périodiquement avec un client Bedrock — entre ses tests, tu continues à avancer le code.

Si tu identifies un bug qui demande une décision produit (genre : faut-il supporter X ?), tu prends la décision la plus simple compatible PMMP et tu continues. Tu n'arrêtes pas pour demander.

## Documents à lire AVANT de coder

Lis ces fichiers **intégralement** dans cet ordre :

1. **`CLAUDE.md`** (à la racine) — rappel des règles du projet
2. **`new-docs/INVENTORY-ITEMS-SYSTEM.md`** — état exact du code actuel + plan détaillé en 6 phases (A→F) avec chemins de fichiers et lignes
3. **`new-docs/09-INVENTORY-SYSTEM.md`** — architecture cible
4. **`new-docs/99-ROADMAP.md`** section "🚨 PRIORITÉ ABSOLUE — Phase INV" — vue d'ensemble checklist

Puis lis intégralement les fichiers PMMP listés dans `INVENTORY-ITEMS-SYSTEM.md` §6, en particulier :
- `.reference/PocketMine-MP/src/network/mcpe/InventoryManager.php` (le cœur du système)
- `.reference/PocketMine-MP/src/network/mcpe/handler/ItemStackRequestExecutor.php`
- `.reference/PocketMine-MP/src/network/mcpe/handler/ItemStackResponseBuilder.php`
- `.reference/PocketMine-MP/src/inventory/PlayerInventory.php` + `BaseInventory.php` + `SimpleInventory.php`
- `.reference/PocketMine-MP/src/entity/object/ItemEntity.php`
- `.reference/PocketMine-MP/vendor/pocketmine/bedrock-protocol/src/ContainerOpenPacket.php` + autres packets inventaire

## Règle protocole : PocketMine est la SEULE référence

L'utilisateur l'a répété 4 milliards de fois. **PocketMine-MP est LA référence.** Pas dragonfly, pas gophertunnel, pas bedrock-rs, pas prismarine. Si tu trouves une différence entre PMMP et le client (PMMP est en protocol 924, on cible 944), tu vérifies dans le code mc-rs si une note existe (par exemple `SetActorMotionPacket` ajoute un champ `tick` en 944 — déjà fixé). Sinon tu suis PMMP à la lettre.

Les autres références dans `.reference/` (dragonfly, gophertunnel, etc.) servent UNIQUEMENT à vérifier qu'un format wire 944 ne diverge pas trop de PMMP 924. Tu ne portes RIEN depuis ces autres références. Tu portes uniquement depuis PMMP.

## État du code au démarrage

- Le serveur compile (`cargo build --release` OK)
- Tests passent (`cargo test`)
- Connexion + spawn + déplacement + chat + commands : ✅ fonctionnels
- Block break : ✅ envoie UpdateBlock + spawn item entity (mais l'item s'affiche en ombre côté client)
- Inventaire : ❌ bricolé, ne fonctionne pas

Voir `INVENTORY-ITEMS-SYSTEM.md` §7 pour le détail exact.

## Workflow attendu

1. **Lis tous les docs ci-dessus.** Ne saute pas cette étape. La quasi-totalité de la connaissance nécessaire est déjà documentée.
2. **Fais une checklist TodoWrite** des sous-tâches des Phases A→F.
3. **Exécute la Phase A complètement** (infrastructure : `inventory_manager.rs`, extension `PlayerInventory`, refactor `ItemStackWrapper`).
4. **Build + test après chaque changement majeur** (`cargo fmt && cargo build --release && cargo test`).
5. **Exécute la Phase B complètement** (port `InventoryManager`).
6. **Exécute la Phase C complètement** (branchement dans `Connection`).
7. **Exécute la Phase D complètement** (items au sol).
8. **Exécute la Phase E complètement** (drop depuis inventaire).
9. **Demande à l'utilisateur de tester** chaque test E2E un par un (`/launch` ou skill équivalent pour relancer le serveur). Entre ses tests, continue à avancer le code.
10. **Si un test échoue : lis les logs, identifie la cause, fixe, retest. Pas de bricolage.**

## Skills disponibles

L'utilisateur a configuré ces skills :
- `/build` — build release
- `/launch` — kill + lance le serveur
- `/restart` — kill + build + lance
- `/logs` — affiche les derniers logs serveur
- `/stop` — arrête le serveur
- `/test` — lance les tests
- `/commit <message>` — git add + commit

Utilise-les. Quand tu veux que l'utilisateur teste, utilise `/restart` pour relancer le serveur avec ton dernier code, puis dis-lui : « Connecte-toi et teste X ».

## Communication

L'utilisateur parle français. Il en a marre de tourner en rond. Il a deux exigences :
1. **Sois direct.** Pas de gants, pas de "je propose", pas de "ça pourrait marcher". Tu fais ou tu ne fais pas.
2. **Ne t'arrête pas.** Si tu finis un sous-ensemble, tu enchaînes le suivant. Pas de question rhétorique. Pas de pause inutile.

## Critère de fin de mission

Les 10 tests E2E listés dans `INVENTORY-ITEMS-SYSTEM.md` §4 Phase F sont **TOUS validés** par l'utilisateur sur un vrai client Bedrock 1.26.10. Tu commits le travail (`/commit "feat: inventaire complet PMMP"`). Mission accomplie.

## Démarre maintenant

Ne réponds pas avec un plan théorique. Commence par lire les fichiers, puis attaque la Phase A immédiatement. Le premier message qui parvient à l'utilisateur doit déjà contenir des modifications de code en cours.
