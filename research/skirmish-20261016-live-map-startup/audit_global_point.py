"""Bounded static provenance audit for native global point shape B3A8C4.

Records PE zero-fill backing and every file-backed little-endian absolute word
within a broad neighboring window. This is not a proof against indirect or
computed writes, dynamic initialization through aliases, or later mutation.
No default point shape is authorized by this report.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-conversion"))
import native_machine as nm


def audit(image):
    target = 0xB3A8C4
    pe = struct.unpack_from("<I", image.b, 0x3C)[0]
    optional = pe + 24
    table = optional + struct.unpack_from("<H", image.b, pe + 20)[0]
    sections = []
    references = []
    for i, (virtual, rva, size, offset) in enumerate(image.sections):
        name = image.b[table + 40*i:table + 40*i + 8].split(b"\0")[0].decode("ascii")
        characteristics = struct.unpack_from("<I", image.b, table + 40*i + 36)[0]
        start = image.base + rva
        if start <= target < start + virtual:
            sections.append({"name": name, "virtual_address": start, "virtual_size": virtual,
                             "raw_size": size, "raw_offset": offset, "characteristics": characteristics,
                             "zero_filled_start": start + size, "zero_filled_end": start + virtual,
                             "full_shape_span_in_zero_filled_tail": start + size <= target and target + 24 <= start + virtual})
        for at in range(size - 3):
            word = struct.unpack_from("<I", image.b, offset + at)[0]
            if 0xB3A880 <= word < 0xB3A900:
                references.append({"word_address": start + at, "value": word,
                                   "section": name, "word_bytes": image.b[offset+at:offset+at+4].hex()})
    assert len(sections) == 1 and sections[0]["full_shape_span_in_zero_filled_tail"]
    return {"source_sha256": hashlib.sha256(image.b).hexdigest(),
            "containing_sections": sections, "absolute_word_references": references}


def main():
    original, patched = audit(nm.P), audit(nm.V)
    assert original["containing_sections"] == patched["containing_sections"]
    # v14 has appended code/metadata sections; report both independent scans.
    output = {"schema": 1, "scope": __doc__, "original": original, "v14": patched,
              "global_getter": {"address": "004e3e90", "bytes": nm.P.range(0x4E3E90, 6).hex()},
              "live_global_shape_producer_accepted": False,
              "remaining": "Static PE zero-fill is observed; absence of indirect alias writes/dynamic initialization is not proven. Explicit live point-shape owner remains required."}
    assert nm.P.range(0x4E3E90, 6) == nm.V.range(0x4E3E90, 6)
    target = HERE / "global-point-static-audit.json"
    encoded = json.dumps(output, indent=2) + "\n"
    if target.exists():
        assert target.read_text(encoding="utf-8") == encoded
    else:
        target.write_text(encoded, encoding="utf-8")
    print(json.dumps({"sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
                      "reference_words_original": len(original["absolute_word_references"]),
                      "reference_words_v14": len(patched["absolute_word_references"]),
                      "live_global_shape_producer_accepted": False}))


if __name__ == "__main__":
    main()
