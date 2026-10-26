"""Construit `crates/mc-rs-server/data/block_drops_bds.json` : drops observés sur BDS, par état de la palette.

Entrées : les relevés de `bds-capture drops` faits sur le plan de `plan_bds_drops.py` (états représentants),
et la correspondance état → représentant écrite avec ce plan. Pour chaque état de la palette, la table de son
représentant : pour chaque outil relevé, les résultats observés et leur nombre de tirages (un seul résultat
pour un bloc déterministe ; plusieurs, pondérés, pour un bloc aléatoire). Les items blocs sont rattachés à
leur runtime ID par le hash réseau de BDS (`bds_palette.py`).

Restent sans table (repli sur `data/block_break.json`, PMMP) : les états jamais posés (noms refusés par
`setblock`, blocs techniques) et ceux dont PMMP lit les drops dans la tuile (lit, bannières, boîtes de
Shulker, pot de fleurs, cadres, pupitre), car `/loot` ne voit que la tuile par défaut d'un bloc posé par
commande.

Usage :
    python scripts/dev/build_bds_drops.py --groups <plan>.groups.json <relevé.jsonl> [<relevé.jsonl> ...]

Méthode : dev-docs/36-CASSAGE-DE-BLOCS.md.
"""

import argparse
import collections
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bds_palette import load_palette  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
BREAK_DATA = ROOT / "crates" / "mc-rs-server" / "data" / "block_break.json"
OUT = ROOT / "crates" / "mc-rs-server" / "data" / "block_drops_bds.json"
BDS_VERSION = "1.26.52.3"


def tool_key(tool: dict) -> str:
    """Clé d'outil, reprise telle quelle par `block_break.rs` : `main`, `iron_pickaxe`,
    `netherite_pickaxe+fortune3`…"""
    name = tool.get("tool", "") or "main"
    enchants = "".join(f"+{k}{v}" for k, v in sorted(tool.get("enchants", {}).items()))
    return name + enchants


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("--groups", type=Path, required=True)
    parser.add_argument("surveys", type=Path, nargs="+")
    args = parser.parse_args()

    palette = load_palette()
    by_hash = {state["hash"]: rid for rid, state in enumerate(palette)}
    representative = json.loads(args.groups.read_text(encoding="utf-8"))["representative"]
    if len(representative) != len(palette):
        sys.exit("correspondance des groupes périmée : relancer plan_bds_drops.py")
    command_rid = {palette[rid]["command"]: rid for rid in set(representative)}

    # Tirages valides (bloc en place au tick du tirage), par représentant et par outil.
    samples = collections.defaultdict(collections.Counter)
    tools = set()
    for survey in args.surveys:
        with survey.open(encoding="utf-8") as f:
            for line in f:
                row = json.loads(line)
                rid = command_rid.get(row["block"])
                if rid is None or not row["placed"]:
                    continue
                items = []
                for item in sorted(row["items"], key=lambda i: (i["item"], i.get("meta", 0), i.get("block_hash", 0))):
                    drop = {"item": item["item"], "count": item["count"]}
                    if item.get("meta"):
                        drop["meta"] = item["meta"]
                    if item.get("block_hash"):
                        drop["block"] = by_hash[item["block_hash"] & 0xFFFFFFFF]
                    items.append(drop)
                key = tool_key(row["tool"])
                tools.add(key)
                samples[(rid, key)][json.dumps(items, sort_keys=True)] += 1

    # PMMP lit dans la tuile les drops de ces états : on garde son modèle.
    pmmp = json.loads(BREAK_DATA.read_text(encoding="utf-8"))
    from_tile = {
        rid for rid, profile_index in enumerate(pmmp["states"])
        if any(rule == "world" for rule in (pmmp["profiles"][profile_index].get("drops") or {}).values())
    }

    tables, table_index, states = [], {}, []
    for rid in range(len(palette)):
        rep = representative[rid]
        table = {}
        for key in sorted(tools):
            outcomes = samples.get((rep, key))
            if outcomes:
                table[key] = [
                    {"weight": n, "items": json.loads(items)}
                    for items, n in sorted(outcomes.items(), key=lambda o: (-o[1], o[0]))
                ]
        if not table or rid in from_tile:
            states.append(-1)
            continue
        encoded = json.dumps(table, sort_keys=True)
        if encoded not in table_index:
            table_index[encoded] = len(tables)
            tables.append(table)
        states.append(table_index[encoded])

    covered = sum(1 for s in states if s >= 0)
    random_tables = sum(1 for t in tables if any(len(o) > 1 for o in t.values()))
    data = {
        "source": {
            "bds": BDS_VERSION,
            "method": "bds-capture drops (loot spawn … mine), scripts/dev/build_bds_drops.py",
            "surveys": [str(s.resolve().relative_to(ROOT)).replace("\\", "/") for s in args.surveys],
        },
        "tools": sorted(tools),
        "tables": tables,
        "states": states,
    }
    OUT.write_text(json.dumps(data, separators=(",", ":"), sort_keys=False) + "\n", encoding="utf-8")
    names_tile = sorted({palette[rid]["name"] for rid in from_tile})
    print(f"{covered} états sur {len(palette)} avec drops de BDS, {len(tables)} tables dont {random_tables} aléatoires")
    print(f"drops lus dans la tuile (gardés de PMMP) : {' '.join(n.removeprefix('minecraft:') for n in names_tile)}")
    missing = sorted({palette[rid]["name"] for rid in range(len(palette)) if states[rid] < 0} - set(names_tile))
    print(f"blocs dont un état au moins reste sans table ({len(missing)}) : {' '.join(n.removeprefix('minecraft:') for n in missing)}")
    print(f"écrit {OUT.relative_to(ROOT)} ({OUT.stat().st_size // 1024} Ko)")


main()
