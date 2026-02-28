---
layout: default
title: Accueil
nav_order: 1
---

# Développement de Plugins MC-RS

MC-RS est un serveur Minecraft Bedrock Edition écrit en Rust, ciblant le **protocole v924** (1.26.0). Il dispose d'un système de plugins modulaire supportant les **scripts Lua** et les **modules WASM**.

## Architecture en bref

Le serveur est organisé en workspace Rust avec 11 crates :

| Crate | Rôle |
|-------|------|
| `mc-rs-server` | Point d'entrée, orchestration, configuration |
| `mc-rs-proto` | Définitions de paquets, codec, sérialisation |
| `mc-rs-raknet` | Transport RakNet UDP (fiabilité, fragmentation) |
| `mc-rs-crypto` | ECDH P-384, AES-256-CFB8, JWT |
| `mc-rs-nbt` | NBT little-endian + variante réseau |
| `mc-rs-world` | Chunks, registre de blocs, génération de monde |
| `mc-rs-game` | Logique de jeu (combat, nourriture, crafting, XP) |
| `mc-rs-command` | Framework de commandes |
| `mc-rs-plugin-api` | Interfaces de plugins (types, traits, événements) |
| `mc-rs-plugin-lua` | Runtime de scripting Lua 5.4 |
| `mc-rs-plugin-wasm` | Runtime de plugins WASM (wasmtime) |

## Runtimes de plugins

| Runtime | Langage | Sandboxing | Idéal pour |
|---------|---------|------------|------------|
| **Lua** | Lua 5.4 | Limites mémoire + instructions, `os`/`io`/`debug` supprimés | Scripts rapides, plugins simples |
| **WASM** | Tout → Wasm (Rust, C, ...) | Fuel metering + limites mémoire | Plugins complexes, haute performance |

Les deux runtimes partagent la même [API Plugin](plugin-api) avec 17 événements, un planificateur de tâches et une interaction complète avec le serveur.

## Documentation

- [Démarrage rapide](getting-started) — Créez votre premier plugin en 5 minutes
- [Architecture](architecture) — Comment fonctionne le système de plugins
- [Référence ServerApi](plugin-api) — Toutes les fonctions et types disponibles
- [Référence des événements](events) — Liste complète des 17 événements
- [Scripting Lua](lua-scripting) — Guide et référence de l'API `mc.*`
- [Plugins WASM](wasm-plugins) — Guide de développement WASM, exports et fonctions hôte
- [Exemples](examples) — Exemples complets de plugins fonctionnels

[English](../en/){: .btn }
