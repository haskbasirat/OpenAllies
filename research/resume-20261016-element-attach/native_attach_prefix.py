"""Original5705A0 and4D1350/571640 prefix, stopping at Recruit582C40 entry.

This does not execute or simulate Recruit. The stop is an instrumentation
boundary, not a claim that native Recruit throws. Page zero is unmapped for
the measured prefix; original/v14 images execute separately.
"""
from pathlib import Path
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase7-actor-live"))
import ability_flags as baseline


class Boundary(Exception):
    pass


class Machine(baseline.Machine):
    def hook(self, uc, pc, size, user):
        if pc == 0x582C40:
            self.cut = pc
            raise Boundary()
        super().hook(uc, pc, size, user)


def main():
    old = json.loads((ROOT / "research/phase7-actor-live/native-ability-flags.json").read_text())
    rows, observed = [], {}
    for image_name, image in [("original", baseline.P), ("v14", baseline.V)]:
        for flags, state, actions, slot in [
            (2692965632, (4 << 32) | 134742016, 1048576, 4),
            (0, 0, 0x123456789ABC, -1),
            (0x20000000, 0, 0xFFFFFFFFFFFFFFFF, 0),
            (0, 1 << 27, 0, 5),
        ]:
            source = {**old["cases"][0]["input"], "flags": flags,
                      "state": state, "actions": actions, "wall": False,
                      "has_element": True, "element_definition": False,
                      "element_parent": False, "element_ready": False,
                      "has_org": False, "other": [16, 21, 13, 10, 15, 19],
                      "mind": True}
            machine = Machine(source)
            old_size = (baseline.P.image_size + 4095) & ~4095
            new_size = (image.image_size + 4095) & ~4095
            if new_size > old_size:
                machine.u.mem_map(image.base + old_size, new_size - old_size)
            for _, rva, size, offset in image.sections:
                machine.u.mem_write(image.base + rva, image.b[offset:offset + size])
            # No registration/allocator/seh seams run in the measured prefix.
            machine.u.mem_unmap(0, 4096)
            machine.observed = {}
            machine.events = []
            machine.cut = None
            try:
                machine.invoke(0x5705A0, machine.elem, machine.parent, slot & 0xFFFFFFFF)
                raise AssertionError("expected native Recruit call boundary")
            except Boundary:
                pass
            assert machine.cut == 0x582C40
            assert machine.r(machine.elem + 0x10) == machine.parent
            observed.update(machine.observed)
            rows.append({
                "image": image_name,
                "input": {"flags": flags, "state": state, "actions": actions, "slot": slot},
                "output": {"boundary": "0x582c40", "organization": 4099,
                           "slot": machine.r(machine.elem + 0x18),
                           "attached": machine.u.mem_read(machine.elem + 0x1C, 1)[0],
                           "actions": machine.mask(machine.thing)},
            })
            assert rows[-1]["output"]["actions"] == 0
    for address, code in observed.items():
        assert code == baseline.P.range(address, len(code)) == baseline.V.range(address, len(code))
    result = {
        "scope": "Original5705A0→4D1350→571640 prefix through first Recruit582C40 call; Recruit and later callbacks are not executed",
        "original_sha256": baseline.H(baseline.P.b), "v14_sha256": baseline.H(baseline.V.b),
        "producer_sha256": baseline.H(Path(__file__).read_bytes()),
        "cases": rows,
        "instructions": [{"va": hex(a), "bytes": b.hex()} for a, b in sorted(observed.items())],
        "seams_during_prefix": [],
        "boundary": "Emulation stops at582C40 entry; this is not a native exception result",
    }
    (HERE / "native-attach-prefix.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"cases": len(rows), "instructions": len(observed), "passed": True}))


if __name__ == "__main__":
    main()
