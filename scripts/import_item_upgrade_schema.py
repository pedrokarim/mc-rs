"""Consolide les schémas de migration d'items de PocketMine-MP.

Lit les fichiers `NNNN_*.json` de `pmmp/BedrockItemUpgradeSchema`
(`id_meta_upgrade_schema/`) et écrit
`crates/mc-rs-server/data/item_id_upgrade_schema.json` : liste des schémas triée
par numéro (préfixe à 4 chiffres, comme `ItemIdMetaUpgradeSchemaUtils::loadSchemas`),
chacun avec ses `renamedIds` et `remappedMetas`, plus le nom du fichier source.

Usage :
    python scripts/import_item_upgrade_schema.py [dossier_id_meta_upgrade_schema]
Défaut : `.reference/BedrockItemUpgradeSchema/id_meta_upgrade_schema`.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SOURCE = ROOT / ".reference" / "BedrockItemUpgradeSchema" / "id_meta_upgrade_schema"
OUTPUT = ROOT / "crates" / "mc-rs-server" / "data" / "item_id_upgrade_schema.json"
FILE_PATTERN = re.compile(r"^(\d{4}).*\.json$")


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    source = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SOURCE
    schemas = []
    for path in source.iterdir():
        match = FILE_PATTERN.match(path.name)
        if not match:
            continue
        raw = json.loads(path.read_text(encoding="utf-8"))
        schemas.append(
            {
                "id": int(match.group(1)),
                "source": path.name,
                "renamedIds": raw.get("renamedIds", {}),
                "remappedMetas": raw.get("remappedMetas", {}),
            }
        )
    schemas.sort(key=lambda s: s["id"])
    OUTPUT.write_text(json.dumps(schemas, ensure_ascii=False, indent=1), encoding="utf-8")
    renamed = sum(len(s["renamedIds"]) for s in schemas)
    remapped = sum(len(m) for s in schemas for m in s["remappedMetas"].values())
    print(
        f"{len(schemas)} schémas (jusqu'à {schemas[-1]['source']}) : "
        f"{renamed} renommages, {remapped} remappages par meta → {OUTPUT.relative_to(ROOT)}"
    )


if __name__ == "__main__":
    main()
