"""Actual first-Kingdom field-shaped footprint/visibility service bodies.

The point is an explicit fixture input pending the separately owned startup
producer receipt. No shape iterator, footprint, virtual getter, phase getter or
memory-eligibility body is intercepted. Current zone sets are explicitly null
fixture state; initial-map construction is proved separately. Not full Init.
"""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-movement"))
import harness as h
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import *

H = lambda raw: hashlib.sha256(raw).hexdigest()
INPUT = ROOT / "research/skirmish-20261016-load-spatial/actual-first-kingdom.json"


def run(case, image, phase, mask):
    m = h.Machine(case["precision"])
    if image is h.V:
        old = (h.P.image_size + 4095) & ~4095
        new = (image.image_size + 4095) & ~4095
        if new > old: m.u.mem_map(image.base + old, new - old)
        for vs, r, size, off in image.sections:
            if size: m.u.mem_write(image.base + r, image.b[off:off+min(size, max(vs, size))])
    member = case["member"]
    grid = case["grid"]
    obj, definition, world, map_object, session = [m.alloc(n) for n in (0x300, 0x600, 0x200, 0x100, 0x200)]
    width, height = grid["dimensions"]
    tiles = m.alloc(width * height * 48)
    m.w32(0xb36534, world)
    m.w32(world + 0x30, map_object)
    m.w32(map_object + 4, grid["coordinate_shift"], grid["cell_size_bits"],
          h.f2b(width * 2**grid["coordinate_shift"]), h.f2b(height * 2**grid["coordinate_shift"]))
    m.w32(map_object + 0x1c, width, height)
    m.w32(map_object + 0x28, tiles)
    # Explicit fixture point input; this does not certify startup provenance.
    m.w32(0xb3a8c4, 0, 0, 0, 0, 0, 0)
    m.w32(obj, 0xa299d8, definition, member["actor"]["flags"] | (1 << 26))
    m.w32(definition, 0xa05540)
    shape = member["blocking"]
    m.w32(definition + 0x38, shape["kind"], shape["radius_bits"], *shape["bounds"])
    m.w32(definition + 0x174, mask)
    m.w32(obj + 0x20, *member["actor"]["position"], member["height_bits"], *member["actor"]["facing"], member["facing_z_bits"])
    m.w32(obj + 0x54, 0x12ab34cd, 0, 0)
    m.w32(0xb36550, session)
    m.w32(session + 0xf8, phase)
    visited_tiles = []
    def read(u, access, address, size, value, data):
        if tiles <= address < tiles + width * height * 48 and (address - tiles) % 48 == 4:
            visited_tiles.append((address - tiles) // 48)
    m.u.hook_add(UC_HOOK_MEM_READ, read)
    m.call(0x4ea150, obj)
    assert m.fpu_top() == 0
    before = bytes(m.u.mem_read(obj, 0x300))
    m.call(0x4eb490, obj)
    eligible = m.u.reg_read(UC_X86_REG_EAX) & 255
    assert eligible == 0
    m.call(0x4ebc70, obj)
    assert bytes(m.u.mem_read(obj, 0x300)) == before
    return {"occupancy_indices": visited_tiles, "eligible_al": eligible,
            "detection_word_unchanged": m.r32(obj + 0x54), "memory_count": m.r32(obj + 0x58)}


def main():
    capture = json.loads(INPUT.read_text())
    cases = []
    for actual in capture["cases"]:
        for phase in [0, 1, 3, 5, 6]:
            for mask in [0, 1 << 26]:
                original = run(actual, h.P, phase, mask)
                assert original == run(actual, h.V, phase, mask)
                cases.append({"precision": actual["precision"], "phase": phase,
                              "explicit_component_mask_probe": mask,
                              "input_member": actual["member"], "grid": actual["grid"],
                              "explicit_fixture_point": [0]*6, "output": original})
    pins = [{"va": hex(a), "bytes": raw} for a, raw in sorted(h.VISITED.items())]
    assert all(int(v["va"],16) not in h.ALLOCATORS for v in pins)
    doc = {"schema": 1, "scope": __doc__, "producer_sha256": H(Path(__file__).read_bytes()),
           "source_capture_sha256": H(INPUT.read_bytes()), "original_sha256": h.ORIGINAL_SHA,
           "v14_sha256": h.V14_SHA, "cases": cases, "executed_address_byte_pins": pins,
           "seams": {}}
    raw = (json.dumps(doc, indent=1) + "\n").encode()
    path = HERE / "native-current-services.json"
    if "--write" in sys.argv: path.write_bytes(raw)
    else: assert json.loads(path.read_text()) == doc
    print(json.dumps({"rows":len(cases), "fresh_original_v14_machines":len(cases)*2,
                      "executed_pins":len(pins), "sha256":H(raw), "immutable":"--write" not in sys.argv}))


if __name__ == "__main__": main()
