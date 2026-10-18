"""Untouched world-object list insertion 4F1F70, original/v14 independently.

Execute the actual world-constructor prefix through its world+8 store and stop
before unrelated 511DF0. Then call actual 4F1F70 for each flag word. The only
descendant seam reached is successful allocation 6F7810 (12 bytes, A5 filled).
No engine, SDK or original file is modified. IDs below label allocations;
they are not claimed native GTIDs. All three x87 PCs are explicit.
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
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP


def run(case, image, pc):
    machine = nm.Machine(pc)
    previous_size = (nm.P.image_size + 4095) & ~4095
    current_size = (image.image_size + 4095) & ~4095
    if current_size > previous_size:
        machine.u.mem_map(image.base + previous_size, current_size - previous_size)
    for _, rva, size, offset in image.sections:
        machine.u.mem_write(image.base + rva, image.b[offset:offset + size])
    world = machine.alloc(0x400, 0xA5)
    machine.w(nm.STACK, nm.STOP)
    machine.u.reg_write(UC_X86_REG_ESP, nm.STACK)
    machine.u.reg_write(UC_X86_REG_ECX, world)
    # This prefix publishes the world and stores null head, no service seam.
    machine.run(0x4F7D00, 0x4F7D47)
    assert machine.r(world + 8) == 0
    objects = {label: machine.alloc(0x40, 0xA5) for label in sorted({x[0] for x in case})}
    object_labels = {address: label for label, address in objects.items()}
    nodes = []
    allocations = []
    instructions = {}

    def allocate(m):
        assert m.arg() == 12
        pointer = m.alloc(12, 0xA5)
        nodes.append(pointer)
        allocations.append(12)
        m.ret(0, pointer)

    def node_label(pointer):
        if pointer == 0:
            return None
        return nodes.index(pointer) + 1

    def link_label(pointer):
        if pointer == world + 8:
            return {"head": True}
        return {"next_of": node_label(pointer - 8)}

    def snapshot():
        return {
            "head": node_label(machine.r(world + 8)),
            "nodes": [{"node": index + 1, "object": object_labels[machine.r(address)],
                       "previous_link": link_label(machine.r(address + 4)),
                       "next": node_label(machine.r(address + 8))}
                      for index, address in enumerate(nodes)],
        }

    def observe(_, address, size, __):
        if image.base <= address < image.base + image.image_size:
            raw = image.range(address, size)
            assert raw == nm.P.range(address, size) == nm.V.range(address, size)
            instructions[f"{address:08x}"] = raw.hex()

    machine.u.hook_add(UC_HOOK_CODE, observe)
    machine.seam(0x6F7810, allocate)
    records = []
    for label, flags in case:
        machine.w(objects[label] + 8, flags)
        before_allocations = len(allocations)
        machine.call(0x4F1F70, world, [objects[label]], 4)
        assert len(allocations) - before_allocations == bool(flags & 0x100)
        records.append({"object": label, "flags": flags, "state": snapshot()})
    return {"constructor_head_null": True, "allocations": allocations, "steps": records}, instructions


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    sequences = [
        [(1, 0)], [(1, 0x100)], [(1, 0xFFFFFFFF)],
        [(1, 0xFFFFFEFF)], [(1, 0x80)], [(1, 0x20000000)],
        [(1, 0x100), (2, 0x100), (3, 0x100)],
        [(1, 0x100), (1, 0x100), (1, 0)],
        [(1, 0), (2, 0x100), (3, 0xFFFFFEFF), (4, 0x100), (2, 0x100)],
    ]
    for count in (7, 17, 65):
        sequences.append([(i % 5 + 1, (0x80000100 if i % 3 else 0xA0042800)) for i in range(count)])
    results = []
    pins = {}
    for pc in ("Single24", "Double53", "Extended64"):
        for index, sequence in enumerate(sequences):
            original, a = run(sequence, nm.P, pc)
            patched, b = run(sequence, nm.V, pc)
            assert original == patched
            assert a == b
            pins.update(a)
            results.append({"case": index, "precision": pc, "sequence": sequence, "output": original})
    # Constructor prefix instructions were executed before observation began;
    # pin its exact contiguous prefix independently as well.
    assert nm.P.range(0x4F7D00, 0x47) == nm.V.range(0x4F7D00, 0x47)
    output = {"schema": 1, "scope": __doc__, "cases": results,
              "paired_case_count": len(results), "native_execution_count": len(results) * 2,
              "source_images": [{"name": label, "sha256": hashlib.sha256(image.b).hexdigest()}
                                for label, image in (("original", nm.P), ("v14", nm.V))],
              "instruction_pins": pins,
              "constructor_prefix": {"address": "004f7d00", "bytes": nm.P.range(0x4F7D00, 0x47).hex()},
              "allocation_failure_or_reentrant_allocator_claimed": False}
    target = HERE / "native-world-object-list.json"
    if args.write:
        assert not target.exists(), "Do not overwrite frozen native evidence."
        target.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    else:
        # Source sequences use tuples; JSON stores arrays. Normalize the new
        # output rather than incorrectly treating that representation as a
        # native mismatch. No frozen evidence bytes are rewritten.
        assert json.loads(target.read_text(encoding="utf-8")) == json.loads(json.dumps(output))
    print(json.dumps({"paired_cases": len(results), "native_executions": len(results) * 2,
                      "instruction_pins": len(pins), "sha256": hashlib.sha256(target.read_bytes()).hexdigest()}))


if __name__ == "__main__":
    main()
