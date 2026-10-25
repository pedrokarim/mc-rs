#!/usr/bin/env python3
"""Extrait les données de génération des biomes Bedrock vanilla.

Source : .reference/bedrock-samples/behavior_pack/biomes/*.biome.json
         (packs officiels Mojang, version = bedrock-samples/version.json)
IDs     : .reference/PocketMine-MP/vendor/pocketmine/bedrock-data/biome_id_map.json

Produit : crates/mc-rs-server/data/biomes.json (superset, trié par clé).

Le fichier de sortie conserve les champs historiques (temperature, downfall,
top/mid/foundation/sea_material, tags) ET ajoute tout ce qui sert à la
génération moderne : id numérique, climat complet, règles multinoise,
noise_type, surface_builder complet, et overworld_generation_rules.

Rejouer ce script à chaque bump de version Bedrock.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
BIOMES_DIR = REPO / ".reference/bedrock-samples/behavior_pack/biomes"
ID_MAP = REPO / ".reference/PocketMine-MP/vendor/pocketmine/bedrock-data/biome_id_map.json"
OUT = REPO / "crates/mc-rs-server/data/biomes.json"


def load_id_map() -> dict[str, int]:
    return json.loads(ID_MAP.read_text(encoding="utf-8"))


def extract_one(path: Path, id_map: dict[str, int]) -> tuple[str, dict] | None:
    doc = json.loads(path.read_text(encoding="utf-8"))
    biome = doc.get("minecraft:biome")
    if biome is None:
        return None
    ident = biome["description"]["identifier"]
    short = ident.removeprefix("minecraft:")
    comp = biome["components"]

    climate = comp.get("minecraft:climate", {})
    builder = comp.get("minecraft:surface_builder", {}).get("builder", {})
    multinoise = comp.get("minecraft:multinoise_generation_rules")
    overworld_height = comp.get("minecraft:overworld_height", {})
    gen_rules = comp.get("minecraft:overworld_generation_rules")
    tags = comp.get("minecraft:tags", {}).get("tags", [])

    entry: dict = {
        # --- champs historiques (compat biomes_vanilla.rs) ---
        "temperature": climate.get("temperature", 0.0),
        "downfall": climate.get("downfall", 0.0),
        "top_material": builder.get("top_material", "minecraft:grass_block"),
        "mid_material": builder.get("mid_material", "minecraft:dirt"),
        "foundation_material": builder.get("foundation_material", "minecraft:stone"),
        "sea_material": builder.get("sea_material", "minecraft:water"),
        "tags": tags,
        # --- enrichissement génération ---
        "id": id_map.get(short),
        "snow_accumulation": climate.get("snow_accumulation"),
        "noise_type": overworld_height.get("noise_type"),
        "surface_builder": builder or None,
        "multinoise": multinoise,
        "generation_rules": gen_rules,
    }
    # Supprime les clés nulles pour garder le JSON lisible.
    entry = {k: v for k, v in entry.items() if v is not None}
    return ident, entry


def main() -> int:
    if not BIOMES_DIR.is_dir():
        print(f"ERREUR: dossier biomes introuvable: {BIOMES_DIR}", file=sys.stderr)
        return 1
    id_map = load_id_map()

    out: dict[str, dict] = {}
    missing_id: list[str] = []
    for path in sorted(BIOMES_DIR.glob("*.biome.json")):
        res = extract_one(path, id_map)
        if res is None:
            continue
        ident, entry = res
        if "id" not in entry:
            missing_id.append(ident)
        out[ident] = entry

    out = dict(sorted(out.items()))
    OUT.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print(f"OK: {len(out)} biomes -> {OUT.relative_to(REPO)}")
    n_mn = sum(1 for e in out.values() if "multinoise" in e)
    n_nt = sum(1 for e in out.values() if "noise_type" in e)
    n_gr = sum(1 for e in out.values() if "generation_rules" in e)
    print(f"     multinoise={n_mn}  noise_type={n_nt}  generation_rules={n_gr}")
    if missing_id:
        print(f"AVERTISSEMENT: {len(missing_id)} biomes sans id numérique: {missing_id}",
              file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
