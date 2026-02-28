# 07 - Mécaniques de jeu

## Tick System

Le serveur tourne à **20 ticks par seconde** (TPS), soit un tick toutes les **50 millisecondes**.

### Ordre de traitement par tick

```
1. Réception et traitement des paquets réseau
2. Mises à jour des entités
   a. IA des mobs (behaviors)
   b. Pathfinding
   c. Mouvement et physique
   d. Effets de potion
   e. Timers (invulnérabilité, cooldowns)
3. Random block ticks (crops, herbe, feuilles)
4. Scheduled block ticks (redstone, eau, lave)
5. Block entity ticks (fours, hoppers)
6. Vérification des spawns de mobs
7. Météo
8. Cycle jour/nuit
9. Chunk loading/unloading
10. Envoi des paquets de mise à jour
11. Sauvegarde auto (périodique)
```

### Simulation Distance

- **Ticking area** : Les chunks dans un rayon de `simulation-distance` autour de chaque joueur sont tickés
- Par défaut : 4 chunks (64 blocs)
- Les chunks en dehors de cette zone sont "gelés" (pas de block ticks, pas de spawns)
- Les chunks chargés mais non tickés sont uniquement envoyés visuellement

## Physique

### Constantes

```rust
const GRAVITY: f64 = 0.08;           // blocs/tick² (vers le bas)
const TERMINAL_VELOCITY: f64 = -3.92; // blocs/tick
const AIR_DRAG: f64 = 0.98;          // Multiplicateur par tick (Y)
const AIR_DRAG_XZ: f64 = 0.91;       // Multiplicateur par tick (X, Z) en l'air
const GROUND_FRICTION: f64 = 0.6;    // Friction au sol (variable selon le bloc)
const WATER_DRAG: f64 = 0.8;         // Drag dans l'eau
const LAVA_DRAG: f64 = 0.5;          // Drag dans la lave
const JUMP_VELOCITY: f64 = 0.42;     // Vélocité Y initiale d'un saut
const STEP_HEIGHT: f64 = 0.6;        // Hauteur max franchissable sans sauter
const PLAYER_WIDTH: f64 = 0.6;       // Largeur AABB joueur
const PLAYER_HEIGHT: f64 = 1.8;      // Hauteur AABB joueur
const PLAYER_EYE_HEIGHT: f64 = 1.62; // Hauteur des yeux
const SNEAK_HEIGHT: f64 = 1.5;       // Hauteur en accroupi
const SWIM_HEIGHT: f64 = 0.6;        // Hauteur en nage
```

### Simulation de mouvement

```
Chaque tick :
1. Appliquer les effets (Speed, Slowness, Jump Boost)
2. Si dans l'eau/lave : appliquer drag aquatique
3. Sinon : appliquer gravité (vel.y -= 0.08)
4. Appliquer drag aérien (vel.y *= 0.98)
5. Résoudre les collisions AABB
   a. Tester mouvement sur Y d'abord
   b. Tester mouvement sur X
   c. Tester mouvement sur Z
   d. Step-up si collision horizontale et hauteur ≤ 0.6
6. Appliquer friction
   - Au sol : vel.x *= slipperiness * 0.91, vel.z idem
   - En l'air : vel.x *= 0.91, vel.z *= 0.91
7. Mettre à jour on_ground
```

### Slipperiness des blocs

| Bloc | Slipperiness |
|------|-------------|
| Glace | 0.98 |
| Packed Ice | 0.98 |
| Blue Ice | 0.989 |
| Slime Block | 0.8 |
| Tous les autres | 0.6 |

### Collision AABB

```rust
// Pseudo-code de résolution de collision
fn resolve_collisions(pos: Vec3, vel: Vec3, bbox: AABB, world: &World) -> Vec3 {
    let expanded_bbox = bbox.expand_by(vel);
    let nearby_blocks = world.get_block_aabbs_in(expanded_bbox);

    let mut remaining_vel = vel;

    // Y en premier (gravité)
    for block_aabb in &nearby_blocks {
        remaining_vel.y = clip_y(bbox, block_aabb, remaining_vel.y);
    }
    bbox.offset_y(remaining_vel.y);

    // X ensuite
    for block_aabb in &nearby_blocks {
        remaining_vel.x = clip_x(bbox, block_aabb, remaining_vel.x);
    }
    bbox.offset_x(remaining_vel.x);

    // Z enfin
    for block_aabb in &nearby_blocks {
        remaining_vel.z = clip_z(bbox, block_aabb, remaining_vel.z);
    }

    remaining_vel
}
```

## Mouvement server-authoritative

### Validation du mouvement

En mode server-authoritative, le serveur doit simuler le mouvement :

```
1. Recevoir PlayerAuthInput du client
   - Position prédite par le client
   - Inputs (direction, saut, sprint, etc.)
   - Tick du client

2. Simuler le mouvement côté serveur
   - Appliquer les inputs aux vélocités
   - Simuler la physique (gravité, collisions)
   - Calculer la position attendue

3. Comparer avec la prédiction client
   - Si delta < seuil (0.1 bloc) : accepter la position client
   - Si delta ≥ seuil : envoyer une correction

4. Correction
   - Envoyer CorrectPlayerMovePrediction
   - Ou MovePlayer avec mode=Reset
```

### Tolérance et anti-triche

```rust
const POSITION_THRESHOLD: f64 = 0.1;    // Tolérance de position (blocs)
const SPEED_THRESHOLD: f64 = 0.5;       // Marge de vitesse
const MAX_VERTICAL_SPEED: f64 = 3.92;   // Vitesse verticale max
const MAX_HORIZONTAL_SPEED: f64 = 1.0;  // Vitesse horizontale max (sprint + effets)
```

## Combat

### Système Bedrock (pas de cooldown)

Contrairement à Java 1.9+, Bedrock n'a **PAS de cooldown d'attaque**. Chaque clic fait les dégâts complets.

### Dégâts par arme

| Arme | Dégâts |
|------|--------|
| Main nue | 1 |
| Épée bois | 5 |
| Épée pierre | 6 |
| Épée fer | 7 |
| Épée diamant | 8 |
| Épée netherite | 9 |
| Hache bois | 4 |
| Hache pierre | 5 |
| Hache fer | 6 |
| Hache diamant | 7 |
| Hache netherite | 8 |
| Trident | 9 |

### Calcul de dégâts

```
1. Dégâts de base = dégâts de l'arme
2. Enchantements offensifs :
   - Sharpness : +1.25 par niveau
   - Smite (vs undead) : +2.5 par niveau
   - Bane of Arthropods (vs arthropodes) : +2.5 par niveau
3. Effets de potion :
   - Strength : +3 par niveau
   - Weakness : -4 par niveau
4. Coup critique (en tombant) : ×1.5
5. Réduction par armure :
   - Chaque point d'armure réduit de 4%
   - Max 80% de réduction (20 points d'armure)
   - Formule : damage_taken = damage * (1 - min(20, armor_points) / 25)
6. Enchantements défensifs (Protection) :
   - Protection : -4% par niveau (tous types)
   - Fire/Blast/Projectile Protection : -8% par niveau (type spécifique)
   - Max 80% combiné
7. Invulnérabilité : 10 ticks (0.5s) après un coup
```

### Knockback

```
horizontal_kb = 0.4 (base) + 0.5 * knockback_level
vertical_kb = 0.4 (fixe)
Direction : depuis l'attaquant vers la cible
Sprint : +0.5 horizontal supplémentaire
```

## Système de faim

### Mécaniques

```
Food Level     : 0-20 (affiché comme 10 morceaux de viande)
Saturation     : 0.0-food_level (buffer invisible avant la faim)
Exhaustion     : 0.0-4.0 (accumulateur)

Quand exhaustion ≥ 4.0 :
  exhaustion -= 4.0
  Si saturation > 0 : saturation -= 1
  Sinon : food_level -= 1

Régénération (food_level ≥ 18) :
  - 1 demi-cœur toutes les 80 ticks (4 secondes)
  - Coût : 6 exhaustion par demi-cœur

Famine (food_level == 0) :
  - 1 demi-cœur de dégâts toutes les 80 ticks
  - Easy : s'arrête à 10 HP
  - Normal : s'arrête à 1 HP
  - Hard : peut tuer
```

### Sources d'exhaustion

| Action | Exhaustion |
|--------|-----------|
| Nager (par mètre) | 0.01 |
| Casser un bloc | 0.005 |
| Sprinter (par mètre) | 0.1 |
| Sauter | 0.05 |
| Sprint-sauter | 0.2 |
| Attaquer | 0.1 |
| Recevoir des dégâts | 0.1 |
| Faim (régénération) | 6.0 |

## Système d'expérience

### Formules XP

```
XP pour passer au niveau suivant :
  Niveau 0-16  : 2 * level + 7
  Niveau 17-31 : 5 * level - 38
  Niveau 32+   : 9 * level - 158

XP total pour atteindre un niveau :
  Niveau 0-16  : level² + 6 * level
  Niveau 17-31 : 2.5 * level² - 40.5 * level + 360
  Niveau 32+   : 4.5 * level² - 162.5 * level + 2220
```

### Sources d'XP

| Source | XP |
|--------|---|
| Minerai de charbon | 0-2 |
| Minerai de diamant | 3-7 |
| Minerai d'émeraude | 3-7 |
| Minerai de lapis | 2-5 |
| Minerai de redstone | 1-5 |
| Mob hostile | 5 |
| Blaze | 10 |
| Ender Dragon (premier) | 12000 |
| Ender Dragon (suivants) | 500 |
| Four (par item fondu) | variable |
| Pêche | 1-6 |
| Commerce | 3-6 |
| Élevage | 1-7 |

## Block Ticks

### Random Ticks

- Chaque tick de jeu, `randomTickSpeed` blocs sont sélectionnés aléatoirement dans chaque sub-chunk tické
- Par défaut `randomTickSpeed = 1` sur Bedrock (3 sur Java)
- Blocs affectés : crops, saplings, herbe, feuilles, feu, neige, glace, cactus, canne à sucre, champignons, vignes

### Scheduled Ticks

Certains blocs programment un tick futur :
- **Eau** : Coule tous les 5 ticks
- **Lave** : Coule tous les 30 ticks (Overworld) / 10 ticks (Nether)
- **Redstone** : 2 ticks de délai par composant
- **Pistons** : 2 ticks pour extension, 0 pour rétraction (Bedrock instantané !)
- **TNT** : 80 ticks de fusible (4 secondes)
- **Sable/gravier** : Vérifie la chute au prochain tick

### Redstone — Différences Bedrock vs Java

| Mécanisme | Java | Bedrock |
|-----------|------|---------|
| Pistons | 2 ticks (1 redstone tick) | **Instantané** (0 tick) |
| Quasi-connectivity | Oui | **Non** |
| Update order | Déterministe | Plus aléatoire |
| Comparateur | Latence 1 tick | Latence 1 tick |
| Observateur | 1 tick de pulse | 2 ticks de pulse |

## Inventaire et Crafting

### Système ItemStackRequest (moderne)

Le client envoie un `ItemStackRequest` décrivant ce qu'il veut faire :

```
1. Le client manipule son inventaire local (prédiction)
2. Le client envoie ItemStackRequest avec les actions
3. Le serveur valide :
   - Les items source existent-ils ?
   - Le déplacement est-il légal ?
   - La recette est-elle valide ? (si craft)
4. Le serveur répond ItemStackResponse :
   - Accepté : confirme les changements
   - Refusé : le client annule sa prédiction

Chaque item a un StackNetworkId unique assigné par le serveur.
```

### Types de recettes

| Type | Description | Exemple |
|------|-------------|---------|
| Shaped | Recette avec forme définie | Pioche (forme en T) |
| Shapeless | Recette sans forme | Teinture + laine |
| Furnace | Four / Haut fourneau / Fumoir | Minerai → lingot |
| Smithing | Table de forgeron | Diamant → Netherite |
| Brewing | Support d'alchimie | Potions |
| Stonecutter | Tailleur de pierre | Pierre → escalier |

### CraftingData (0x34)

Ce paquet envoie **toutes les recettes** au client au login. Il contient des milliers d'entrées.

## Commandes

### Architecture (type Brigadier)

```
/give @a[type=zombie,r=10] diamond_sword 1

Parsing :
  "give"           → Literal node
  "@a[...]"        → EntitySelector argument
  "diamond_sword"  → Item argument
  "1"              → Int argument (optionnel, default 1)
```

### Entity Selectors

| Sélecteur | Signification |
|-----------|---------------|
| `@a` | Tous les joueurs |
| `@p` | Joueur le plus proche |
| `@r` | Joueur aléatoire |
| `@e` | Toutes les entités |
| `@s` | L'entité qui exécute |
| `@initiator` | L'initiateur (NPC) |

### Filtres de sélecteur

```
@e[type=zombie]           # Type d'entité
@a[r=10]                  # Rayon
@a[rm=5,r=10]             # Rayon min/max
@a[x=0,y=64,z=0]         # Position de référence
@a[scores={obj=10..20}]   # Score
@a[tag=vip]               # Tag
@a[name="Steve"]          # Nom
@a[m=creative]            # Gamemode
@a[l=30,lm=10]            # Niveau XP
@a[c=3]                   # Nombre max de résultats
@e[family=monster]        # Famille d'entité
```

### Commandes essentielles à implémenter

| Priorité | Commandes |
|----------|-----------|
| **Critique** | `help`, `stop`, `say`, `tell`, `me`, `list` |
| **Haute** | `gamemode`, `tp`, `give`, `kill`, `effect`, `time`, `weather` |
| **Moyenne** | `setblock`, `fill`, `clone`, `summon`, `clear`, `enchant` |
| **Basse** | `scoreboard`, `tag`, `execute`, `function`, `playsound` |
| **Avancée** | `title`, `bossbar`, `particle`, `structure`, `loot` |

## Permissions

### Niveaux Bedrock

| Niveau | Nom | Description |
|--------|-----|-------------|
| 0 | Visitor | Peut se déplacer et regarder |
| 1 | Member | Peut interagir avec le monde |
| 2 | Operator | Peut utiliser les commandes |
| 3 | Custom | Personnalisé |

### Abilities Layer

Bedrock utilise un système de "couches" pour les permissions :

```
Base layer (type du joueur) :
  → Member abilities (build, mine, doors, containers, attack)

Gamemode layer :
  → Creative : fly, instabuild, invulnerable
  → Adventure : no build, no mine
  → Spectator : no clip, fly, invisible, invulnerable

Custom layer (commandes/plugins) :
  → Peut override n'importe quelle ability
```

## Formulaires (Forms UI)

### SimpleForm

```json
{
    "type": "form",
    "title": "Menu Principal",
    "content": "Choisissez une option :",
    "buttons": [
        {"text": "Jouer", "image": {"type": "path", "data": "textures/ui/play_button"}},
        {"text": "Options"},
        {"text": "Quitter"}
    ]
}
```
Réponse : index du bouton cliqué (0, 1, 2) ou `null` si fermé.

### ModalForm

```json
{
    "type": "modal",
    "title": "Confirmation",
    "content": "Êtes-vous sûr ?",
    "button1": "Oui",
    "button2": "Non"
}
```
Réponse : `true` (button1) ou `false` (button2).

### CustomForm

```json
{
    "type": "custom_form",
    "title": "Configuration",
    "content": [
        {"type": "label", "text": "Paramètres du serveur"},
        {"type": "input", "text": "Nom", "placeholder": "Entrez votre nom", "default": "Steve"},
        {"type": "toggle", "text": "PvP activé", "default": true},
        {"type": "slider", "text": "Difficulté", "min": 0, "max": 3, "step": 1, "default": 2},
        {"type": "dropdown", "text": "Gamemode", "options": ["Survival", "Creative", "Adventure"], "default": 0},
        {"type": "step_slider", "text": "Render Distance", "steps": ["4", "8", "12", "16"], "default": 1}
    ]
}
```
Réponse : tableau JSON des valeurs `[null, "Steve", true, 2, 0, 1]`.

## Scoreboard

### Composants

- **Objective** : Un critère de scoring (ex: "kills", "deaths")
- **Score** : Valeur associée à un joueur/entité pour un objectif
- **Display Slot** : Où afficher (sidebar, list, below_name)

### Paquets

- `SetDisplayObjective` : Associer un objectif à un emplacement
- `SetScore` : Mettre à jour les scores
- `RemoveObjective` : Supprimer un objectif

### Critères Bedrock

| Critère | Description |
|---------|-------------|
| `dummy` | Modifiable uniquement par commandes |
| `player_kill_count` | Joueurs tués |
| `total_kill_count` | Total entités tuées |
| `death_count` | Nombre de morts |

## Météo et cycle jour/nuit

### Cycle jour/nuit

```
Durée d'un jour complet : 24000 ticks (20 minutes réelles)

0      = Lever du soleil
1000   = Jour
6000   = Midi
12000  = Coucher du soleil
13000  = Nuit
18000  = Minuit
23000  = Pré-aube
24000  = Retour à 0
```

### Météo

```
Pluie :
  - Durée : 12000-24000 ticks
  - Temps entre deux pluies : 12000-168000 ticks
  - Inhibe le spawning de certains mobs (en surface)
  - Éteint les feux de camp (Bedrock)

Orage :
  - Peut se produire pendant la pluie
  - Lightning : dégâts de 5 cœurs, enflamme les blocs
  - Spawn de squelettes piégeurs, zombie villageois
  - Creeper → Charged Creeper si frappé
```

## Effets de potion

### Effets principaux

| ID | Nom | Effet |
|----|-----|-------|
| 1 | Speed | +20% vitesse par niveau |
| 2 | Slowness | -15% vitesse par niveau |
| 3 | Haste | +20% vitesse de minage par niveau |
| 4 | Mining Fatigue | Réduit vitesse de minage |
| 5 | Strength | +3 dégâts par niveau |
| 6 | Instant Health | Soigne 4 HP par niveau |
| 7 | Instant Damage | 6 dégâts par niveau |
| 8 | Jump Boost | +50% hauteur de saut par niveau |
| 9 | Nausea | Distorsion visuelle |
| 10 | Regeneration | Regen 1 HP / 50 ticks / niveau |
| 11 | Resistance | -20% dégâts par niveau |
| 12 | Fire Resistance | Immunité au feu |
| 13 | Water Breathing | Respiration sous l'eau |
| 14 | Invisibility | Invisible |
| 15 | Blindness | Visibilité réduite |
| 16 | Night Vision | Vision nocturne |
| 17 | Hunger | Exhaustion accélérée |
| 18 | Weakness | -4 dégâts |
| 19 | Poison | 1 dégât / 25 ticks / niveau (min 1 HP) |
| 20 | Wither | 1 dégât / 40 ticks / niveau (peut tuer) |
| 21 | Health Boost | +4 HP max par niveau |
| 22 | Absorption | +4 HP absorption par niveau |
| 23 | Saturation | Restaure saturation |
| 25 | Levitation | Monte de 0.9 blocs/sec par niveau |
| 26 | Fatal Poison | Comme Poison mais peut tuer |
| 27 | Conduit Power | Vision sous-marine + mining speed |
| 28 | Slow Falling | Chute lente (pas de dégâts de chute) |
