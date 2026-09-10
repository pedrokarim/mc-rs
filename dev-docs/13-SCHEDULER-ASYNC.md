# 13 - Scheduler & Async

## PocketMine : Système de tâches

### Architecture

```
Server
├── TaskScheduler (global)          → tâches internes du serveur
├── AsyncPool (thread pool)         → tâches asynchrones
└── Plugins
    └── plugin.getScheduler()       → TaskScheduler par plugin
```

### TaskScheduler (synchrone, main thread)

Exécute des tâches sur le main thread à des intervalles définis.

```php
// Tâche immédiate (prochain tick)
$scheduler->scheduleTask(new MyTask());

// Tâche retardée (après N ticks)
$scheduler->scheduleDelayedTask(new MyTask(), 20); // 1 seconde

// Tâche répétée (chaque N ticks)
$scheduler->scheduleRepeatingTask(new MyTask(), 20); // chaque seconde

// Tâche retardée + répétée
$scheduler->scheduleDelayedRepeatingTask(new MyTask(), 100, 20); // 5s delay, puis chaque 1s
```

**TaskHandler :**
- Wrapper autour d'une Task
- Gère : delay, period, nextRun, cancelled
- `cancel()` : annule la tâche

**Tick du scheduler :**
```
Pour chaque tick :
  while queue.peek().nextRun <= currentTick :
    handler = queue.pop()
    handler.run()
    if handler.isRepeating() && !handler.isCancelled() :
      handler.nextRun = currentTick + handler.period
      queue.push(handler)
```

### AsyncTask (worker threads)

Pour le travail CPU-bound sans bloquer le main thread.

```php
class ChunkGenerationTask extends AsyncTask {
    // Données sérialisées vers le worker
    private string $serializedData;

    // Exécuté dans le worker thread
    public function onRun() : void {
        // PAS d'accès au Server ici !
        $result = $this->generateChunk();
        $this->setResult($result);
    }

    // Exécuté sur le main thread après completion
    public function onCompletion() : void {
        $result = $this->getResult();
        // Accès au Server OK ici
        $this->fetchLocal("world")->applyChunk($result);
    }
}

// Soumettre
$server->getAsyncPool()->submitTask(new ChunkGenerationTask(...));
```

**AsyncPool :**
- Pool de workers threads (configurable, défaut 2)
- `submitTask(task)` : soumet à un worker
- Les résultats sont collectés sur le main thread via `collectTasks()`

**Cycle de vie AsyncTask :**
1. `storeLocal(key, value)` — stocker des objets non-sérialisables (pour onCompletion)
2. Task sérialisée et envoyée au worker
3. Worker exécute `onRun()`
4. `onProgressUpdate()` appelé si `publishProgress()` utilisé
5. Worker marque la task comme terminée
6. Main thread collecte et appelle `onCompletion()`

### Tâches internes importantes

| Tâche | Type | Description |
|---|---|---|
| `PopulationTask` | Async | Génération de chunks |
| `LightPopulationTask` | Async | Calcul de lumière |
| `ChunkRequestTask` | Async | Sérialisation de chunks pour le réseau |
| `CompressBatchTask` | Async | Compression de paquets |
| `PrepareEncryptionTask` | Async | Génération de clés ECDSA |
| `FetchAuthKeysTask` | Async | Récupération clés Xbox Live |
| `SendUsageTask` | Async | Télémétrie anonyme |

### ClosureTask

```php
$scheduler->scheduleRepeatingTask(new ClosureTask(function() {
    // Code à exécuter
}), 20);
```

### CancelTaskException

Lancer `CancelTaskException` dans une tâche la cancel automatiquement :
```php
public function onRun() : void {
    if ($this->shouldStop) {
        throw new CancelTaskException();
    }
}
```

### Fichiers PocketMine de référence

```
src/scheduler/Task.php
src/scheduler/ClosureTask.php
src/scheduler/TaskScheduler.php
src/scheduler/TaskHandler.php
src/scheduler/AsyncTask.php
src/scheduler/AsyncPool.php
src/scheduler/AsyncWorker.php
src/scheduler/CancelTaskException.php
```

---

## Équivalent Rust

### Crate : intégré dans `mc-rs-server`

```rust
use std::collections::BinaryHeap;

/// Handle vers une tâche planifiée
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskHandle(u64);

/// Tâche planifiée
struct ScheduledTask {
    handle: TaskHandle,
    next_run: u64,        // tick d'exécution
    period: Option<u64>,  // None = one-shot
    task: Box<dyn FnMut() + Send>,
    cancelled: bool,
}

/// Scheduler synchrone (main thread)
pub struct Scheduler {
    tasks: BinaryHeap<ScheduledTask>,  // min-heap par next_run
    next_handle: u64,
}

impl Scheduler {
    pub fn schedule(&mut self, task: impl FnOnce() + Send + 'static) -> TaskHandle {
        self.schedule_delayed(0, task)
    }

    pub fn schedule_delayed(&mut self, delay: u64, task: impl FnOnce() + Send + 'static) -> TaskHandle {
        let handle = self.next_handle();
        let mut called = false;
        self.tasks.push(ScheduledTask {
            handle,
            next_run: self.current_tick + delay,
            period: None,
            task: Box::new(move || {
                if !called { task(); called = true; }
            }),
            cancelled: false,
        });
        handle
    }

    pub fn schedule_repeating(&mut self, period: u64, task: impl FnMut() + Send + 'static) -> TaskHandle {
        self.schedule_delayed_repeating(0, period, task)
    }

    pub fn schedule_delayed_repeating(
        &mut self,
        delay: u64,
        period: u64,
        task: impl FnMut() + Send + 'static,
    ) -> TaskHandle {
        let handle = self.next_handle();
        self.tasks.push(ScheduledTask {
            handle,
            next_run: self.current_tick + delay,
            period: Some(period),
            task: Box::new(task),
            cancelled: false,
        });
        handle
    }

    pub fn cancel(&mut self, handle: TaskHandle) {
        // Marquer comme annulé (sera nettoyé au tick)
        for task in self.tasks.iter_mut() {
            if task.handle == handle {
                task.cancelled = true;
            }
        }
    }

    pub fn tick(&mut self, current_tick: u64) {
        while let Some(task) = self.tasks.peek() {
            if task.next_run > current_tick || task.cancelled {
                if task.cancelled {
                    self.tasks.pop();
                    continue;
                }
                break;
            }

            let mut task = self.tasks.pop().unwrap();
            (task.task)();

            if let Some(period) = task.period {
                if !task.cancelled {
                    task.next_run = current_tick + period;
                    self.tasks.push(task);
                }
            }
        }
    }
}

/// Pool de tâches asynchrones
pub struct AsyncPool {
    runtime: tokio::runtime::Runtime,
    results: Arc<Mutex<Vec<AsyncResult>>>,
}

struct AsyncResult {
    task_id: u64,
    result: Box<dyn Any + Send>,
    completion: Box<dyn FnOnce(Box<dyn Any + Send>) + Send>,
}

impl AsyncPool {
    pub fn new(workers: usize) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(workers)
            .build()
            .unwrap();
        Self {
            runtime,
            results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Soumettre une tâche async avec callback de completion
    pub fn submit<T, F, C>(&self, work: F, on_complete: C)
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
        C: FnOnce(T) + Send + 'static,
    {
        let results = self.results.clone();
        let task_id = self.next_task_id();

        self.runtime.spawn_blocking(move || {
            let result = work();
            results.lock().unwrap().push(AsyncResult {
                task_id,
                result: Box::new(result),
                completion: Box::new(|r| {
                    if let Ok(typed) = r.downcast::<T>() {
                        on_complete(*typed);
                    }
                }),
            });
        });
    }

    /// Collecter les résultats sur le main thread
    pub fn collect(&self) {
        let results: Vec<AsyncResult> = {
            let mut lock = self.results.lock().unwrap();
            std::mem::take(&mut *lock)
        };

        for result in results {
            (result.completion)(result.result);
        }
    }
}
```
