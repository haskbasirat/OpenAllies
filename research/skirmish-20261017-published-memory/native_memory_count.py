"""Actual Object virtual8C=4EA3D0 signed count predicate, original/v14/allPC.

This is a bounded field getter proof. The absent pointer is poisoned: native
does not read the allocation pointer while answering this predicate.
"""
from pathlib import Path
import sys, json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-ai"))
import native_machine as nm
ORIGINAL = nm.original

def main():
    rows, pins = [], {}
    for pc in nm.PRECISIONS:
        for count in [-2147483648, -1, 0, 1, 3, 2147483647]:
            outputs = []
            for image in [ORIGINAL, nm.modified]:
                nm.original = image
                m = nm.Machine(pc)
                obj = m.alloc(0x60)
                m.w(obj + 0x58, count & 0xffffffff)
                m.w(obj + 0x5c, 0xdeadbeef)
                result = m.call(0x4ea3d0, obj) & 0xff
                assert result == int(count > 0)
                outputs.append(result)
                pins.update(m.visited)
            assert outputs[0] == outputs[1]
            rows.append(dict(precision=pc, signed_count=count, return_al=outputs[0]))
    for va, raw in pins.items():
        assert ORIGINAL.range(va, len(raw)) == nm.modified.range(va, len(raw)) == raw
    doc = dict(schema=1, scope=__doc__, rows=rows,
        instructions=[dict(va=hex(va), bytes=raw.hex()) for va, raw in sorted(pins.items())],
        original_sha256=hashlib.sha256(ORIGINAL.b).hexdigest(),
        v14_sha256=hashlib.sha256(nm.modified.b).hexdigest())
    path = HERE / "native-memory-count.json"
    if "--write" in sys.argv:
        path.write_text(json.dumps(doc, indent=2) + "\n")
    else:
        assert json.loads(path.read_text()) == doc
    print(json.dumps(dict(rows=len(rows), runs=len(rows)*2, executed_pins=len(pins), sha256=hashlib.sha256(path.read_bytes()).hexdigest())))

if __name__ == "__main__":
    main()
