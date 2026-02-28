---
layout: default
title: MC-RS Developer Docs
nav_exclude: true
---

# MC-RS Developer Documentation

**MC-RS** is a Minecraft Bedrock Edition server written in Rust, targeting protocol v924 (1.26.0).

This documentation covers the **plugin and scripting system** for developers who want to extend the server with custom plugins.

---

## Choose your language / Choisissez votre langue

<div style="display: flex; gap: 2rem; margin-top: 2rem;">
  <a href="en/" style="display: inline-block; padding: 1rem 2rem; background: #2563eb; color: white; border-radius: 8px; text-decoration: none; font-size: 1.2rem; font-weight: bold;">
    English Documentation
  </a>
  <a href="fr/" style="display: inline-block; padding: 1rem 2rem; background: #7c3aed; color: white; border-radius: 8px; text-decoration: none; font-size: 1.2rem; font-weight: bold;">
    Documentation Fran&ccedil;aise
  </a>
</div>

---

## Plugin System Overview

MC-RS supports two plugin runtimes:

| Runtime | Language | Sandboxing | Use Case |
|---------|----------|------------|----------|
| **Lua** | Lua 5.4 | Memory + instruction limits, restricted globals | Quick scripts, simple plugins |
| **WASM** | Any (Rust, C, ...) | Fuel metering + memory page limits | Performance-critical, complex plugins |

Both runtimes share the same **Plugin API** with 17 events, a task scheduler, and full server interaction capabilities.
