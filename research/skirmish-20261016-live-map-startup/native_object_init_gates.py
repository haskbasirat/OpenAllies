"""Actual 4EE4C0 membership and 4EB6E0 blocking entry gates.

The Object/Actor/Kingdom vtable +B0 bodies execute, rather than substituting a
Boolean by address label. A reached descriptor builder (4CF210) or blocking
query constructor (554350) stops the case as an explicit required service.
No insertion/removal, collision, footprint, visibility or exception semantics
are claimed. Each original/v14 image is remapped independently, at all 3 PCs.
"""
from pathlib import Path
import argparse
import hashlib
import json
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-conversion"))
import native_machine as nm
from unicorn import UC_HOOK_CODE


class Required(Exception):
    pass


VTABLES = {2: 0xA2A2A0, 3: 0xA273F0, 4: 0xA299D8}


def run(native_class, flags, entry, image, pc):
    machine = nm.Machine(pc)
    old = (nm.P.image_size + 4095) & ~4095
    new = (image.image_size + 4095) & ~4095
    if new > old:
        machine.u.mem_map(image.base + old, new - old)
    for _, rva, size, offset in image.sections:
        machine.u.mem_write(image.base + rva, image.b[offset:offset + size])
    obj = machine.alloc(0x200, 0xA5)
    world = machine.alloc(0x400, 0xA5)
    terrain = machine.alloc(0x100, 0xA5)
    machine.w(obj, VTABLES[native_class])
    machine.w(obj + 8, flags)
    machine.w(0xB36534, world)
    machine.w(world + 0x30, terrain)
    before = bytes(machine.u.mem_read(obj, 0x200))
    pins = {}
    reached = []

    def required(name):
        def stop(_):
            reached.append(name)
            raise Required(name)
        return stop

    # Stops before any descriptor writes or any downstream owned-world effect.
    machine.seam(0x4CF210, required("membership_descriptor_4CF210"))
    machine.seam(0x554350, required("blocking_query_554350"))

    def observe(_, address, size, __):
        if image.base <= address < image.base + image.image_size:
            raw = image.range(address, size)
            assert raw == nm.P.range(address, size) == nm.V.range(address, size)
            pins[f"{address:08x}"] = raw.hex()

    machine.u.hook_add(UC_HOOK_CODE, observe)
    outcome = "returned"
    try:
        machine.call(entry, obj, [], 0)
    except Required:
        outcome = "required_service"
    assert not machine.null_accesses
    assert machine.outcome is None
    assert bytes(machine.u.mem_read(obj, 0x200)) == before
    if entry == 0x4EE4C0:
        # The actual +B0 body in all admitted classes returns true.
        assert (outcome == "required_service") == (flags & (1 << 26) == 0)
        assert reached == ([] if flags & (1 << 26) else ["membership_descriptor_4CF210"])
    else:
        assert (outcome == "required_service") == (flags & (1 << 10) != 0)
        assert reached == (["blocking_query_554350"] if flags & (1 << 10) else [])
    return {"outcome": outcome, "required": reached, "object_bytes_unchanged": True}, pins


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    flags = sorted({sum((1 << bit) for i, bit in enumerate([3, 10, 14, 26]) if mask & (1 << i))
                    for mask in range(16)} | {0xFFFFFFFF, 0xA0C46900, 0xA0042800, 0x80000000})
    cases = []
    pins = {}
    for pc in ("Single24", "Double53", "Extended64"):
        for native_class in VTABLES:
            for flag in flags:
                for entry in (0x4EE4C0, 0x4EB6E0):
                    original, a = run(native_class, flag, entry, nm.P, pc)
                    patched, b = run(native_class, flag, entry, nm.V, pc)
                    assert original == patched and a == b
                    pins.update(a)
                    cases.append({"precision": pc, "native_class": native_class, "flags": flag,
                                  "entry": f"{entry:08x}", "output": original})
    table = []
    for native_class, vtable in VTABLES.items():
        slot = vtable + 0xB0
        raw = nm.P.range(slot, 4)
        assert raw == nm.V.range(slot, 4)
        leaf = int.from_bytes(raw, "little")
        assert leaf == 0x6A71D0 and nm.P.range(leaf, 3) == nm.V.range(leaf, 3) == b"\xb0\x01\xc3"
        table.append({"native_class": native_class, "vtable": f"{vtable:08x}",
                      "slot": f"{slot:08x}", "slot_bytes": raw.hex(),
                      "leaf": f"{leaf:08x}", "leaf_bytes": "b001c3", "returns_true": True})
    output = {"schema": 1, "scope": __doc__, "paired_case_count": len(cases),
              "native_execution_count": len(cases) * 2, "cases": cases, "vtables": table,
              "source_images": [{"name": label, "sha256": hashlib.sha256(image.b).hexdigest()}
                                for label, image in (("original", nm.P), ("v14", nm.V))],
              "instruction_pins": pins}
    target = HERE / "native-object-init-gates.json"
    if args.write:
        assert not target.exists(), "Do not overwrite frozen native evidence."
        target.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    else:
        assert json.loads(target.read_text(encoding="utf-8")) == output
    print(json.dumps({"paired_cases": len(cases), "native_executions": len(cases) * 2,
                      "instruction_pins": len(pins), "sha256": hashlib.sha256(target.read_bytes()).hexdigest()}))


if __name__ == "__main__":
    main()
