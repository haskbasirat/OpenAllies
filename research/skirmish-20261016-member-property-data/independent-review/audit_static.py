"""Pin bounded native windows directly from both accepted executable images."""
from pathlib import Path
import hashlib
import importlib.util
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
spec = importlib.util.spec_from_file_location("member_property_author_static", HERE.parent / "native_member_property_data.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
cs = Cs(CS_ARCH_X86, CS_MODE_32)
windows = []
for label, va, size in [
    ("cursor body", 0x45ec60, 0x14),
    ("constructor list clear", 0x462830, 0x0c),
    ("ElementComponent selection through SupportProperty call", 0x46682b, 0x27c),
    ("list body through ret", 0x4c4cf0, 0x111),
]:
    original = m.ORIGINAL.range(va, size)
    assert original == m.h.V.range(va, size)
    windows.append({"label": label, "va": hex(va), "length": size,
                    "original_v14_byte_identical": True,
                    "sha256": hashlib.sha256(original).hexdigest(),
                    "instructions": [{"va": hex(i.address), "bytes": i.bytes.hex(),
                                      "mnemonic": i.mnemonic, "operands": i.op_str}
                                     for i in cs.disasm(original, va)]})

strings = []
for label, va in [("ElementComponent", 0xa083bc), ("SupportProperty", 0xa08204), ("property_IDS", 0xa178e4),
                  ("Unknown property format", 0xa24120)]:
    raw = bytearray()
    while (unit := m.ORIGINAL.range(va + len(raw), 2)) != b"\0\0":
        assert unit == m.h.V.range(va + len(raw), 2)
        raw.extend(unit)
    strings.append({"label": label, "va": hex(va), "text": raw.decode("utf-16-le"),
                    "sha256_utf16le_without_nul": hashlib.sha256(raw).hexdigest()})

files = ["crates/aa-rules/src/native_member_property_data.rs",
         "crates/aa-rules/tests/native_member_property_data.rs",
         "crates/aa-rules/src/native_layout_data.rs",
         "crates/aa-rules/src/resolved.rs", "crates/aa-rules/src/native_load.rs",
         "crates/aa-runtime/src/catalog.rs", "Cargo.toml", "Cargo.lock",
         "crates/aa-rules/Cargo.toml"]
pins = [{"path": f, "sha256": hashlib.sha256((ROOT/f).read_bytes()).hexdigest()} for f in files]
doc = {"original_sha256": m.h.ORIGINAL_SHA, "v14_sha256": m.h.V14_SHA,
       "scope": "Bounded static inspection; the constructor and ActorData call site are not newly emulated by this review.",
       "windows": windows, "strings": strings, "reviewed_source_and_manifest_pins": pins}
with (HERE / "static-review.json").open("w", encoding="utf-8", newline="\n") as f:
    f.write(json.dumps(doc, indent=2) + "\n")
print(json.dumps({"windows": len(windows), "strings": strings, "source_files_pinned": len(pins)}))
