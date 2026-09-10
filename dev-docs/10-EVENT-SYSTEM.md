# 10 - Event System

## PocketMine : Système d'événements

### Architecture

```
Event (abstract)
  ├── call() → dispatch à tous les handlers enregistrés
  ├── Cancellable (interface) → peut être annulé
  └── HandlerList → liste de RegisteredListener triés par priorité
```

### Priorités d'exécution (dans l'ordre)

| Priorité | Valeur | Usage |
|---|---|---|
| LOWEST | 5 | Premier exécuté, peut être override |
| LOW | 4 | Avant normal |
| NORMAL | 3 | Défaut |
| HIGH | 2 | Après normal |
| HIGHEST | 1 | Dernier avant monitor |
| MONITOR | 0 | Lecture seule, pas de modification |

### Dispatch d'un événement

```php
$event = new PlayerJoinEvent($player, $message);
$event->call();
// Après call(), l'événement peut avoir été modifié/annulé par les handlers

if ($event->isCancelled()) {
    return; // L'action ne se produit pas
}
$message = $event->getJoinMessage(); // Peut avoir été modifié
```

### Enregistrement de listeners

```php
class MyListener implements Listener {
    /**
     * @priority HIGHEST
     * @handleCancelled
     */
    public function onPlayerJoin(PlayerJoinEvent $event) : void {
        // Handler détecté par réflexion :
        // - Type du paramètre = type d'événement
        // - Annotations optionnelles : @priority, @handleCancelled, @notHandler
    }
}

$pluginManager->registerEvents(new MyListener(), $plugin);
```

### Catégories d'événements

**Player :**
- PlayerPreLoginEvent, PlayerLoginEvent, PlayerJoinEvent, PlayerQuitEvent
- PlayerChatEvent, PlayerCommandPreprocessEvent
- PlayerMoveEvent, PlayerJumpEvent, PlayerToggleSneakEvent, PlayerToggleSprintEvent
- PlayerInteractEvent, PlayerItemUseEvent, PlayerItemConsumeEvent
- PlayerBlockPickEvent, PlayerBlockPlaceEvent
- PlayerDeathEvent, PlayerRespawnEvent
- PlayerExperienceChangeEvent, PlayerGameModeChangeEvent
- PlayerBedEnterEvent, PlayerBedLeaveEvent
- PlayerDropItemEvent, PlayerItemHeldEvent
- PlayerKickEvent, PlayerTransferEvent
- PlayerChangeSkinEvent

**Block :**
- BlockBreakEvent, BlockPlaceEvent
- BlockUpdateEvent, BlockSpreadEvent
- BlockBurnEvent, BlockMeltEvent, BlockFadeEvent
- BlockGrowEvent, BlockFormEvent
- LeavesDecayEvent
- SignChangeEvent

**Entity :**
- EntityDamageEvent, EntityDamageByEntityEvent, EntityDamageByChildEntityEvent
- EntityDeathEvent
- EntitySpawnEvent, EntityDespawnEvent
- EntityMotionEvent, EntityTeleportEvent
- EntityExplodeEvent
- EntityShootBowEvent
- EntityRegainHealthEvent, EntityEffectAddEvent, EntityEffectRemoveEvent
- ProjectileHitEvent, ProjectileHitEntityEvent, ProjectileHitBlockEvent
- ItemSpawnEvent, ItemDespawnEvent

**Inventory :**
- InventoryOpenEvent, InventoryCloseEvent
- InventoryTransactionEvent
- CraftItemEvent
- FurnaceBurnEvent, FurnaceSmeltEvent

**World :**
- WorldLoadEvent, WorldUnloadEvent, WorldSaveEvent
- ChunkLoadEvent, ChunkUnloadEvent, ChunkPopulateEvent
- SpawnChangeEvent
- WorldInitEvent

**Server :**
- ServerStartEvent, ServerStopEvent
- QueryRegenerateEvent
- CommandEvent
- DataPacketReceiveEvent, DataPacketSendEvent

### Annulation

Les événements qui implémentent `Cancellable` peuvent être annulés :
```php
$event->cancel();     // Annuler
$event->uncancel();   // Dés-annuler
$event->isCancelled(); // Vérifier
```

Les handlers avec `@handleCancelled` reçoivent les événements même annulés.
Les handlers MONITOR ne doivent jamais modifier/annuler.

### Fichiers PocketMine de référence

```
src/event/Event.php
src/event/Cancellable.php
src/event/CancellableTrait.php
src/event/EventPriority.php
src/event/HandlerList.php
src/event/HandlerListManager.php
src/event/RegisteredListener.php
src/event/Listener.php
src/event/player/*.php
src/event/block/*.php
src/event/entity/*.php
src/event/inventory/*.php
src/event/world/*.php
src/event/server/*.php
```

---

## Équivalent Rust

### Crate : `mc-rs-event`

```rust
use std::any::{Any, TypeId};

/// Priorités d'événements
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum EventPriority {
    Lowest = 5,
    Low = 4,
    Normal = 3,
    High = 2,
    Highest = 1,
    Monitor = 0,
}

/// Trait de base pour tous les événements
pub trait Event: Any + Send + 'static {
    fn event_name(&self) -> &'static str;
    fn is_cancellable(&self) -> bool { false }
}

/// Trait pour les événements annulables
pub trait Cancellable: Event {
    fn is_cancelled(&self) -> bool;
    fn set_cancelled(&mut self, cancelled: bool);

    fn cancel(&mut self) { self.set_cancelled(true); }
    fn uncancel(&mut self) { self.set_cancelled(false); }
}

/// Macro pour créer un événement facilement
macro_rules! define_event {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $ty,)*
        }
        impl Event for $name {
            fn event_name(&self) -> &'static str { stringify!($name) }
        }
    };
    ($name:ident cancellable { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $ty,)*
            cancelled: bool,
        }
        impl Event for $name {
            fn event_name(&self) -> &'static str { stringify!($name) }
            fn is_cancellable(&self) -> bool { true }
        }
        impl Cancellable for $name {
            fn is_cancelled(&self) -> bool { self.cancelled }
            fn set_cancelled(&mut self, cancelled: bool) { self.cancelled = cancelled; }
        }
    };
}

// Exemples d'événements
define_event!(PlayerJoinEvent {
    player_id: EntityId,
    join_message: String,
});

define_event!(PlayerChatEvent cancellable {
    player_id: EntityId,
    message: String,
    format: String,
    recipients: Vec<EntityId>,
});

define_event!(BlockBreakEvent cancellable {
    player_id: EntityId,
    block_pos: BlockPos,
    block_state: BlockState,
    drops: Vec<ItemStack>,
    xp_drop: u32,
});

/// Handler enregistré
struct RegisteredHandler {
    priority: EventPriority,
    handle_cancelled: bool,
    plugin_id: PluginId,
    handler: Box<dyn Fn(&mut dyn Any) + Send + Sync>,
}

/// Gestionnaire d'événements global
pub struct EventManager {
    handlers: HashMap<TypeId, Vec<RegisteredHandler>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    /// Enregistrer un handler pour un type d'événement
    pub fn register<E: Event + 'static>(
        &mut self,
        priority: EventPriority,
        handle_cancelled: bool,
        plugin_id: PluginId,
        handler: impl Fn(&mut E) + Send + Sync + 'static,
    ) {
        let type_id = TypeId::of::<E>();
        let handlers = self.handlers.entry(type_id).or_default();

        handlers.push(RegisteredHandler {
            priority,
            handle_cancelled,
            plugin_id,
            handler: Box::new(move |event: &mut dyn Any| {
                if let Some(e) = event.downcast_mut::<E>() {
                    handler(e);
                }
            }),
        });

        // Trier par priorité (LOWEST=5 en premier, MONITOR=0 en dernier)
        handlers.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Dispatcher un événement à tous les handlers
    pub fn call<E: Event + 'static>(&self, event: &mut E) {
        let type_id = TypeId::of::<E>();
        if let Some(handlers) = self.handlers.get(&type_id) {
            for handler in handlers {
                // Skip si annulé et handler ne gère pas les annulés
                if event.is_cancellable() {
                    if let Some(c) = (event as &mut dyn Any).downcast_ref::<dyn Cancellable>() {
                        if c.is_cancelled() && !handler.handle_cancelled {
                            continue;
                        }
                    }
                }
                (handler.handler)(event as &mut dyn Any);
            }
        }
    }

    /// Désenregistrer tous les handlers d'un plugin
    pub fn unregister_plugin(&mut self, plugin_id: PluginId) {
        for handlers in self.handlers.values_mut() {
            handlers.retain(|h| h.plugin_id != plugin_id);
        }
    }
}
```
