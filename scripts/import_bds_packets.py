"""Extrait de la capture de BDS des paquets de pure donnée, envoyés tels quels par mc-rs.

Certains paquets ne contiennent que des définitions du jeu, sans identifiant propre au serveur : le client
1.26.50+ les exige (JigsawStructureData : sans lui, déconnexion `MissingStructureData`, écran « Boat »).
mc-rs les renvoie octet pour octet. Ce script copie la charge utile du premier paquet de chaque type dans
`crates/mc-rs-server/data/bds_packets/<nom>.bin` et écrit `manifest.json` (provenance, taille, SHA-256).

Usage :
    python scripts/import_bds_packets.py <capture>/s2c.bin --version 1.26.52.3

Méthode et capture de référence : dev-docs/34-BDS-RETRO-INGENIERIE.md.
"""

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "crates" / "mc-rs-server" / "data" / "bds_packets"

# Identifiant (gophertunnel `packet/id.go`) → nom du fichier.
PACKETS = {
    313: "jigsaw_structure_data",
    337: "voxel_shapes",
}


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("capture", type=Path, help="s2c.bin produit par bds-capture capture")
    parser.add_argument("--version", required=True, help="version de BDS capturée, ex. 1.26.52.3")
    args = parser.parse_args()

    data = args.capture.read_bytes()
    found: dict[int, bytes] = {}
    i = 0
    while i + 16 <= len(data):
        _, pid, length = struct.unpack_from("<QII", data, i)
        i += 16
        if pid in PACKETS and pid not in found:
            found[pid] = data[i : i + length]
        i += length
    missing = sorted(set(PACKETS) - set(found))
    if missing:
        sys.exit(f"paquets absents de la capture : {missing}")

    OUTPUT.mkdir(parents=True, exist_ok=True)
    manifest = {
        "bds_version": args.version,
        "capture": args.capture.as_posix().split(".reference/", 1)[-1],
        "generated_by": "scripts/import_bds_packets.py",
        "packets": {},
    }
    for pid, name in PACKETS.items():
        payload = found[pid]
        (OUTPUT / f"{name}.bin").write_bytes(payload)
        manifest["packets"][name] = {
            "id": pid,
            "size": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        }
        print(f"{name} (id {pid}) : {len(payload)} octets")
    (OUTPUT / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=1) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"écrit {OUTPUT}")


main()
