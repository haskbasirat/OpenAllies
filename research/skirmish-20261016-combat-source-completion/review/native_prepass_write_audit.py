"""Independent native prepass leaf write sets for the CV dependency slice.

Runs the original and v14 code on distinct, typed synthetic source objects.
Successful allocation is the only hooked application service. This does not
execute full 4BB480/4BB130/4BB730 or certify their absence of failure in a real
startup. It proves the listed leaves' writes under the supplied valid graph.
"""
from pathlib import Path
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / "research/phase8-economy"))
from harness import Machine, MODES, ORIGINAL, ACCEPTED, sha, pin
from unicorn import UC_HOOK_MEM_WRITE

PROTECTED = [0x3c, 0x174, 0x1d4, 0x1e4, 0x1ec, 0x1f0, 0x210, 0x228,
             0x284, 0x288, 0x2f0, 0x30c, 0x35c, 0x36c, 0x464, 0x468, 0x478]

class Audit(Machine):
    def __init__(self, cw, image):
        super().__init__(cw)
        if image == "v14":
            old = (ORIGINAL.size + 4095) & ~4095
            new = (ACCEPTED.size + 4095) & ~4095
            if new > old:
                self.uc.mem_map(ORIGINAL.base + old, new - old)
            for rva, size, offset, _ in ACCEPTED.sections:
                if size:
                    self.uc.mem_write(ACCEPTED.base + rva, ACCEPTED.data[offset:offset+size])
        self.cursor = 0x2001000
        self.regions = {}
        self.writes = []
        self.ranges = [(0x400000, 0x1000000)]
        self.seam(0x6f7810, lambda m, esp: (m.alloc(m.arg(0)), 0))
        self.uc.hook_add(UC_HOOK_MEM_WRITE, self.write)

    def fixture(self, name, size):
        address = self.cursor
        self.cursor += (size + 15) & ~15
        self.uc.mem_write(address, b"\0" * size)
        self.regions[name] = (address, size)
        return address

    def actor(self, name):
        actor = self.fixture(name, 0x600)
        for index, offset in enumerate(PROTECTED):
            self.put(actor + offset, 0x3f800000 + index)
        return actor

    def write(self, uc, access, address, size, value, _):
        for name, (start, length) in self.regions.items():
            if start <= address < start + length:
                self.writes.append([name, address-start, size, value & ((1 << (size*8))-1)])
                break

    def protected(self):
        return {name: {str(offset): self.read(address+offset) for offset in PROTECTED}
                for name, (address, size) in self.regions.items() if size == 0x600}

    def snapshot(self, names):
        return {name: bytes(self.uc.mem_read(*self.regions[name])).hex() for name in names}

def run(image, precision, cw, case):
    m = Audit(cw, image)
    if case.startswith("upgrade"):
        owner, parent, model = m.actor("owner"), m.actor("parent"), m.actor("model")
        m.put(parent+0x4e0, model)
        for index, offset in enumerate(range(0x50, 0x68, 4)):
            m.put(model+offset, 0x41000000+index)
        if case == "upgrade_existing_model":
            m.put(owner+0x4e0, owner)
        before = m.protected()
        m.call(0x45fae0, [parent, int(case == "upgrade_indirect")], owner)
        if case == "upgrade_duplicate":
            m.call(0x45fae0, [parent, 0], owner)
        assert m.protected() == before
        allowed = {0x4d0, 0x4d4, 0x4c8, 0x4cc, 0x4e0, *range(0x50, 0x68, 4)}
        assert all(name == "owner" and off in allowed for name, off, size, value in m.writes)
    elif case.startswith("affiliation"):
        owner, other = m.actor("owner"), m.actor("other")
        before = m.protected()
        m.call(0x45fbb0, [other], owner)
        if case == "affiliation_duplicate":
            m.call(0x45fbb0, [other], owner)
        assert m.protected() == before
        assert all((name == "owner" and off in (0x488, 0x48c)) or
                   (name == "other" and off in (0x444, 0x448))
                   for name, off, size, value in m.writes)
    elif case == "selection_layout_clone_append":
        owner, member = m.actor("owner"), m.actor("member")
        layout = m.fixture("original_layout", 0x18)
        data = m.fixture("original_layout_data", 0x40)
        parameters = m.fixture("original_parameters", 12)
        wrapper = m.fixture("new_wrapper", 8)
        m.put(layout+8, data); m.put(layout+0xc, parameters)
        m.put(layout+0x10, 3); m.put(layout+0x14, 3); m.put(data+0x20, 3)
        m.put_words(parameters, [member, 0, member])
        before = m.protected()
        immutable = m.snapshot(["original_layout", "original_layout_data", "original_parameters"])
        m.call(0x492c20, [member, layout], wrapper)
        m.call(0x45f7b0, [wrapper], owner)
        assert m.protected() == before
        assert m.snapshot(immutable) == immutable
        assert all(name == "new_wrapper" or (name == "owner" and off in (0x490, 0x494))
                   for name, off, size, value in m.writes)
    elif case.startswith("weighted_choice"):
        actor = m.actor("actor")
        group = m.fixture("placement_group", 0x30)
        before = m.protected()
        m.call(0x471a10, [actor], group)
        if case == "weighted_choice_duplicate":
            m.call(0x471a10, [actor], group)
        assert m.protected() == before
        assert all(name == "placement_group" and off in (0x18, 0x1c)
                   for name, off, size, value in m.writes)
    else:
        raise ValueError(case)
    assert not m.null_accesses
    return {"image": image, "precision": precision, "case": case, "writes": m.writes,
            "protected_cv_fields_unchanged": True, "executed_addresses": sorted(m.visited)}

def main():
    rows = []
    for image in ("original", "v14"):
        for precision, cw in MODES:
            for case in ("upgrade_direct", "upgrade_indirect", "upgrade_existing_model", "upgrade_duplicate",
                         "affiliation", "affiliation_duplicate", "selection_layout_clone_append",
                         "weighted_choice", "weighted_choice_duplicate"):
                rows.append(run(image, precision, cw, case))
    doc = {"schema": 1, "scope": __doc__, "original_sha256": sha(ORIGINAL.data),
           "v14_sha256": sha(ACCEPTED.data), "generator_sha256": sha(Path(__file__).read_bytes()),
           "protected_actor_offsets": PROTECTED, "cases": rows,
           "function_windows": pin({"upgrade_leaf": (0x45fae0, 0x45fbaa),
             "affiliation_leaf": (0x45fbb0, 0x45fc3b), "layout_wrapper": (0x492c20, 0x492c35),
             "layout_owner": (0x492b90, 0x492c17), "layout_clone_ctor": (0x4c2290, 0x4c2310),
             "layout_parameter_copy": (0x4c21e0, 0x4c2242), "selection_append_thunk": (0x45f7b0, 0x45f7c3),
             "list_append": (0x45f240, 0x45f2a3), "weighted_choice_thunk": (0x471a10, 0x471a25),
             "weighted_list_append": (0x4b6ba0, 0x4b6bed)}),
           "limitations": ["Synthetic typed nonalias objects; not a full shipped source graph run.",
             "Successful allocator seam only; heap failure, callback reentry and SEH failure are not proved.",
             "Parent prepasses and their potential prerequisite failures still require execution or an explicit dependency-slice receipt.",
             "Protected ActorData+64 is deliberately excluded because that word belongs to copied Object shape; AbilityDefinition+64 is a distinct typed object."]}
    path = HERE / "native-prepass-write-audit.json"
    path.write_text(json.dumps(doc, indent=2) + "\n")
    print(json.dumps({"cases": len(rows), "sha256": sha(path.read_bytes())}))

if __name__ == "__main__":
    main()
