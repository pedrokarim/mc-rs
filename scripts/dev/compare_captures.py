"""Compare, champ par champ, deux captures décodées par `bds-capture decode` (JSON Lines).

Usage :
    python scripts/dev/compare_captures.py <reference.jsonl> <mc-rs.jsonl> [--packets StartGame,ItemRegistry]
        [--max 40] [--out rapport.txt]

Pour chaque type de paquet présent des deux côtés, le premier paquet de chaque capture est comparé
récursivement : clés absentes d'un côté, valeurs scalaires différentes, longueurs de listes. Les listes de
dictionnaires portant un champ `Name` (blocs, items…) sont appariées par nom, les autres par position.
Les écarts attendus (graine, position, identifiants d'entité, horloge) restent listés : c'est au lecteur de
les trier.
"""

import argparse
import json
import sys
from collections import Counter
from pathlib import Path


def first_packets(path: Path) -> tuple[dict[str, dict], Counter]:
    first: dict[str, dict] = {}
    counts: Counter = Counter()
    with path.open(encoding="utf-8") as f:
        for line in f:
            d = json.loads(line)
            counts[d["name"]] += 1
            if d["name"] not in first and "packet" in d:
                first[d["name"]] = d["packet"]
    return first, counts


def keyed(items: list) -> dict | None:
    """Indexe une liste de dictionnaires par leur champ `Name` s'il est présent et unique."""
    if not items or not all(isinstance(x, dict) and isinstance(x.get("Name"), str) for x in items):
        return None
    names = [x["Name"] for x in items]
    if len(set(names)) != len(names):
        return None
    return dict(zip(names, items))


def diff(a, b, path: str, out: list, limit: int) -> None:
    if len(out) >= limit:
        return
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a:
                out.append(f"{path}.{k} : absent de la référence, mc-rs = {short(b[k])}")
            elif k not in b:
                out.append(f"{path}.{k} : absent de mc-rs, référence = {short(a[k])}")
            else:
                diff(a[k], b[k], f"{path}.{k}", out, limit)
        return
    if isinstance(a, list) and isinstance(b, list):
        ka, kb = keyed(a), keyed(b)
        if ka is not None and kb is not None:
            only_a, only_b = sorted(set(ka) - set(kb)), sorted(set(kb) - set(ka))
            if only_a:
                out.append(f"{path} : {len(only_a)} nom(s) seulement dans la référence : {short(only_a)}")
            if only_b:
                out.append(f"{path} : {len(only_b)} nom(s) seulement dans mc-rs : {short(only_b)}")
            for name in sorted(set(ka) & set(kb)):
                diff(ka[name], kb[name], f"{path}[{name}]", out, limit)
            return
        if len(a) != len(b):
            out.append(f"{path} : longueur référence {len(a)}, mc-rs {len(b)}")
        for i, (x, y) in enumerate(zip(a, b)):
            diff(x, y, f"{path}[{i}]", out, limit)
            if len(out) >= limit:
                return
        return
    if a != b:
        out.append(f"{path} : référence = {short(a)}, mc-rs = {short(b)}")


def short(v, n: int = 160) -> str:
    s = json.dumps(v, ensure_ascii=False, sort_keys=True)
    return s if len(s) <= n else s[:n] + f"… ({len(s)} car.)"


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("reference", type=Path)
    parser.add_argument("mcrs", type=Path)
    parser.add_argument("--packets", default="", help="types de paquets à comparer (défaut : communs)")
    parser.add_argument("--max", type=int, default=40, help="écarts affichés au plus par paquet")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    ref, ref_counts = first_packets(args.reference)
    mine, mine_counts = first_packets(args.mcrs)
    lines = [f"Référence : {args.reference}", f"mc-rs     : {args.mcrs}", ""]
    only_ref = sorted(set(ref_counts) - set(mine_counts))
    only_mine = sorted(set(mine_counts) - set(ref_counts))
    lines.append(f"Paquets envoyés seulement par la référence : {', '.join(only_ref) or 'aucun'}")
    lines.append(f"Paquets envoyés seulement par mc-rs : {', '.join(only_mine) or 'aucun'}")
    lines.append("")

    wanted = [p for p in args.packets.split(",") if p] or sorted(set(ref) & set(mine))
    for name in wanted:
        if name not in ref or name not in mine:
            lines.append(f"== {name} : absent d'un côté")
            continue
        out: list[str] = []
        diff(ref[name], mine[name], name, out, args.max)
        lines.append(f"== {name} (référence ×{ref_counts[name]}, mc-rs ×{mine_counts[name]}) : "
                     f"{len(out)}{'+' if len(out) >= args.max else ''} écart(s)")
        lines.extend("  " + o for o in out)
    text = "\n".join(lines)
    print(text)
    if args.out:
        args.out.write_text(text + "\n", encoding="utf-8")


main()
