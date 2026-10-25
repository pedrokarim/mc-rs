"""Lit un enregistrement de paquets au format de MCRS_DUMP_PACKETS=full (mc-rs) et de bds-capture.

Format : suite d'enregistrements little-endian `[u64 ms][u32 id][u32 longueur][charge utile]`.

Usage :
    python scripts/dev/packet_dump.py <fichier.bin>                  # résumé par identifiant
    python scripts/dev/packet_dump.py <fichier.bin> --id 11 --out d  # extrait les charges utiles de l'id 11

Les noms viennent des constantes de gophertunnel (`minecraft/protocol/packet/id.go`, branche amont
`origin/master` de `.reference/gophertunnel`) : l'énumération Mojang `MinecraftPacketIds.json` ne donne pas
les valeurs (son index n'est pas l'identifiant).
"""

import argparse
import re
import struct
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOPHERTUNNEL = ROOT / ".reference" / "gophertunnel"


def packet_names() -> dict[int, str]:
    """Identifiant → nom, en rejouant le bloc `const` à `iota` de gophertunnel `id.go`."""
    try:
        source = subprocess.run(
            ["git", "-C", str(GOPHERTUNNEL), "show", "origin/master:minecraft/protocol/packet/id.go"],
            capture_output=True, text=True, encoding="utf-8", check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return {}
    names, iota, offset = {}, 0, 0
    for line in source.split("const (", 1)[1].split(")", 1)[0].splitlines():
        line = line.split("//", 1)[0].strip()
        if not line:
            continue
        match = re.match(r"(\w+)(?:\s*=\s*iota\s*\+\s*(\d+))?$", line)
        if not match:
            continue
        if match.group(2):
            offset = int(match.group(2))
        if match.group(1) != "_":
            names[iota + offset] = match.group(1).removeprefix("ID")
        iota += 1
    return names


def read_records(path: Path):
    data = path.read_bytes()
    i = 0
    while i + 16 <= len(data):
        ms, pid, length = struct.unpack_from("<QII", data, i)
        i += 16
        yield ms, pid, data[i : i + length]
        i += length


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("file", type=Path)
    parser.add_argument("--id", type=int, help="identifiant de paquet à extraire")
    parser.add_argument("--out", type=Path, help="dossier de sortie des charges utiles extraites")
    args = parser.parse_args()

    names = packet_names()
    records = list(read_records(args.file))
    if args.id is None:
        counts = Counter(pid for _, pid, _ in records)
        sizes = Counter()
        for _, pid, payload in records:
            sizes[pid] += len(payload)
        print(f"{len(records)} paquets")
        for pid, n in sorted(counts.items()):
            print(f"  {pid:4d} 0x{pid:03X} {names.get(pid, '?'):40s} ×{n:5d}  {sizes[pid]:9d} octets")
        return

    selected = [(ms, payload) for ms, pid, payload in records if pid == args.id]
    print(f"{len(selected)} paquet(s) {args.id} ({names.get(args.id, '?')})")
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        for n, (ms, payload) in enumerate(selected):
            (args.out / f"{args.id}_{n:04d}_{ms}ms.bin").write_bytes(payload)


main()
