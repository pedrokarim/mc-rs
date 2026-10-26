"""Construit `crates/mc-rs-server/data/block_liquid.json` : propriétés liquides de BDS, par état de la palette.

Source : le dump de BDS d'Allay (`.reference/Allay/data/resources/block_states.json`), dont les états suivent
l'ordre de la palette de mc-rs (vérifié ici par le hash réseau de chaque état, `bds_palette.py`).
- `canContainLiquidSource` : l'état peut contenir une source d'eau (waterlogging), rangée par Bedrock dans la
  seconde couche du sous-chunk ;
- `liquidReactionOnTouch` : ce que fait un liquide qui coule sur l'état (`BLOCKING` arrêté, `POPPED` bloc cassé
  avec ses drops, `BROKEN` bloc détruit, `NOREACTION` liquide qui traverse) ;
- `isSolid` : bloc solide (une nouvelle source d'eau ne se forme que sur un bloc solide ou une source).

Format : runtime IDs en intervalles `[premier, dernier]` (inclus), `BLOCKING` par défaut.

Usage :
    python scripts/dev/build_block_liquid.py
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bds_palette import ALLAY_STATES, load_palette  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates" / "mc-rs-server" / "data" / "block_liquid.json"


def ranges(ids: list[int]) -> list[list[int]]:
    out: list[list[int]] = []
    for i in sorted(ids):
        if out and out[-1][1] == i - 1:
            out[-1][1] = i
        else:
            out.append([i, i])
    return out


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    palette = load_palette()
    allay = json.loads(ALLAY_STATES.read_text(encoding="utf-8"))
    if len(allay) != len(palette):
        sys.exit(f"palette : {len(palette)} états, dump d'Allay : {len(allay)}")
    for i, (p, a) in enumerate(zip(palette, allay)):
        if p["name"] != a["name"] or p["hash"] != a["blockStateHash"]:
            sys.exit(f"état {i} différent : {p['command']} ≠ {a['name']} (hash {a['blockStateHash']})")

    reactions: dict[str, list[int]] = {}
    for i, a in enumerate(allay):
        if a["liquidReactionOnTouch"] != "BLOCKING":
            reactions.setdefault(a["liquidReactionOnTouch"], []).append(i)
    data = {
        "source": "BDS, dump d'Allay (data/resources/block_states.json), scripts/dev/build_block_liquid.py",
        "state_count": len(palette),
        "can_contain_liquid_source": ranges([i for i, a in enumerate(allay) if a["canContainLiquidSource"]]),
        "solid": ranges([i for i, a in enumerate(allay) if a["isSolid"]]),
        "liquid_reaction_on_touch": {k: ranges(v) for k, v in sorted(reactions.items())},
    }
    OUT.write_text(json.dumps(data, separators=(",", ":")) + "\n", encoding="utf-8")
    can = sum(b - a + 1 for a, b in data["can_contain_liquid_source"])
    print(f"{OUT} : {can} états peuvent contenir de l'eau, réactions {{"
          + ", ".join(f"{k}: {sum(b - a + 1 for a, b in v)}" for k, v in data["liquid_reaction_on_touch"].items())
          + "}")


main()
