# 24 - Chunk Loading & Sending

## Problème actuel

Les chunks arrêtent de charger après un moment d'exploration. La zone initiale
se charge correctement mais les nouveaux chunks ne se chargent plus quand le joueur
se déplace.

---

## Analyse : PMMP vs notre implémentation

### 5 différences critiques identifiées

| # | Aspect | PMMP | Notre code | Impact |
|---|---|---|---|---|
| 1 | **Ordre d'envoi** | Spirale depuis le joueur (plus proche d'abord) | Boucle carrée `for dx..for dz` | Le joueur voit d'abord les chunks lointains au lieu des proches |
| 2 | **Rate limiting** | 4 chunks/tick max (`chunksPerTick`) | TOUS les chunks d'un coup | Flood de données → timeout client |
| 3 | **Déchargement** | Chunks hors vue distance sont dépawned | `sent_chunks` grandit à l'infini | Mémoire illimitée, état incohérent |
| 4 | **Timing** | Recalcul débounced (5-20 ticks) | Recalcul à chaque `PlayerAuthInput` (10-20x/sec) | CPU gaspillé, flood réseau |
| 5 | **Génération** | Asynchrone (ChunkRequestTask) | Synchrone dans le handler de paquets (corrigé le 2026-11-08, voir Phase 4) | Bloque le thread principal |

---

## Références PMMP

### Fichiers clés

| Fichier | Rôle |
|---|---|
| `src/player/ChunkSelector.php` | Algorithme spirale pour sélection des chunks par distance |
| `src/player/Player.php` | `orderChunks()` (L999), `requestChunks()` (L855), `unloadChunk()` (L814) |
| `src/network/mcpe/NetworkSession.php` | `tick()` → `doChunkRequests()` (L1521), `syncViewAreaCenterPoint()` (L1199) |
| `src/network/mcpe/handler/InGamePacketHandler.php` | Handling de `PlayerAuthInput` pour le mouvement |

### Notre code

| Fichier | Lignes | Rôle |
|---|---|---|
| `crates/mc-rs-server/src/connection.rs` | L446-519 | `RequestChunkRadius` → envoi initial |
| `crates/mc-rs-server/src/connection.rs` | L626-687 | Mouvement → envoi de chunks |
| `crates/mc-rs-server/src/connection.rs` | L64 | `sent_chunks: HashSet` |
| `crates/mc-rs-server/src/world/chunk_cache.rs` | L38-152 | Cache mémoire + LevelDB |

---

## Flow PMMP (détaillé)

### 1. ChunkSelector — Spirale d'Archimède

```
Chunks sélectionnés par distance croissante :
  Distance 0: (0,0) — chunk du joueur
  Distance 1: (1,0) (0,1) (-1,0) (0,-1) (1,1) (-1,1) (-1,-1) (1,-1)
  Distance 2: (2,0) (2,1) ... etc.
```

La spirale visite tous les chunks dans un rayon donné, triés par distance.
Résultat : le joueur voit d'abord son environnement immédiat.

### 2. orderChunks() — Debounced

```php
if($this->nextChunkOrderRun-- <= 0) {
    $this->orderChunks();
}
```

- `nextChunkOrderRun` est mis à 0 quand le joueur change de chunk
- Mis à 20 quand le joueur se déplace sans changer de chunk
- Mis à `PHP_INT_MAX` quand rien ne change (ne s'exécute plus)
- Résultat : recalcul seulement quand nécessaire

### 3. requestChunks() — Rate limited

```php
$limit = $this->chunksPerTick - count($this->activeChunkGenerationRequests);
foreach($this->loadQueue as $index => $distance) {
    if($count >= $limit) break;  // STOP après N chunks
    // Demander la génération asynchrone du chunk
    $count++;
}
```

- Maximum 4 chunks générés par tick (configurable)
- Priorise les chunks proches (la queue est ordonnée)
- Ne bloque pas le thread principal

### 4. Unloading

```php
$unloadChunks = $this->usedChunks; // Copie de tous les chunks connus
foreach($this->chunkSelector->selectChunks(...) as $hash) {
    unset($unloadChunks[$hash]); // Retire ceux encore en vue
}
// Ce qui reste → à décharger
foreach($unloadChunks as $index => $status) {
    $this->unloadChunk($X, $Z);
}
```

### 5. UsedChunkStatus — State machine

```
NEEDED → REQUESTED_GENERATION → REQUESTED_SENDING → SENT
  ↑                                                    |
  └────────── onChunkChanged() ────────────────────────┘
```

- NEEDED : le chunk est dans la vue, pas encore généré
- REQUESTED_GENERATION : génération en cours (async)
- REQUESTED_SENDING : sérialisé, en cours d'envoi
- SENT : le client l'a reçu
- Si le chunk est modifié (block break, etc.) : revient à NEEDED

---

## Plan d'implémentation

### Phase 1 : Rate limiting + spirale (fix immédiat)

**Objectif** : Empêcher le flood de chunks et prioriser les proches.

1. Ajouter un `chunk_load_queue: VecDeque<(i32, i32)>` dans `Connection`
2. Remplir la queue en spirale depuis la position du joueur
3. Envoyer max **8 chunks par tick** (pas par paquet)
4. Déplacer l'envoi de chunks dans la boucle principale (tick-based, pas event-based)

### Phase 2 : Chunk unloading

**Objectif** : Libérer les chunks hors vue distance.

1. Quand `orderChunks()` est appelé, calculer quels chunks sont hors vue
2. Retirer ces chunks de `sent_chunks`
3. Le client se charge de les dégager visuellement tout seul

### Phase 3 : Debounce du recalcul

**Objectif** : Ne pas recalculer la queue à chaque mouvement.

1. Ajouter `next_chunk_order_tick: u64` dans `Connection`
2. Sur changement de chunk : `next_chunk_order_tick = current_tick`
3. Sur mouvement normal : `next_chunk_order_tick = min(current, current + 20)`
4. Dans le tick loop : `if current_tick >= next_chunk_order_tick { reorder() }`

### Phase 4 : Génération asynchrone (faite le 2026-11-08)

**Objectif** : Ne pas bloquer le thread principal.

Avant, un chunk absent était chargé de la LevelDB ou généré dans la boucle principale au moment de l'envoi.
Avec le générateur « noise », un chunk coûte de 90 à 200 ms selon la charge de la machine : tout le serveur
se figeait pendant qu'un joueur explorait.

Ce qui est en place :

- `world/chunk_workers.rs` : `ChunkWorkers`, des threads (`chunk-worker-N`) qui exécutent les tâches d'une
  file partagée par les trois dimensions. Nombre : `[world] generation_threads` dans `server.toml`, 0 par
  défaut, soit les cœurs moins deux comme l'`async-workers: auto` de PMMP (`Server.php`), entre 1 et 4.
  Ils sont branchés au démarrage (`Worlds::start_chunk_workers`, appelé dans `main.rs`).
- `ChunkCache::request_chunk(cx, cz)` : vrai si le chunk est en mémoire. Sinon, il est confié une seule fois
  aux workers (ensemble `pending`, l'équivalent de `REQUESTED_GENERATION`), et la réponse est faux. Sans
  workers, comme dans les tests, il est chargé sur place.
- Le worker lit la LevelDB ou génère le chunk, puis construit d'avance le paquet LevelChunk compressé
  (`ChunkColumn::level_chunk_zlib_batch`), sans tenir ni le verrou du cache ni celui de la LevelDB pendant
  le calcul. Il prend ensuite le verrou du cache pour insérer le chunk, sauf s'il y est déjà parce que la
  boucle principale l'a chargé entre-temps (on garde alors celui-là, peut-être déjà modifié). Une panique
  pendant la génération est rattrapée : le chunk est redemandé au prochain envoi.
- `Connection::send_chunk_batch` (`connection/chunks.rs`) n'envoie que les chunks prêts. Les autres sont
  demandés et remis en tête de file dans leur ordre, au plus 16 par joueur et par envoi
  (`MAX_CHUNK_REQUESTS_AHEAD`), pour ne pas générer pour rien une zone que le joueur quitte.
- L'apparition naturelle des mobs (`mob_spawner.rs`) ignore les chunks pas encore chargés (PMMP
  `World::isChunkLoaded`) au lieu de les générer sur place.
- La passe de surface clone son `SurfaceBuilder` (`Arc`) au lieu de garder son verrou pendant tout le
  calcul, ce qui aurait fait attendre les workers les uns après les autres.

Mesure (`chunk_cache::tests::measure_chunk_workers`, ignoré, en release, sur une machine chargée par
d'autres programmes) : 64 chunks « noise » prennent 12,6 s sur place (197 ms chacun) contre 2,5 s avec
4 workers ; la boucle qui les demande n'est bloquée que 2 ms au total.

Restent synchrones, dans le thread qui les appelle : toute lecture de bloc (`get_block`, `set_block`…) dans
un chunk absent, c'est-à-dire le spawn et les téléportations (recherche d'un sol sûr sur quelques chunks),
les entités (mobs, objets, flèches) qui sortent de la zone chargée, et les commandes. PMMP ne fait pas
bouger les entités dans un chunk non chargé ; ce n'est pas encore le cas ici.

---

## Métriques cibles

| Métrique | Actuel | Cible |
|---|---|---|
| Chunks envoyés/sec | Burst de 200+ | 8/tick × 20 = 160/sec max |
| Taille de `sent_chunks` | ∞ (croît) | ≤ `(2r+1)²` = ~1000 max |
| Latence mouvement→chunk visible | Instant (si ça marche) ou ∞ | ~0.5s (8 chunks en 1 tick) |
| Mémoire chunk côté serveur | ∞ | Bornée par vue distance |
