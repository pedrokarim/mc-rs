# MC-RS

Un serveur Minecraft Bedrock Edition écrit en Rust.

> Franchement rien d'ambitieux. On cherche pas du tout à changer le monde ici. Juste un petit serveur Minecraft réécrit from scratch en Rust avec 11 crates, de la crypto, du RakNet, de la compression, du WASM... pour passer le temps.

## Présentation

MC-RS cible le **protocole Bedrock v766** (1.21.50). Rien de fou, on veut juste un serveur haute performance, modulaire et extensible, avec un système de plugins WASM et Lua. Un truc tranquille quoi.

## Workspace

Un petit workspace de rien du tout :

| Crate | Description |
|-------|-------------|
| `mc-rs-server` | Point d'entrée, orchestration, configuration |
| `mc-rs-proto` | Définitions de paquets, codec, sérialisation |
| `mc-rs-raknet` | Transport RakNet (UDP, fiabilité, fragmentation) |
| `mc-rs-crypto` | ECDH P-384, AES-256-CFB8, JWT |
| `mc-rs-nbt` | NBT little-endian + variante réseau |
| `mc-rs-world` | Chunks, registre de blocs, génération de monde |
| `mc-rs-game` | Logique de jeu |
| `mc-rs-command` | Framework de commandes |
| `mc-rs-plugin-api` | Interfaces de plugins |
| `mc-rs-plugin-lua` | Runtime de scripting Lua |
| `mc-rs-plugin-wasm` | Runtime de plugins WASM |

## Compilation

```bash
cargo build
cargo test
```

## Licence

TBD
