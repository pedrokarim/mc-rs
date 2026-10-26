"""Classes de blocs (solide, eau, lave, air) de chunks d'un monde Bedrock enregistré (LevelDB), et comparaison.

Lit la base d'un monde (BDS : `worlds/<monde>/db`) avec `amulet-leveldb` (module `leveldb`) et `amulet-nbt`.
Travailler sur une COPIE de la base : l'ouvrir pendant que BDS tourne, ou l'écrire, peut l'abîmer.
Format d'un sous-chunk : clé = x et z en i32 little-endian, octet 47 (`SubChunkPrefix`), indice y (signé) ;
valeur = version (8 ou 9), nombre de couches, indice y en version 9, puis par couche un stockage palettisé de
4 096 cases (index `(x << 8) | (z << 4) | y`, mots u32 little-endian) suivi d'une palette de composés NBT
little-endian (`name`, `states`). Seule la première couche compte (la seconde porte l'eau des blocs gorgés).

Sortie au format du test ignoré `noise_chunk::tests::dump_chunk_classes` de mc-rs :
`{"cx,cz": [256 colonnes (x puis z) de 144 caractères, y = -64 à 79]}`, `#` solide, `~` eau, `L` lave, `.` air.

Usage :
    python scripts/dev/world_block_classes.py extract <db> <sortie.json> [--chunks "cx,cz cx,cz…"]
    python scripts/dev/world_block_classes.py compare <référence.json> <autre.json>
Sans --chunks, extrait tous les chunks enregistrés.
"""

import argparse
import collections
import json
import struct
import sys

MIN_Y = -64
TOP_Y = 80
SUB_CHUNK_PREFIX = 47


def read_storage(data: bytes, pos: int) -> tuple[list[str], int]:
    """Un stockage palettisé de 4 096 cases ; renvoie les noms de blocs et la position suivante."""
    import amulet_nbt

    bits = data[pos] >> 1
    pos += 1
    if bits == 0:
        indices = [0] * 4096
        size = 1
    else:
        per_word = 32 // bits
        count = (4096 + per_word - 1) // per_word
        words = struct.unpack_from(f"<{count}I", data, pos)
        pos += 4 * count
        mask = (1 << bits) - 1
        indices = [(words[i // per_word] >> ((i % per_word) * bits)) & mask for i in range(4096)]
        size = struct.unpack_from("<i", data, pos)[0]
        pos += 4
    palette = []
    for _ in range(size):
        context = amulet_nbt.ReadContext()
        tag = amulet_nbt.load(data[pos:], compressed=False, little_endian=True, read_context=context)
        pos += context.offset
        palette.append(str(tag.compound["name"].py_str))
    return [palette[i] for i in indices], pos


def classify(name: str) -> str:
    name = name.removeprefix("minecraft:")
    if name in ("air", "cave_air", "void_air"):
        return "."
    if name in ("water", "flowing_water"):
        return "~"
    if name in ("lava", "flowing_lava"):
        return "L"
    return "#"


def extract(db_path: str, out: str, chunks: list[tuple[int, int]] | None) -> None:
    from leveldb import LevelDB

    db = LevelDB(db_path, create_if_missing=False)
    if chunks is None:
        chunks = sorted({struct.unpack("<ii", key[:8]) for key, _ in db.iterate()
                         if len(key) == 10 and key[8] == SUB_CHUNK_PREFIX})
    height = TOP_Y - MIN_Y
    result = {}
    for cx, cz in chunks:
        columns = [["?"] * height for _ in range(256)]
        for section in range(MIN_Y >> 4, (TOP_Y + 15) >> 4):
            key = struct.pack("<ii", cx, cz) + bytes([SUB_CHUNK_PREFIX, section & 0xFF])
            try:
                data = db.get(key)
            except KeyError:
                continue
            version, layers = data[0], data[1]
            pos = 3 if version == 9 else 2
            if layers == 0:
                blocks = ["minecraft:air"] * 4096
            else:
                blocks, pos = read_storage(data, pos)
            for i, name in enumerate(blocks):
                y = section * 16 + (i & 15)
                if MIN_Y <= y < TOP_Y:
                    columns[(i >> 8) * 16 + ((i >> 4) & 15)][y - MIN_Y] = classify(name)
        result[f"{cx},{cz}"] = ["".join(column) for column in columns]
    with open(out, "w", encoding="utf-8") as f:
        json.dump(result, f)
    missing = sum(column.count("?") for columns in result.values() for column in columns)
    print(f"{len(result)} chunks écrits dans {out} ({missing} cases absentes de la base)")


def compare(reference: str, other: str) -> None:
    ref = json.load(open(reference, encoding="utf-8"))
    oth = json.load(open(other, encoding="utf-8"))
    pairs = collections.Counter()
    for key in ref.keys() & oth.keys():
        for a_column, b_column in zip(ref[key], oth[key]):
            pairs.update(zip(a_column, b_column))
    pairs = {k: v for k, v in pairs.items() if "?" not in k}
    total = sum(pairs.values())
    same = sum(v for (a, b), v in pairs.items() if a == b)
    voids = {k: v for k, v in pairs.items() if k[0] in ".~" and k[1] in ".~"}
    voids_same = voids.get((".", "."), 0) + voids.get(("~", "~"), 0)
    print(f"classes identiques : {100 * same / total:.2f} % de {total} blocs")
    print(f"vides communs : eau/air d'accord à {100 * voids_same / max(1, sum(voids.values())):.2f} %")
    print("paires (référence, autre) :", sorted(("".join(k), v) for k, v in pairs.items()))


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    p_extract = sub.add_parser("extract")
    p_extract.add_argument("db")
    p_extract.add_argument("out")
    p_extract.add_argument("--chunks")
    p_compare = sub.add_parser("compare")
    p_compare.add_argument("reference")
    p_compare.add_argument("other")
    args = parser.parse_args()
    if args.command == "extract":
        chunks = None
        if args.chunks:
            chunks = [tuple(int(v) for v in c.split(",")) for c in args.chunks.split()]
        extract(args.db, args.out, chunks)
    else:
        compare(args.reference, args.other)


main()
