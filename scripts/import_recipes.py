"""Régénère `crates/mc-rs-server/data/recipes/vanilla.json` depuis bedrock-samples.

Les recettes Mojang (`behavior_pack/recipes/*.json`) sont lues directement dans
l'historique git de `.reference/bedrock-samples` au tag demandé (pas besoin de
checkout). Pour chaque fichier, le corps de l'objet `minecraft:recipe_shaped`,
`minecraft:recipe_shapeless` ou `minecraft:recipe_furnace` est ajouté à la liste
correspondante, dans l'ordre des noms de fichiers. Les autres types (brassage,
forge) sont ignorés, comme le chargeur `recipes_vanilla.rs`.

Usage :
    python scripts/import_recipes.py v1.26.40.05            # écrit vanilla.json
    python scripts/import_recipes.py v1.26.10.4 --check     # compare au fichier actuel
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SAMPLES = ROOT / ".reference" / "bedrock-samples"
OUTPUT = ROOT / "crates" / "mc-rs-server" / "data" / "recipes" / "vanilla.json"
RECIPES_DIR = "behavior_pack/recipes/"
KINDS = {
    "minecraft:recipe_shaped": "shaped",
    "minecraft:recipe_shapeless": "shapeless",
    "minecraft:recipe_furnace": "furnace",
}


def git(*args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(SAMPLES), *args],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout


def strip_comments(text: str) -> str:
    """Retire les commentaires `//` et `/* */` hors chaînes (fichiers JSON5 de Mojang)."""
    text = text.lstrip("\ufeff")
    return re.sub(
        r'("(?:\\.|[^"\\])*")|//[^\n]*|/\*.*?\*/',
        lambda m: m.group(1) or "",
        text,
        flags=re.S,
    )


def build(ref: str) -> tuple[dict, list[str]]:
    names = sorted(
        n for n in git("ls-tree", "--name-only", ref, RECIPES_DIR).splitlines() if n.endswith(".json")
    )
    out = {kind: [] for kind in KINDS.values()}
    skipped = []
    for name in names:
        doc = json.loads(strip_comments(git("show", f"{ref}:{name}")))
        body = next(((KINDS[k], v) for k, v in doc.items() if k in KINDS), None)
        if body is None:
            skipped.append(name.rsplit("/", 1)[-1])
            continue
        out[body[0]].append(body[1])
    return out, skipped


def main() -> None:
    # La console Windows n'est pas en UTF-8 par défaut.
    sys.stdout.reconfigure(encoding="utf-8")
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    ref = sys.argv[1]
    data, skipped = build(ref)
    counts = ", ".join(f"{k} {len(v)}" for k, v in data.items())
    print(f"{ref} : {counts} ; {len(skipped)} fichiers d'autres types ignorés")

    if "--check" in sys.argv[2:]:
        current = json.loads(OUTPUT.read_text(encoding="utf-8"))
        same = current == data
        print("identique au fichier actuel" if same else "DIFFÉRENT du fichier actuel")
        if not same:
            for kind in KINDS.values():
                a = {json.dumps(r, sort_keys=True) for r in current.get(kind, [])}
                b = {json.dumps(r, sort_keys=True) for r in data[kind]}
                print(f"  {kind} : {len(b - a)} ajoutées, {len(a - b)} retirées")
        sys.exit(0 if same else 1)

    OUTPUT.write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"écrit {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
