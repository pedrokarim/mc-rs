"""Palette de blocs de mc-rs vue par BDS : états typés, hash réseau, syntaxe des commandes.

Lit `crates/mc-rs-server/data/canonical_block_states.nbt` (NBT réseau : longueurs en VarUInt32, entiers en
VarInt32 zigzag, composés racine concaténés ; indice = runtime ID de mc-rs) et calcule pour chaque état le
« network block hash » que BDS utilise quand `block-network-ids-are-hashes=true` (FNV-1a 32 bits du NBT
little-endian `{name, states}` aux clés triées, d'après dragonfly `server/world/network_block_hash.go`).
`check()` compare ces hashes à ceux du dump de BDS d'Allay (`blockStateHash`, même ordre d'états).

Usage en module : `from bds_palette import load_palette`. En script : `python scripts/dev/bds_palette.py`
vérifie la palette contre le dump d'Allay.
"""

import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PALETTE = ROOT / "crates" / "mc-rs-server" / "data" / "canonical_block_states.nbt"
ALLAY_STATES = ROOT / ".reference" / "Allay" / "data" / "resources" / "block_states.json"

TAG_END, TAG_BYTE, TAG_SHORT, TAG_INT, TAG_LONG, TAG_FLOAT, TAG_DOUBLE = 0, 1, 2, 3, 4, 5, 6
TAG_BYTE_ARRAY, TAG_STRING, TAG_LIST, TAG_COMPOUND, TAG_INT_ARRAY, TAG_LONG_ARRAY = 7, 8, 9, 10, 11, 12


class _Reader:
    def __init__(self, data: bytes):
        self.data = data
        self.pos = 0

    def byte(self) -> int:
        b = self.data[self.pos]
        self.pos += 1
        return b

    def varuint(self) -> int:
        shift = result = 0
        while True:
            b = self.byte()
            result |= (b & 0x7F) << shift
            if not b & 0x80:
                return result
            shift += 7

    def varint(self) -> int:
        n = self.varuint()
        return (n >> 1) ^ -(n & 1)

    def string(self) -> str:
        n = self.varuint()
        s = self.data[self.pos : self.pos + n].decode("utf-8")
        self.pos += n
        return s

    def payload(self, tag: int):
        if tag == TAG_BYTE:
            return ("byte", struct.unpack("<b", bytes([self.byte()]))[0])
        if tag == TAG_SHORT:
            v = struct.unpack_from("<h", self.data, self.pos)[0]
            self.pos += 2
            return ("short", v)
        if tag == TAG_INT:
            return ("int", self.varint())
        if tag == TAG_LONG:
            return ("long", self.varint())
        if tag == TAG_FLOAT:
            v = struct.unpack_from("<f", self.data, self.pos)[0]
            self.pos += 4
            return ("float", v)
        if tag == TAG_DOUBLE:
            v = struct.unpack_from("<d", self.data, self.pos)[0]
            self.pos += 8
            return ("double", v)
        if tag == TAG_STRING:
            return ("string", self.string())
        if tag == TAG_COMPOUND:
            out = {}
            while True:
                t = self.byte()
                if t == TAG_END:
                    return ("compound", out)
                name = self.string()
                out[name] = self.payload(t)
        if tag == TAG_LIST:
            t = self.byte()
            return ("list", [self.payload(t) for _ in range(self.varint())])
        raise ValueError(f"tag NBT {tag} non géré")


def _hash_state(name: str, states: dict) -> int:
    """FNV-1a 32 bits du NBT little-endian {name, states}, clés triées (dragonfly `networkBlockHash`)."""
    if name == "minecraft:unknown":
        return 0xFFFFFFFE

    def s(text: str) -> bytes:
        raw = text.encode("utf-8")
        return struct.pack("<H", len(raw)) + raw

    data = bytearray(b"\x0a\x00\x00")
    data += b"\x08" + s("name") + s(name)
    data += b"\x0a" + s("states")
    for key in sorted(states):
        kind, value = states[key]
        if kind == "string":
            data += b"\x08" + s(key) + s(value)
        elif kind == "byte":
            data += b"\x01" + s(key) + struct.pack("<B", value & 0xFF)
        elif kind == "short":
            data += b"\x02" + s(key) + struct.pack("<H", value & 0xFFFF)
        elif kind == "int":
            data += b"\x03" + s(key) + struct.pack("<I", value & 0xFFFFFFFF)
        else:
            raise ValueError(f"{name} : état {key} de type {kind} non géré")
    data += b"\x00\x00"
    h = 0x811C9DC5
    for b in data:
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return h


def command_block(name: str, states: dict) -> str:
    """Bloc dans la syntaxe des commandes de BDS : `minecraft:wheat ["growth"=7]`. Les octets sont des
    booléens (`true`/`false`), les chaînes entre guillemets."""
    parts = []
    for key in sorted(states):
        kind, value = states[key]
        if kind == "string":
            parts.append(f'"{key}"="{value}"')
        elif kind == "byte":
            parts.append(f'"{key}"={"true" if value else "false"}')
        else:
            parts.append(f'"{key}"={value}')
    return f"{name} [{','.join(parts)}]" if parts else name


def load_palette() -> list[dict]:
    """États de la palette dans l'ordre des runtime IDs : `name`, `states` ({clé: (type, valeur)}),
    `hash` (network block hash de BDS), `command` (syntaxe de commande)."""
    reader = _Reader(PALETTE.read_bytes())
    palette = []
    while reader.pos < len(reader.data):
        tag = reader.byte()
        reader.string()
        _, root = reader.payload(tag)
        name = root["name"][1]
        states = root["states"][1]
        palette.append({
            "name": name,
            "states": states,
            "hash": _hash_state(name, states),
            "command": command_block(name, states),
        })
    return palette


def check() -> None:
    palette = load_palette()
    allay = json.loads(ALLAY_STATES.read_text(encoding="utf-8"))
    if len(allay) != len(palette):
        sys.exit(f"palette : {len(palette)} états, dump d'Allay : {len(allay)}")
    bad = [i for i, (p, a) in enumerate(zip(palette, allay)) if p["name"] != a["name"] or p["hash"] != a["blockStateHash"]]
    if bad:
        i = bad[0]
        sys.exit(f"{len(bad)} hashes différents, premier : {i} {palette[i]['command']} {palette[i]['hash']} ≠ {allay[i]['blockStateHash']}")
    print(f"{len(palette)} états, hashes identiques à ceux de BDS (dump d'Allay)")


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    check()
