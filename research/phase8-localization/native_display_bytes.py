"""Run original DisplayString loading for no-BOM C-locale byte strings.

This reuses the documented heap/OS seams in phase8-triggers/native_mapstrings.py.
The original 856F90 reader, 96CBD7 isleadbyte, 9710C6 mbtowc, TGI parser,
494000 no-escape path, 5BA3C0 prefix handling, 4BA710 registration, and 4B9F40
lookup execute from the original executable. The two real developer-map names
were mislabeled as unsupported font escapes by aa-runtime/localization.rs.
No font-escape lookup table is synthesized here.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-triggers"))
from native_mapstrings import MapStrings, L, dc  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def utf16(text):
    raw = text.encode("utf-16le", errors="surrogatepass")
    return list(struct.unpack("<" + "H" * (len(raw) // 2), raw))


def run(name, raw, keys):
    machine = MapStrings()
    manager, _ = machine.load(raw)
    assert manager is not None
    outputs = []
    for key in keys:
        result = machine.lookup(manager, key)
        assert result is not None, key
        outputs.append({"key": key, "output_utf16": utf16(result)})
    return {
        "name": name,
        "bytes_hex": raw.hex(),
        "sha256": digest(raw),
        "lookups": outputs,
        "diagnostics": machine.diags,
        "var_reads": machine.var_reads,
    }, machine


def main():
    header = b'[Text language=Default]\r\n{\r\nids=table\r\nname="N"\r\n'
    fields = [
        ("ascii", b"Infantry Officer"),
        ("all_high_bytes", bytes(range(0x80, 0x100))),
        ("utf8_bytes_without_bom", b"caf\xc3\xa9"),
        ("high_before_quote", b"a\xe4"),
        ("c1_bytes", b"\x80\x9f\xff\xfe x"),
        ("leading_at", b"@Fallschrimj\xe4ger"),
        ("double_at", b"@@Fallschrimj\xe4ger"),
        ("embedded_at", b"a@b"),
        ("at_only", b"@"),
        ("empty", b""),
        ("spaces_preserved", b"  Fallschrimj\xe4ger  "),
    ]
    raw = header + b"".join(
        key.encode() + b'="' + value + b'"\r\n' for key, value in fields
    ) + b"}\r\n"
    rows, visited = [], {}
    row, machine = run("synthetic/no-bom-byte-strings", raw, [k for k, _ in fields])
    row["expected_source_order"] = [k for k, _ in fields]
    rows.append(row)
    visited.update(machine.visited)
    map_path = ROOT.parent / "reconstruction/assets/original/Data/Maps/DeveloperMaps/GER_Art_Map.RMP"
    raw, pins = dc.chunk(map_path, "MAPSTRINGS")
    row, machine = run("DeveloperMaps/GER_Art_Map.RMP", raw, ["name_192", "name_195"])
    row["map_sha256"] = pins["map_sha256"]
    rows.append(row)
    visited.update(machine.visited)
    # The narrow acceptance does not include high bytes in a BOM-marked UTF-8
    # file; the original selects its distinct MultiByteToWideChar path there.
    machine.visited = visited
    required = [0x856f90, 0x96cbd7, 0x9710c6, 0x971006, 0x494000, 0x5ba3c0, 0x4b9f40]
    assert all(address in visited for address in required), [hex(a) for a in required if a not in visited]
    document = {
        "schema": 1,
        "original_sha256": L.pe.ORIGINAL_SHA,
        "v14_sha256": L.pe.ACCEPTED_SHA,
        "producer_sha256": digest(Path(__file__).read_bytes()),
        "dependencies": {
            "phase8-triggers/native_mapstrings.py": digest((ROOT / "research/phase8-triggers/native_mapstrings.py").read_bytes()),
            "phase8-triggers/loader.py": digest((ROOT / "research/phase8-triggers/loader.py").read_bytes()),
        },
        "scope": __doc__.strip(),
        "cases": rows,
        "required_original_entries_executed": [hex(a) for a in required],
        "visited_windows": machine.windows(),
    }
    target = HERE / "native-display-bytes.json"
    target.write_text(json.dumps(document, indent=2) + "\n")
    dis = Cs(CS_ARCH_X86, CS_MODE_32)
    listing = []
    for address, length in [(0x856f90, 0x1fc), (0x96cbd7, 0x16), (0x9710c6, 0x35), (0x971006, 0xc0), (0x5ba3c0, 0x95)]:
        listing.append(f"\nOriginal instructions at {address:#010x}:\n")
        for instruction in dis.disasm(L.IMAGE.at(address, length), address):
            listing.append(f"{instruction.address:08x} {instruction.bytes.hex():24} {instruction.mnemonic} {instruction.op_str}\n")
    (HERE / "decoder-instructions.txt").write_text("".join(listing))
    print(json.dumps({"cases": len(rows), "lookups": sum(len(r["lookups"]) for r in rows), "fixture_sha256": digest(target.read_bytes()), "visited_windows": len(visited)}))


if __name__ == "__main__":
    main()
