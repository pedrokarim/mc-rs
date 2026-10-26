"""Résume un relevé de drops de BDS (`bds-capture drops`) et le compare aux données de mc-rs.

Pour chaque bloc et chaque outil : résultats observés (multiensemble d'items par tirage) et fréquences, bloc
déterministe ou aléatoire, et écart avec `crates/mc-rs-server/data/block_break.json` (drops calculés par PMMP
pour l'état par défaut du bloc). Les tirages dont le bloc n'a pas été posé (`placed` faux) sont écartés.

Usage :
    python scripts/dev/summarize_bds_drops.py <relevé.jsonl> [--json bilan.json] [--diff]

Méthode : dev-docs/36-CASSAGE-DE-BLOCS.md.
"""

import argparse
import collections
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BREAK_DATA = ROOT / "crates" / "mc-rs-server" / "data" / "block_break.json"
REGISTRY = ROOT / "crates" / "mc-rs-server" / "src" / "world" / "block_registry_data.rs"

# Outil du relevé → scénario de block_break.json (main nue, famille, Toucher de soie). Fortune n'a pas de
# scénario fixe : les blocs concernés sont aléatoires.
TOOL_SCENARIO = {
    "": "hand",
    "wooden_pickaxe": "pickaxe", "stone_pickaxe": "pickaxe", "iron_pickaxe": "pickaxe",
    "diamond_pickaxe": "pickaxe", "netherite_pickaxe": "pickaxe",
    "wooden_axe": "axe", "wooden_shovel": "shovel", "wooden_hoe": "hoe", "wooden_sword": "sword",
    "shears": "shears",
}


def tool_key(tool: dict) -> str:
    name = tool.get("tool", "")
    enchants = ",".join(f"{k}{v}" for k, v in sorted(tool.get("enchants", {}).items()))
    return f"{name}+{enchants}" if enchants else (name or "main")


def outcome(items: list) -> tuple:
    counts = collections.Counter()
    for item in items:
        counts[item["item"]] += item["count"]
    return tuple(sorted(counts.items()))


def first_runtime_ids() -> dict:
    ids = {}
    for line in REGISTRY.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith('("minecraft:'):
            name, rid = line.strip("(),").split('", ')
            ids[name.strip('"')] = int(rid)
    return ids


def mcrs_outcome(profile: dict, scenario: str):
    """Drops fixes de mc-rs pour un scénario, ou None si la règle n'est pas une liste fixe."""
    drops = (profile.get("drops") or {}).get(scenario)
    if not isinstance(drops, list):
        return None
    counts = collections.Counter()
    for d in drops:
        counts[d["item"]] += d.get("count", 1)
    return tuple(sorted(counts.items()))


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("survey", type=Path)
    parser.add_argument("--json", type=Path, help="écrit le bilan complet")
    parser.add_argument("--diff", action="store_true", help="liste les écarts déterministes avec mc-rs")
    args = parser.parse_args()

    samples = collections.defaultdict(list)
    unplaced = collections.Counter()
    with args.survey.open(encoding="utf-8") as f:
        for line in f:
            row = json.loads(line)
            block = row["block"] if row["block"].startswith("minecraft:") else "minecraft:" + row["block"]
            if not row["placed"]:
                unplaced[block] += 1
                continue
            samples[(block, tool_key(row["tool"]), row["tool"].get("tool", ""), bool(row["tool"].get("enchants")))].append(
                outcome(row["items"])
            )

    summary = {}
    for (block, key, tool, enchanted), outs in sorted(samples.items()):
        freq = collections.Counter(outs)
        summary.setdefault(block, {})[key] = {
            "samples": len(outs),
            "deterministic": len(freq) == 1,
            "outcomes": [{"items": [list(i) for i in o], "count": n} for o, n in freq.most_common()],
        }

    random_blocks = sorted(b for b, tools in summary.items() if any(not t["deterministic"] for t in tools.values()))
    never_placed = sorted(b for b in unplaced if b not in summary)
    print(f"{len(summary)} blocs observés, {len(random_blocks)} aléatoires, {len(never_placed)} jamais posés")
    if never_placed:
        print("jamais posés :", " ".join(b.removeprefix("minecraft:") for b in never_placed))

    if args.diff:
        data = json.loads(BREAK_DATA.read_text(encoding="utf-8"))
        first = first_runtime_ids()
        diffs = []
        for (block, key, tool, enchanted), outs in sorted(samples.items()):
            if enchanted or tool not in TOOL_SCENARIO or len(set(outs)) != 1 or block not in first:
                continue
            profile = data["profiles"][data["states"][first[block]]]
            # Même choix que `BreakProfile::drops` : famille de l'outil s'il est compatible, sinon
            # scénario « incompatible » (PMMP `BlockBreakInfo::isToolCompatible`).
            info = data["tools"].get("minecraft:" + tool, {"tool_type": 0, "harvest_level": 0})
            compatible = profile["hardness"] >= 0 and (
                profile["tool_type"] == 0
                or profile["harvest_level"] == 0
                or (profile["tool_type"] & info["tool_type"] and info["harvest_level"] >= profile["harvest_level"])
            )
            expected = mcrs_outcome(profile, TOOL_SCENARIO[tool] if compatible else "incompatible")
            if profile["source"] != "pmmp":
                expected = "pas de données de drops"
            elif expected is None:
                continue
            if expected != outs[0]:
                diffs.append((block, key, outs[0], expected))
        print(f"{len(diffs)} écarts déterministes (état par défaut)")
        for block, key, got, expected in diffs:
            print(f"  {block.removeprefix('minecraft:')} [{key}] BDS={got} mc-rs={expected}")

    if args.json:
        args.json.write_text(
            json.dumps({"survey": str(args.survey), "unplaced": never_placed, "blocks": summary}, indent=1) + "\n",
            encoding="utf-8",
        )
        print(f"écrit {args.json}")


main()
