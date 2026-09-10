# 25 - Extraction De La Block Palette

## Pourquoi Ce Doc Existe

`mc-rs` utilise des runtime IDs Bedrock séquentiels pour les chunks, les structures et les mises à jour de blocs.

Le point important est que :

- PocketMine PHP ne génère pas `canonical_block_states.nbt`
- le fichier vient d’un mod natif BDS séparé : `pmmp/bds-mod-mapping`
- le serveur Bedrock dédié est la vraie source d’autorité pour l’ordre des runtime IDs

Le but de ce doc est de rendre ce workflow reproductible sans dépendre d’une “magie PocketMine”.

## Ce Que Fait Réellement PMMP

Le repo `pmmp/bds-mod-mapping` charge la palette interne de BDS au démarrage du serveur et écrit directement `mapping_files/canonical_block_states.nbt`.

Le coeur de l’extraction ressemble à ceci :

```cpp
auto palette = serverInstance->getMinecraft()->getLevel()->getBlockPalette();
unsigned int numStates = palette->getNumBlockNetworkIds();

for (unsigned int i = 0; i < numStates; i++) {
    auto state = palette->getBlock(i);
    paletteStream->writeType(state->tag, "", "");
}

std::ofstream paletteOutput("mapping_files/canonical_block_states.nbt");
paletteOutput << paletteStream->buffer;
```

Source upstream :

- `pmmp/bds-mod-mapping/src/main.cpp`
- `pmmp/BedrockData/README.md`

Important : `tools/generate-bedrock-data-from-packets.php` dans PocketMine génère d’autres fichiers BedrockData depuis des traces réseau, mais pas `canonical_block_states.nbt`. Il consomme ce fichier, il ne le produit pas.

## Format Du Fichier

`canonical_block_states.nbt` est une suite de `TAG_Compound` en **Network Little-Endian NBT**.

Règles pratiques :

- chaque entrée correspond à un block state canonique
- le runtime ID réseau est simplement l’index de cette entrée dans le fichier
- pour `mc-rs`, on retient le premier runtime ID vu pour chaque nom de bloc quand on a seulement un `minecraft:foo` sans états détaillés

## Extraction Recommandée

La méthode recommandée est d’utiliser `pmmp/bds-modding-devkit` dans un environnement Linux ou WSL2.

### Pré-requis

- WSL2 ou Linux
- `python3`
- `pip`
- `clang`
- `libc++-dev`
- `libc++abi-dev`
- `binutils`
- `cmake`
- une archive ou un dossier de `Bedrock Dedicated Server`

### Étapes

1. Cloner `pmmp/bds-modding-devkit`
2. Initialiser les submodules
3. Préparer le venv Python
4. Installer les dépendances Python
5. Installer les fichiers BDS dans le devkit
6. Lancer le serveur moddé

Exemple :

```bash
git clone https://github.com/pmmp/bds-modding-devkit.git
cd bds-modding-devkit
git submodule update --init
python3 -m venv ./python-venv
source ./python-venv/bin/activate
python3 -m pip install -r python_requirements.txt
./scripts/setup /path/to/bedrock-server-1.26.10.4
./start.sh
```

Notes utiles :

- `start.sh` lance BDS depuis la racine du devkit
- `bds-mod-mapping` écrit donc `mapping_files/` dans cette racine
- le fichier attendu est alors `./mapping_files/canonical_block_states.nbt`

## Mise À Jour Dans mc-rs

`mc-rs` ne doit plus parser `canonical_block_states.nbt` au runtime pour construire le registre de blocs.

Le workflow maintenable est :

1. extraire une nouvelle palette depuis BDS
2. copier le fichier dans `crates/mc-rs-server/data/canonical_block_states.nbt`
3. régénérer la table Rust statique

Commande :

```powershell
cargo run -p mc-rs-server --bin generate_block_registry -- `
  crates/mc-rs-server/data/canonical_block_states.nbt `
  crates/mc-rs-server/src/world/block_registry_data.rs
```

Le fichier généré `block_registry_data.rs` contient la table `nom de bloc -> premier runtime ID`.

## Politique Projet

Dans `mc-rs` :

- `canonical_block_states.nbt` reste une donnée de référence et une source de régénération
- le runtime du serveur doit utiliser la table Rust générée, pas reparcourir le NBT à chaque démarrage
- toute mise à jour de version Bedrock doit inclure :
  - une nouvelle palette BDS
  - une régénération de `block_registry_data.rs`
  - une vérification du rendu de chunks côté client
