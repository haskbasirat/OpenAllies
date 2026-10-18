"""Untouched 4EB490/4EA350 memory eligibility and original class virtuals.

Actual 76AC90 reads an explicitly supplied Sim+F8 word. ObjectData/ActorData
type leaves and Object/Actor/Kingdom +A8 bodies execute. There are no service
seams in this predicate. Test scalars are a bounded input domain, not a claim
that a live Rust simulation phase or authored +16D producer already exists.
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
from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_READ

VTABLES = {2: 0xA2A2A0, 3: 0xA273F0, 4: 0xA299D8}


def run(case, image):
    machine = nm.Machine(case["precision"])
    old = (nm.P.image_size + 4095) & ~4095
    new = (image.image_size + 4095) & ~4095
    if new > old:
        machine.u.mem_map(image.base + old, new - old)
    for _, rva, size, offset in image.sections:
        machine.u.mem_write(image.base + rva, image.b[offset:offset + size])
    obj = machine.alloc(0x200, 0xA5)
    definition = machine.alloc(0x600, 0xA5)
    simulation = machine.alloc(0x200, 0xA5)
    machine.w(obj, VTABLES[case["native_class"]])
    machine.w(obj + 4, definition)
    machine.w(obj + 8, case["flags"])
    machine.w(definition, 0xA13318 if case["native_class"] == 2 else 0xA05540)
    machine.w(definition + 0x174, case["component_mask"])
    machine.wb(definition + 0x16D, case["announce_byte"])
    machine.w(0xB36550, simulation)
    machine.w(simulation + 0xF8, case["simulation_phase"])
    fields = {obj + 8: "flags", definition + 0x174: "component_mask",
              definition + 0x16D: "announce_byte", simulation + 0xF8: "simulation_phase"}
    reads = []
    pins = {}
    before = [bytes(machine.u.mem_read(obj, 0x200)), bytes(machine.u.mem_read(definition, 0x600)),
              bytes(machine.u.mem_read(simulation, 0x200))]

    def observed_read(_, access, address, size, value, __):
        if address in fields:
            reads.append([fields[address], size, int.from_bytes(machine.u.mem_read(address, size), "little")])

    def observe(_, address, size, __):
        if image.base <= address < image.base + image.image_size:
            raw = image.range(address, size)
            assert raw == nm.P.range(address, size) == nm.V.range(address, size)
            pins[f"{address:08x}"] = raw.hex()

    machine.u.hook_add(UC_HOOK_CODE, observe)
    machine.u.hook_add(UC_HOOK_MEM_READ, observed_read)
    result = bool(machine.call(0x4EB490, obj, [], 0) & 255)
    assert not machine.null_accesses and machine.outcome is None
    assert before == [bytes(machine.u.mem_read(obj, 0x200)), bytes(machine.u.mem_read(definition, 0x600)),
                      bytes(machine.u.mem_read(simulation, 0x200))]
    return {"result": result, "ordered_reads": reads, "owned_input_bytes_unchanged": True}, pins


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    flags = sorted({sum((1 << bit) for i, bit in enumerate([11, 22, 23, 29]) if mask & (1 << i))
                    for mask in range(16)} | {0xA0C46900, 0xFFFFFFFF})
    cases = []
    pins = {}
    for pc in ("Single24", "Double53", "Extended64"):
        for native_class in VTABLES:
            for flag in flags:
                for component_mask in (0, 1 << 26):
                    for announce_byte in (0, 0xFF):
                        for simulation_phase in (5, 6):
                            case = {"precision": pc, "native_class": native_class, "flags": flag,
                                    "component_mask": component_mask, "announce_byte": announce_byte,
                                    "simulation_phase": simulation_phase}
                            original, a = run(case, nm.P)
                            patched, b = run(case, nm.V)
                            assert original == patched and a == b
                            pins.update(a)
                            cases.append({**case, "output": original})
    slots = []
    for label, vtable, offsets in [("Object", 0xA2A2A0, [0xA8]), ("Actor", 0xA273F0, [0xA8]),
                                   ("Kingdom", 0xA299D8, [0xA8]),
                                   ("ObjectData", 0xA13318, [0x38, 0x40]),
                                   ("ActorData", 0xA05540, [0x38, 0x40])]:
        for offset in offsets:
            raw = nm.P.range(vtable + offset, 4)
            assert raw == nm.V.range(vtable + offset, 4)
            slots.append({"class": label, "vtable": f"{vtable:08x}", "offset": offset,
                          "slot_bytes": raw.hex(), "leaf": f"{int.from_bytes(raw, 'little'):08x}"})
    output = {"schema": 1, "scope": __doc__, "cases": cases, "instruction_pins": pins,
              "paired_case_count": len(cases), "native_execution_count": len(cases) * 2,
              "vtable_slots": slots,
              "source_images": [{"name": label, "sha256": hashlib.sha256(image.b).hexdigest()}
                                for label, image in (("original", nm.P), ("v14", nm.V))]}
    target = HERE / "native-memory-eligibility.json"
    if args.write:
        assert not target.exists(), "Do not overwrite frozen native evidence."
        target.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    else:
        assert json.loads(target.read_text(encoding="utf-8")) == output
    print(json.dumps({"paired_cases": len(cases), "native_executions": len(cases) * 2,
                      "instruction_pins": len(pins), "sha256": hashlib.sha256(target.read_bytes()).hexdigest()}))


if __name__ == "__main__":
    main()
