"""Index rapide du binaire Windows de BDS, sans analyse Ghidra : chaînes, références, RTTI, vtables.

L'analyse automatique complète de Ghidra sur `bedrock_server.exe` (plus de 200 Mo, des centaines de milliers
de fonctions) dure des heures. Ce script trouve en quelques secondes les fonctions utiles, que Ghidra
décompile ensuite une à une (`ghidra/DecompileFunctions.java`) :

- `strings` : adresse de chaque chaîne, instructions `lea reg, [rip+disp32]` qui la chargent, et fonction
  qui les contient (table `.pdata` des RUNTIME_FUNCTION, fragments chaînés ramenés à leur fonction) ;
- `rtti` : classes C++ par leur nom MSVC (`.?AVOreBlock@@`), CompleteObjectLocator, vtable primaire et ses
  entrées (adresses de fonctions).

Format PE/COFF : Microsoft « PE Format » (en-têtes, `.pdata`, UNWIND_INFO) ; RTTI MSVC x64 : TypeDescriptor
(nom à +16), CompleteObjectLocator signature 1 (RVA du TypeDescriptor à +12, de lui-même à +20), pointeur
vers le COL juste avant la vtable.

Usage :
    python scripts/dev/bds_pe_index.py <bedrock_server.exe> strings "minecraft:fortune" ...
    python scripts/dev/bds_pe_index.py <bedrock_server.exe> rtti OreBlock LeavesBlock ...
Sortie JSON sur la sortie standard. Méthode : dev-docs/34-BDS-RETRO-INGENIERIE.md.
"""

import bisect
import json
import struct
import sys
from pathlib import Path

import numpy as np


class PeImage:
    def __init__(self, path: Path):
        self.data = path.read_bytes()
        d = self.data
        pe = struct.unpack_from("<I", d, 0x3C)[0]
        if d[pe : pe + 4] != b"PE\0\0":
            sys.exit("pas un fichier PE")
        n_sections, _, _, _, opt_size = struct.unpack_from("<HIIIH", d, pe + 6)
        opt = pe + 24
        if struct.unpack_from("<H", d, opt)[0] != 0x20B:
            sys.exit("PE32+ attendu")
        self.image_base = struct.unpack_from("<Q", d, opt + 24)[0]
        sec = opt + opt_size
        self.sections = []
        for i in range(n_sections):
            o = sec + 40 * i
            name = d[o : o + 8].rstrip(b"\0").decode()
            vsize, va, rsize, raw = struct.unpack_from("<IIII", d, o + 8)
            self.sections.append((name, va, max(vsize, rsize), raw, rsize))
        self.functions = self._load_pdata()

    def section(self, name):
        return next(s for s in self.sections if s[0] == name)

    def rva_to_off(self, rva):
        for _, va, size, raw, rsize in self.sections:
            if va <= rva < va + size and rva - va < rsize:
                return raw + rva - va
        return None

    def is_code(self, rva):
        _, va, size, _, _ = self.section(".text")
        return va <= rva < va + size

    def _load_pdata(self):
        _, va, size, raw, rsize = self.section(".pdata")
        n = min(size, rsize) // 12
        entries = np.frombuffer(self.data, dtype="<u4", count=n * 3, offset=raw).reshape(n, 3)
        entries = entries[entries[:, 0] != 0]
        order = np.argsort(entries[:, 0])
        return entries[order]

    def _primary_begin(self, begin, unwind):
        # UNW_FLAG_CHAININFO : le RUNTIME_FUNCTION parent suit les codes de déroulement.
        for _ in range(32):
            off = self.rva_to_off(unwind)
            if off is None:
                return begin
            flags = self.data[off] >> 3
            if not flags & 0x4:
                return begin
            count = self.data[off + 2]
            chained = off + 4 + 2 * (count + (count & 1))
            begin, _, unwind = struct.unpack_from("<III", self.data, chained)
        return begin

    def function_of(self, rva):
        begins = self.functions[:, 0]
        i = bisect.bisect_right(begins, rva) - 1
        if i < 0:
            return None
        begin, end, unwind = (int(x) for x in self.functions[i])
        if not begin <= rva < end:
            return None
        return self._primary_begin(begin, unwind)

    def find_all(self, needle: bytes, section: str):
        _, va, size, raw, rsize = self.section(section)
        blob = self.data[raw : raw + rsize]
        out, i = [], blob.find(needle)
        while i >= 0:
            out.append(va + i)
            i = blob.find(needle, i + 1)
        return out

    def lea_refs(self, targets):
        """Instructions `lea r64, [rip+disp32]` (REX.W 48/4C, 8D, ModRM mod=00 rm=101) vers `targets`."""
        _, va, size, raw, rsize = self.section(".text")
        b = np.frombuffer(self.data, dtype=np.uint8, count=rsize, offset=raw)
        cand = np.nonzero((b[1:-6] == 0x8D) & ((b[:-7] & 0xFB) == 0x48) & ((b[2:-5] & 0xC7) == 0x05))[0]
        disp = (
            b[cand + 3].astype(np.int64)
            | (b[cand + 4].astype(np.int64) << 8)
            | (b[cand + 5].astype(np.int64) << 16)
            | (b[cand + 6].astype(np.int64) << 24)
        )
        disp = np.where(disp >= 1 << 31, disp - (1 << 32), disp)
        target = va + cand + 7 + disp
        wanted = np.isin(target, np.array(sorted(targets), dtype=np.int64))
        return [(int(va + c), int(t)) for c, t in zip(cand[wanted], target[wanted])]


def cmd_strings(pe: PeImage, names):
    result = {}
    addrs = {}
    for name in names:
        hits = []
        for sec in (".rdata", ".data"):
            hits += [a for a in pe.find_all(name.encode() + b"\0", sec)]
        # Chaîne exacte : précédée d'un octet nul ou en début d'alignement.
        exact = [a for a in hits if pe.data[pe.rva_to_off(a) - 1] == 0]
        addrs[name] = exact
    refs = pe.lea_refs({a for v in addrs.values() for a in v})
    by_target = {}
    for ip, target in refs:
        by_target.setdefault(target, []).append(ip)
    for name, exact in addrs.items():
        entries = []
        for a in exact:
            for ip in by_target.get(a, []):
                f = pe.function_of(ip)
                entries.append(
                    {"string": hex(pe.image_base + a), "ref": hex(pe.image_base + ip),
                     "function": hex(pe.image_base + f) if f is not None else None}
                )
        result[name] = {"addresses": [hex(pe.image_base + a) for a in exact], "refs": entries}
    return result


def cmd_rtti(pe: PeImage, classes):
    _, rva_lo, rsize_lo, raw_lo, rsz = pe.section(".rdata")
    rdata = np.frombuffer(pe.data, dtype="<u4", count=rsz // 4, offset=raw_lo)
    rdata_q = np.frombuffer(pe.data, dtype="<u8", count=rsz // 8, offset=raw_lo)
    result = {}
    for cls in classes:
        mangled = f".?AV{cls}@@".encode() + b"\0"
        tds = [a - 16 for sec in (".data", ".rdata") for a in pe.find_all(mangled, sec)]
        entry = {"type_descriptors": [hex(pe.image_base + t) for t in tds], "vtables": []}
        for td in tds:
            for idx in np.nonzero(rdata == td)[0]:
                col = rva_lo + (int(idx) - 3) * 4
                off = pe.rva_to_off(col)
                if off is None:
                    continue
                sig, obj_off, _, td_rva, _, self_rva = struct.unpack_from("<IIIIII", pe.data, off)
                if sig != 1 or td_rva != td or self_rva != col:
                    continue
                for q in np.nonzero(rdata_q == pe.image_base + col)[0]:
                    vt = rva_lo + (int(q) + 1) * 8
                    methods = []
                    p = pe.rva_to_off(vt)
                    while p is not None and len(methods) < 512:
                        target = struct.unpack_from("<Q", pe.data, p)[0] - pe.image_base
                        if not 0 <= target < 1 << 32 or not pe.is_code(target):
                            break
                        methods.append(hex(pe.image_base + target))
                        p += 8
                    entry["vtables"].append(
                        {"col": hex(pe.image_base + col), "offset": obj_off,
                         "vtable": hex(pe.image_base + vt), "methods": methods}
                    )
        result[cls] = entry
    return result


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if len(sys.argv) < 4 or sys.argv[2] not in ("strings", "rtti"):
        sys.exit(__doc__)
    pe = PeImage(Path(sys.argv[1]))
    handler = cmd_strings if sys.argv[2] == "strings" else cmd_rtti
    print(json.dumps(handler(pe, sys.argv[3:]), indent=1))


main()
