"""Biomes des chunks reçus dans une capture décodée (`bds-capture decode`, paquets LevelChunk).

Chaque LevelChunk porte, à la suite des sous-chunks éventuels, une section de biomes par tranche de 16 blocs de
hauteur (24 dans l'Overworld, de y = -64 à 319) : stockage palettisé de 4 096 cases (16 × 16 × 16, index
`(x << 8) | (z << 4) | y`), en-tête `(bits << 1) | 1`, mots u32 little-endian, taille de palette puis
identifiants en VarInt32 zigzag ; `0xFF` répète la section précédente. Référence : dragonfly
`server/world/chunk/decode.go` (`decodePalettedStorage`), et `chunk_serializer.rs` de mc-rs, qui écrit le même
format. Les sous-chunks qui précèdent les biomes (`SubChunkCount` > 0, cas de mc-rs ; BDS envoie 0 et les blocs
à la demande) sont sautés : version 8 ou 9, nombre de couches, indice y en version 9, puis un stockage
palettisé de 4 096 cases par couche.

Usage :
    python scripts/dev/chunk_biomes.py <s2c.jsonl> [--y 64] [--json carte.json] [--ascii]

Sortie : nombre de colonnes par biome à la hauteur y, et, avec --json, la carte {"cx,cz": [256 biomes]}
(colonnes x puis z).
"""

import argparse
import base64
import collections
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BIOMES = ROOT / "crates" / "mc-rs-server" / "data" / "biomes.json"
MIN_Y = -64


class Reader:
    def __init__(self, data: bytes):
        self.data, self.pos = data, 0

    def u8(self) -> int:
        self.pos += 1
        return self.data[self.pos - 1]

    def u32(self) -> int:
        v = struct.unpack_from("<I", self.data, self.pos)[0]
        self.pos += 4
        return v

    def varint(self) -> int:
        shift = n = 0
        while True:
            b = self.u8()
            n |= (b & 0x7F) << shift
            if not b & 0x80:
                return (n >> 1) ^ -(n & 1)
            shift += 7


def skip_storage(r: Reader) -> None:
    bits = r.u8() >> 1
    if bits == 0:
        r.varint()
        return
    per_word = 32 // bits
    r.pos += 4 * ((4096 + per_word - 1) // per_word)
    for _ in range(r.varint()):
        r.varint()


def biome_sections(payload: bytes, subchunks: int = 0, count: int = 24) -> list[list[int]]:
    """Sections de biomes (4 096 identifiants chacune) qui suivent les `subchunks` sous-chunks du payload."""
    r = Reader(payload)
    for _ in range(subchunks):
        version = r.u8()
        layers = r.u8()
        if version == 9:
            r.u8()
        for _ in range(layers):
            skip_storage(r)
    sections = []
    while len(sections) < count:
        header = r.u8()
        if header == 0xFF:
            sections.append(sections[-1])
            continue
        bits = header >> 1
        if bits == 0:
            sections.append([r.varint()] * 4096)
            continue
        per_word = 32 // bits
        words = [r.u32() for _ in range((4096 + per_word - 1) // per_word)]
        palette = [r.varint() for _ in range(r.varint())]
        mask = (1 << bits) - 1
        sections.append([palette[(words[i // per_word] >> ((i % per_word) * bits)) & mask] for i in range(4096)])
    return sections


def column_biomes(sections: list[list[int]], y: int) -> list[int]:
    """Biome de chaque colonne (x puis z) à la hauteur y."""
    section = sections[(y - MIN_Y) // 16]
    ly = (y - MIN_Y) % 16
    return [section[(x << 8) | (z << 4) | ly] for x in range(16) for z in range(16)]


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("capture", type=Path)
    parser.add_argument("--y", type=int, default=64)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--ascii", action="store_true", help="carte des biomes dominants par chunk")
    args = parser.parse_args()

    names = {v["id"]: k.removeprefix("minecraft:") for k, v in json.loads(BIOMES.read_text(encoding="utf-8")).items()}
    chunks = {}
    with args.capture.open(encoding="utf-8") as f:
        for line in f:
            row = json.loads(line)
            if row.get("name") != "LevelChunk":
                continue
            pk = row["packet"]
            subchunks = pk.get("SubChunkCount", 0)
            if pk.get("Dimension", 0) != 0 or subchunks < 0 or pk.get("CacheEnabled"):
                continue
            cx, cz = pk["Position"]
            sections = biome_sections(base64.b64decode(pk["RawPayload"]), subchunks)
            chunks[(cx, cz)] = column_biomes(sections, args.y)

    counts = collections.Counter(b for cols in chunks.values() for b in cols)
    print(f"{len(chunks)} chunks, y = {args.y}")
    for biome, n in counts.most_common():
        print(f"  {names.get(biome, biome)} ({biome}) : {n} colonnes")
    if args.ascii and chunks:
        letters = {b: chr(ord("A") + i) for i, (b, _) in enumerate(counts.most_common())}
        xs = sorted({c[0] for c in chunks})
        zs = sorted({c[1] for c in chunks})
        for cz in range(zs[0], zs[-1] + 1):
            print("".join(
                letters[collections.Counter(chunks[(cx, cz)]).most_common(1)[0][0]] if (cx, cz) in chunks else " "
                for cx in range(xs[0], xs[-1] + 1)
            ))
        print("  ".join(f"{l}={names.get(b, b)}" for b, l in letters.items()))
    if args.json:
        args.json.write_text(json.dumps({f"{cx},{cz}": cols for (cx, cz), cols in chunks.items()}) + "\n",
                             encoding="utf-8")
        print(f"écrit {args.json}")


main()
