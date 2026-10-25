"""Importe les garnitures d'armure (TrimData) de BDS depuis une capture décodée (`bds-capture decode`).

Écrit `crates/mc-rs-server/data/trim_data.json` : motifs (gabarit de forge → identifiant) et matériaux
(identifiant, code couleur, item), dans l'ordre où BDS les envoie.

Usage :
    python scripts/import_bds_trim_data.py <capture>/s2c.typed.jsonl --version 1.26.52.3
    python scripts/import_bds_trim_data.py <capture>/s2c.typed.jsonl --version 1.26.52.3 --check

Méthode et capture de référence : dev-docs/34-BDS-RETRO-INGENIERIE.md.
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "crates" / "mc-rs-server" / "data" / "trim_data.json"


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("capture", type=Path, help="s2c.jsonl ou s2c.typed.jsonl produit par bds-capture decode")
    parser.add_argument("--version", required=True, help="version de BDS capturée, ex. 1.26.52.3")
    parser.add_argument("--check", action="store_true", help="compare au fichier en place sans l'écrire")
    args = parser.parse_args()

    packet = None
    with args.capture.open(encoding="utf-8") as f:
        for line in f:
            d = json.loads(line)
            if d["name"] == "TrimData":
                if d.get("error") or d.get("unread"):
                    sys.exit("TrimData mal décodé dans la capture")
                packet = d["packet"]
                break
    if packet is None:
        sys.exit(f"TrimData absent de la capture {args.capture}")

    data = {
        "source": {
            "bds_version": args.version,
            "capture": args.capture.as_posix().split(".reference/", 1)[-1],
            "generated_by": "scripts/import_bds_trim_data.py",
        },
        "patterns": [{"item": p["ItemName"], "pattern": p["PatternID"]} for p in packet["Patterns"]],
        "materials": [
            {"material": m["MaterialID"], "colour": m["Colour"], "item": m["ItemName"]} for m in packet["Materials"]
        ],
    }
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    print(f"BDS {args.version} : {len(data['patterns'])} motifs, {len(data['materials'])} matériaux")
    if args.check:
        same = OUTPUT.exists() and OUTPUT.read_text(encoding="utf-8") == text
        print("identique au fichier en place" if same else "DIFFÉRENT du fichier en place")
        sys.exit(0 if same else 1)
    OUTPUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"écrit {OUTPUT}")


main()
