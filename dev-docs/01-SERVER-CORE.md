# 01 - Server Core

## PocketMine : comment ça marche

### Point d'entrée

`PocketMine.php` → vérifie les dépendances → crée `Server` → appelle `tickProcessor()`.

### Boucle principale (20 TPS)

```
while running:
    tick()
    sleep_until(next_tick)  // auto-correctif
```

Chaque tick (50ms) exécute dans l'ordre :

1. **Scheduler tick** — exécuter les tâches planifiées
2. **Async pool collect** — récupérer les résultats des tâches asynchrones
3. **World tick** — tous les mondes chargés font un tick
4. **Network tick** — traiter les paquets de toutes les interfaces réseau
5. **Opérations par seconde** (1x toutes les 20 ticks) :
   - Mise à jour barre de titre
   - Régénération infos Query
   - Rotation stats bande passante
6. **Vérification mémoire**
7. **Entrée console** — traiter les commandes console
8. **Calcul TPS/performance**

### Constantes clés

| Constante | Valeur | Description |
|---|---|---|
| `TARGET_TICKS_PER_SECOND` | 20 | 50ms par tick |
| `TPS_OVERLOAD_WARNING` | 12 | Seuil d'alerte surcharge |

### Initialisation du serveur (ordre)

1. Charger config (`pocketmine.yml`, `server.properties`)
2. Initialiser MemoryManager
3. Initialiser AsyncPool (workers threads)
4. Initialiser CommandMap
5. Initialiser CraftingManager (recettes)
6. Initialiser ResourcePackManager
7. Initialiser PluginManager → charger plugins (phase STARTUP)
8. Initialiser WorldManager → charger les mondes
9. Activer plugins (phase POSTWORLD)
10. Initialiser interfaces réseau (RakLib, Query, UPnP)
11. Démarrer console input handler
12. Entrer dans `tickProcessor()`

---

## Équivalent Rust

### Crate : `mc-rs-server`

```rust
pub struct Server {
    config: ServerConfig,
    network: NetworkManager,
    worlds: WorldManager,
    plugins: PluginManager,
    commands: CommandMap,
    crafting: CraftingManager,
    scheduler: Scheduler,
    players: PlayerManager,
    running: AtomicBool,
    current_tick: u64,
}

impl Server {
    pub fn new(config: ServerConfig) -> Result<Self>;
    pub fn run(&mut self); // boucle principale
    fn tick(&mut self);
    pub fn shutdown(&mut self);
}
```

### Boucle principale Rust

```rust
fn run(&mut self) {
    let tick_duration = Duration::from_millis(50);

    while self.running.load(Ordering::Relaxed) {
        let tick_start = Instant::now();

        self.tick();

        let elapsed = tick_start.elapsed();
        if elapsed < tick_duration {
            std::thread::sleep(tick_duration - elapsed);
        }

        self.current_tick += 1;
    }
}

fn tick(&mut self) {
    self.scheduler.tick(self.current_tick);
    self.scheduler.collect_async_results();
    self.worlds.tick(self.current_tick);
    self.network.tick();

    if self.current_tick % 20 == 0 {
        self.update_tps();
        self.check_memory();
    }

    self.process_console_commands();
}
```

### Modèle de concurrence

| PocketMine (PHP) | MC-RS (Rust) |
|---|---|
| Main thread (tick loop) | Main thread (tick loop) |
| pmmpthread (network) | tokio async runtime |
| AsyncPool (workers) | rayon / tokio::spawn_blocking |
| Logger thread | tracing + async file writer |

### Config : `server.toml`

```toml
[server]
motd = "MC-RS Server"
port = 19132
max_players = 20
online_mode = true  # Xbox Live auth

[world]
name = "world"
generator = "flat"
seed = 0

[game]
gamemode = "survival"
difficulty = "normal"
pvp = true

[network]
compression_threshold = 256
compression_algorithm = "zlib"  # ou "snappy"

[logging]
level = "info"
file = "server.log"
```
