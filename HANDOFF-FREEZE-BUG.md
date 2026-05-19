# HANDOFF — Bug de gel serveur (per-connexion) — mc-rs

> ## ✅ CAUSE RACINE RÉELLE TROUVÉE ET CORRIGÉE (2026-05-19)
>
> Méthode : instrumentation per-session → reproduction → LECTURE du log →
> correction. (Une 1ère théorie "fenêtre datagramme" a été émise sans repro,
> elle était FAUSSE ; ne plus jamais théoriser un fix sans repro.)
>
> **Preuve par instrumentation** : au gel, log `FREEZE DETECTED` montre
> `in=2256 (drop_win=0) out=2092 | dgram_win=2256..4304 | reliable_win=4..2052
> held=2042 | ordered=[(0,2045,0)]`. ⇒ fenêtre datagramme OK (drop_win=0,
> avance), mais **fenêtre RELIABLE figée à `4..2052` depuis le début**.
>
> **CAUSE** : `recv.rs::handle_encapsulated` faisait `handle_split` AVANT le
> traitement de la fenêtre reliable. Le paquet **Login** (énorme : chaîne JWT +
> skin → splitté en N parts reliable, message_index 4,5,6…) : les parts
> incomplètes faisaient `return` avant l'enregistrement du message_index ; le
> paquet réassemblé ne portait qu'UN message_index. Les autres (dès 4) jamais
> enregistrés dans `reliable_received` → `reliable_window_start` bloqué à **4**
> à vie. Au bout de 2048 message_index, tout paquet reliable rejeté (`msg_idx >=
> reliable_window_end`) → canal ordered head-of-line bloqué → plus aucun paquet
> de jeu livré (cassage de blocs, particules, commandes morts), joueur bouge
> (prédiction client), serveur global vivant. Reconnexion = fenêtre reliable
> neuve = réparé. Explique 100 % des faits du §2.
>
> **Fix** (fidèle RakLib `ReceiveReliabilityLayer::handleEncapsulatedPacket` :
> bloc messageIndex PUIS handleSplit, vérifié contre
> `.reference/PocketMine-MP/vendor/pocketmine/raklib/src/generic/ReceiveReliabilityLayer.php`) :
> - `recv.rs::handle_encapsulated` : bloc fenêtre-reliable (dedup + slide sur
>   `message_index`, borne sup inclusive `>` comme RakLib) déplacé AVANT
>   `handle_split`, exécuté pour CHAQUE paquet encapsulé incl. chaque part de
>   split. Test régression `test_split_parts_register_all_reliable_indices`.
>   26/26 tests raknet OK.
>
> **Fix #1 CONSERVÉ (vrai bug latent mais PAS le coupable)** : `ReceiveLayer::
> update()` (force-advance fenêtre datagramme past `highest_seq_number`) +
> dédup `nack_queue`, appelé `session.rs::tick()`. Confirmé non-cause par
> `drop_win=0`. À garder (parité RakLib correcte).
>
> **Instrumentation CONSERVÉE** : `session.rs` `FREEZE DETECTED <addr>` (WARN,
> visible en info) + snapshot INFO/30s ; `recv_layer.diag()` → `RecvDiag`.
>
> **À faire valider par l'utilisateur** : jouer/miner longtemps, les particules
> de cassage ne doivent JAMAIS disparaître. Reste du doc = historique conservé.

---

> Document de passation pour le prochain chat. Tout ce qui a été établi,
> éliminé, instrumenté, et les pistes restantes. Lis-le EN ENTIER avant d'agir.

---

## 1. LE SYMPTÔME (reproductible)

- Le joueur se connecte, joue normalement (particules de cassage OK, commandes OK).
- Au bout d'un moment de **minage continu en descendant** (creuser le sol, aller
  vers les Y bas, traverser beaucoup de chunks), **ça "pète"** :
  - plus de particules quand on casse un bloc
  - les commandes (`/gamemode`, etc.) ne répondent plus
  - le joueur peut **toujours bouger** (prédiction client) et reste **connecté**
  - **le serveur n'est PAS mort** : il continue de ticker (autosave, watchdog)
- **Déconnexion + reconnexion = ça remarche** (temporairement, jusqu'au prochain
  déclenchement). ⇒ **C'est un état de session PAR-CONNEXION qui se corrompt**,
  PAS un gel global du serveur.
- Indicateur fiable du joueur pour tester : "les particules de cassage de bloc
  disparaissent" = le bug s'est déclenché.

Indice secondaire observé une fois : le cycle jour/nuit a fast-forward
(jour→nuit en ~2 min). Probablement lié à un épisode de rafale de ticks
(voir §3, MissedTickBehavior — corrigé).

---

## 2. CE QUI EST PROUVÉ (faits durs, vérifiés sur logs/code)

1. **Le main-loop n'est PAS gelé.** Le watchdog (voir §4) montre le heartbeat
   qui avance en continu (~700-1000/s), `last_phase=[0] loop-top`. L'autosave
   continue (`Saved N dirty chunks`) APRÈS le gel perçu. Le process est vivant,
   CPU bas (~2-17 %, PAS un hot-loop à 100 %).
2. **Aucun panic** (pas de `logs/CRASH-*.log`). Profil `panic=unwind` → un panic
   écrirait un CRASH. Donc pas d'empoisonnement de Mutex.
3. **Aucune erreur `recv_from`** loggée (WARN rate-limité en place).
4. **Aucun `split DROP`** loggé (WARN en place) → la limite de splits n'est
   jamais atteinte.
5. La dernière activité jeu avant gel est **toujours une séquence de cassage de
   blocs** (souvent `minecraft:stone`, Y bas, en descendant).
6. Après le gel : le serveur n'émet plus de `recv pkt 0x..` pour cette
   connexion (plus aucun paquet de jeu traité POUR CETTE SESSION), mais le tick
   global continue.

⇒ **Conclusion : une session RakNet précise cesse de livrer/traiter les paquets
entrants du client, alors que le serveur global et les autres sessions vont
bien. Reconnexion = nouvelle session = réparé.**

---

## 3. HYPOTHÈSES ÉLIMINÉES (NE PAS REFAIRE — perte de temps garantie)

| # | Hypothèse | Verdict | Preuve |
|---|---|---|---|
| H1 | Panic + Mutex empoisonné | ❌ ÉLIMINÉ | aucun CRASH log, panic=unwind |
| H2 | Deadlock mutuel sur `chunk_cache` | ❌ ÉLIMINÉ | locké que depuis la tâche main-loop ; webui/stdin ne le touchent pas |
| H3 | Deadlock dans `save_dirty`/`save_chunk_now` | ❌ ÉLIMINÉ | log "Saved N dirty chunks" émis APRÈS `storage.flush()` ; corrigé quand même (voir §5) |
| H4 | Spin Windows UDP `WSAECONNRESET` (recv_from erreur en boucle) | ❌ ÉLIMINÉ comme cause active | SIO_UDP_CONNRESET désactivé + 0 `recv_from error` loggé. Fix conservé (bon réflexe défensif). |
| H5 | `tokio::interval` MissedTickBehavior::Burst affame l'arm recv | ❌ ÉLIMINÉ comme cause | corrigé en `Skip` mais le gel persiste après |
| H6 | `MAX_CONCURRENT_SPLITS=4` / `MAX_SPLIT_PART_COUNT=128` trop bas → split jeté → wedge ordonné | ❌ ÉLIMINÉ comme cause active | relevé à 512/8192 + WARN ajouté ; **0 `split DROP` loggé** → jamais atteint. (Fix conservé, c'était un vrai bug latent mais pas CE bug.) |
| H7 | Bug "held_slot out of bounds" (suggéré par un agent) | ❌ FAUX | held_slot borné 0-8 partout (vérifié) |

**Leçon** : plusieurs agents ont donné des diagnostics confiants mais FAUX
(held_slot, poisoning). TOUJOURS vérifier le claim dans le code réel avant de
coder. L'utilisateur insiste là-dessus.

---

## 4. INSTRUMENTATION EN PLACE (utilise-la, ne la refais pas)

Tout est déjà câblé et fonctionnel :

- **Panic hook synchrone** (`crates/mc-rs-server/src/logging.rs`
  `install_panic_hook`) : écrit `logs/CRASH-<ts>.log` (location + backtrace,
  flush+sync, survit au crash). `RUST_BACKTRACE=full` requis (déjà dans
  `.claude/commands/launch.md` et `CLAUDE.md`).
- **Watchdog** (`crates/mc-rs-server/src/watchdog.rs`) : thread OS dédié.
  `checkpoint(id)` incrémente un heartbeat ET stocke la phase ; `beat()` idem.
  Si heartbeat gelé > 6 s → `logs/FREEZE-<ts>.log` avec la phase bloquée.
  Log "alive: heartbeat=N last_phase=[i] nom" toutes les 20 s.
  Checkpoints semés dans `main.rs` (0=loop-top,1=recv,2=accept,3=peer_events@recv,
  4=tick-start,5=tick_sessions,6=world_tick,7=game_tick,9=mob_tick,10=item_tick,
  11=autosave,12=peer_events@tick,13=shutdown).
  **LIMITE CONNUE** : le watchdog bat sur la PROGRESSION de la boucle, pas sur
  "le jeu avance pour la session X". Comme la boucle continue d'itérer pendant
  le gel (le bug est per-session, pas global), le watchdog ne déclenche PAS.
  **TODO prochain chat** : ajouter un compteur "paquets de jeu traités pour la
  session" ou "dernier order_index livré par session" et déclencher le watchdog
  si CE compteur gèle alors que la session est connectée.
- **WARN `recv_from error`** (`crates/mc-rs-raknet/src/server.rs`
  `recv_and_process`) : 1er + tous les 500, kind+raw_os.
- **WARN `split DROP`** (`crates/mc-rs-raknet/src/reliability/recv.rs`
  `handle_split`) : si limite split atteinte.

---

## 5. FIXES DÉJÀ APPLIQUÉS (corrects, à GARDER)

Tous buildés, testés (1010+ tests workspace OK, 24 raknet OK), commit-ables :

1. **Protocole 944→975** (Bedrock 1.26.10→1.26.20) : versions partout +
   3 fixes wire vérifiés contre PMMP 5.43.1 / BedrockProtocol 57.1.0 :
   - `ItemStackWrapper::encode_descriptor` (NetworkItemStackDescriptor, format
     975, utilisé SEULEMENT par InventorySlot + MobEquipment)
   - `InventorySlot::encode` : containerName/storage → Optional + descriptor
   - `LevelSoundEvent::encode` : +1 octet Optional(firePosition) trailing
   - Réfs : `.reference/BedrockProtocol` checkout tag
     `57.1.0+bedrock-1.26.20`, PMMP tag `5.43.1` (detached HEAD).
     Modifs locales stashées (`git stash list`).
2. **Palette de blocs 1.26.20** : `crates/mc-rs-server/data/canonical_block_states.nbt`
   remplacé par `.reference/BedrockData` (commit "Updated from 1.26.20"),
   `block_registry_data.rs` régénéré (16899 états / 1355 noms). **Le monde
   `worlds/world` doit être supprimé** après tout changement de palette (les
   chunks LevelDB stockent les runtime IDs = index de palette).
3. **Arbres coupés à plat** : `terrain_generator.rs` ~ligne 653 — `sub_chunk_count`
   étendu pour couvrir le Y max de `veg_map`+`struct_map` (avant : terrain seul
   → cime des arbres tronquée à la frontière de sub-chunk).
4. **Guards `chunk_cache` resserrés** : bloc mob-tick `main.rs` ~1951 ne tient
   plus le guard pendant `raknet.send_to_session` ; `save_chunk_now` par-édition
   retiré de `movement.rs` (×3) → autosave raccourci 30000→1500 ticks.
5. **SIO_UDP_CONNRESET=false** (`server.rs::disable_udp_conn_reset`, FFI Winsock,
   Windows-only, 0 dép).
6. **MissedTickBehavior::Skip** sur `tick_timer` (`main.rs`).
7. **MAX_CONCURRENT_SPLITS 4→512, MAX_SPLIT_PART_COUNT 128→8192** (`consts.rs`).

---

## 6. HYPOTHÈSE LA PLUS FORTE RESTANTE (par où commencer)

Le bug est dans la **couche de fiabilité RakNet en RÉCEPTION**
(client→serveur), `crates/mc-rs-raknet/src/reliability/recv.rs` :

- Le canal **ordered** (`recv_ordered_index[ch]` / `recv_ordered_queue[ch]`,
  lignes ~151-181) fait du **head-of-line blocking** : si un `order_index`
  reliable-ordered venant du client n'est jamais livré, `recv_ordered_index`
  reste bloqué à jamais → tous les paquets de jeu suivants de cette session
  s'empilent sans être traités. Reconnexion = `recv_ordered_index` remis à 0.
- Ce n'est PAS causé par un split jeté (H6 éliminé). Causes candidates :
  a. **Génération ACK/NACK cassée** : le serveur ne demande pas correctement la
     retransmission d'un datagramme manquant → le client ne renvoie jamais le
     paquet manquant → trou permanent dans la séquence ordonnée. Auditer
     `recv.rs on_datagram` (génération de l'ACK/NACK) vs RakLib
     `.reference/PocketMine-MP/vendor/pocketmine/raklib/src/generic/ReceiveReliabilityLayer.php`
     (méthodes `onPacket`, `handleEncapsulatedPacket`, fenêtre NACK).
  b. **Bug d'index ordonné** : off-by-one, wraparound u32, ou un paquet livré
     qui n'incrémente pas `recv_ordered_index` correctement (drain de la queue
     lignes 162-170), ou un split réassemblé qui ressort avec un mauvais
     `order_index`.
  c. **Fenêtre de réception** (`RECV_WINDOW_SIZE=2048`, `consts.rs`) : si la
     dédup par sequence number rejette à tort, ou si la fenêtre déborde sous
     charge (minage = beaucoup de paquets) → datagrammes droppés sans NACK.
  d. Côté SEND serveur : `reliable_cache` / resend (`reliability/send.rs`) —
     vérifié superficiellement (on_ack prune, check_timeouts appelé) mais
     audit complet non fait. Un resend serveur cassé n'explique PAS le symptôme
     (le canal bloqué est la RÉCEPTION client→serveur) mais à garder en tête.

**Comparaison RakLib = LE bon réflexe ICI** (contrairement aux comparaisons
gameplay précédentes jugées peu rentables) : c'est un bug de transport pur,
RakLib (`.reference/PocketMine-MP/vendor/pocketmine/raklib/src/generic/`) est
LA référence canonique correcte. Fichiers clés à diff :
- `ReceiveReliabilityLayer.php` (ordering, NACK, split, fenêtre) vs notre `recv.rs`
- `SendReliabilityLayer.php` vs notre `send.rs`
- `Session.php` (orchestration tick/ACK) vs notre `session.rs`

---

## 7. PLAN RECOMMANDÉ POUR LE PROCHAIN CHAT

1. **D'abord rendre le bug VISIBLE** (l'instrumentation actuelle est aveugle
   au cas per-session). Ajouter dans `recv.rs` un log/compteur par session :
   "ordered channel ch=X bloqué à index N depuis T secondes, M paquets en
   attente dans recv_ordered_queue". Au prochain repro on saura immédiatement
   si c'est bien un HOL block ordonné et sur quel index.
2. Reproduire (miner en descendant en continu — déclencheur fiable).
3. Lire le log → confirmer/infirmer le HOL block ordonné.
4. Audit ciblé `recv.rs` vs RakLib `ReceiveReliabilityLayer.php` sur le point
   exact (ACK/NACK generation OU index ordonné OU fenêtre).
5. Corriger, rebuild, **supprimer `worlds/world`** si la palette/terrain a
   bougé (sinon inutile), relancer, faire valider par l'utilisateur (son test :
   miner longtemps en descendant, vérifier que les particules ne disparaissent
   jamais).

---

## 8. COMMANDES / ENVIRONNEMENT

- Build : `cargo build --release -p mc-rs-server` (clean ; 1 doctest
  PRÉ-EXISTANT échoue : `world.rs:696 CreativeContent::encode` — prose de doc,
  RIEN à voir, ignorer).
- Tests : `cargo test` (1010+ OK).
- Lancer : `RUST_BACKTRACE=full RUST_LOG=info ./target/release/mc-rs-server.exe`
  (le serveur écrit `logs/server.<date>.log` ; PAS de redirection shell).
- Tuer : `powershell -Command "Get-Process mc-rs-server -ErrorAction SilentlyContinue | Stop-Process -Force"`
  (le serveur tient le `.exe` ; le tuer AVANT un rebuild).
- Slash commands : `/launch`, `/restart`, `/logs`, `/stop`, `/build`, `/test`.
- Plateforme : **Windows** (PowerShell). Le client de test est Bedrock
  **1.26.21** (wire = 1.26.20 = protocole 975). Serveur 127.0.0.1:19132.
- Logs utiles :
  - `logs/server.<date>.log` (rotation quotidienne)
  - `logs/CRASH-*.log` (panic)
  - `logs/FREEZE-*.log` (watchdog stall — ne se déclenche PAS pour ce bug
    per-session, voir §4)
  - grep utile : `grep -vE "watchdog|UnconnectedPing|recv pkt 0x02C"` pour voir
    l'activité jeu réelle ; `grep "split DROP\|recv_from error"` pour les WARN.

## 9. RÈGLES PROJET (CRITIQUE — l'utilisateur insiste)

- **Ne JAMAIS toucher `old_crates/`** (interdit total).
- PMMP `.reference/PocketMine-MP/` = seule référence valide. Pour ce bug :
  RakLib dans `.reference/PocketMine-MP/vendor/pocketmine/raklib/`.
- Vérifier le format binaire dans PMMP/RakLib AVANT d'implémenter, jamais deviner.
- Vérifier tout claim d'agent dans le code réel avant de coder (cf. faux
  positifs §3).
- Ne jamais affabuler "c'est réglé" sans preuve. Faire valider par
  l'utilisateur (test : miner en descendant longtemps).
- `.reference/server.log` est OBSOLÈTE — utiliser `logs/server.<date>.log`.
- Communication en français.
