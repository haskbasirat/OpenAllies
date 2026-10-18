"""4EE560(retain=false) ->4CF210,4EA0F0,553830; stop BEFORE4EFEC0.

Actual stopped first Kingdom fields plus labelled ordinary class2/3 probes.
Both native images/all3PC. Only actual8-byte node heap transports are supplied;
global identity retirement is an explicit required boundary, not fake success.
"""
from pathlib import Path
import sys, json, hashlib, copy
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/skirmish-20261016-load-spatial"))
import native_load_spatial as source
h = source.h
from unicorn.x86_const import UC_X86_REG_EIP
H = lambda b: hashlib.sha256(b).hexdigest()

class Machine(source.Machine):
    def __init__(self, case, image):
        super().__init__(case, image)
        self.descriptors, self.boundary = [], False
    def _hook(self, u, pc, size, user):
        if getattr(self, "removing", False) and pc in [0x4cf210, 0x4ea0f0]:
            self.descriptors.append("blocking" if pc == 0x4cf210 else "soft")
        super()._hook(u, pc, size, user)
    def remove(self, present):
        if present:
            super().run()
        before = self.snapshot()
        before["trace"] = []
        self.trace, self.descriptors = [], []
        def stop(_):
            self.boundary = True
            self.u.emu_stop()
        self.seams[0x4efec0] = stop
        h.VISITED.clear()
        self.running = self.removing = True
        try:
            self.call(0x4ee560, self.obj, [0])
        except AssertionError:
            assert self.boundary and self.u.reg_read(UC_X86_REG_EIP) == 0x4efec0
        self.running = self.removing = False
        assert self.boundary
        return dict(before=before, after=self.snapshot(), descriptors=self.descriptors,
            required_boundary="4EFEC0", global_identity_still_present=self.r32(self.ids+0x20004+(self.member["reference"]&65535)*4)==self.obj), dict(h.VISITED)

def main():
    inputs = ROOT / "research/skirmish-20261016-load-spatial/actual-first-kingdom.json"
    actual = json.loads(inputs.read_text())
    rows, pins = [], {}
    for captured in actual["cases"]:
        for cls in [4, 2, 3]:
            case = copy.deepcopy(captured)
            case["native_class"] = cls
            for present in [False, True]:
                a, pa = Machine(case, h.P).remove(present)
                b, pb = Machine(case, h.V).remove(present)
                assert a == b
                assert a["descriptors"] == (["blocking", "soft"] if present else [])
                assert not a["after"]["tiles"]["occupied"]
                pins.update(pa); pins.update(pb)
                rows.append(dict(precision=case["precision"], native_class=cls,
                    scope="actual stopped Kingdom" if cls==4 else "labelled class2/3 vtable probe",
                    member_present=present, expected=a))
    intercepted = set(pins) & {0x6f7430, 0x4efec0}
    doc = dict(schema=1, scope=__doc__, rows=rows, original_sha256=h.ORIGINAL_SHA,
        v14_sha256=h.V14_SHA, input_sha256=H(inputs.read_bytes()),
        address_byte_pins=[dict(va=hex(a), bytes=b) for a,b in sorted(pins.items())],
        executed_instruction_addresses=len(pins)-len(intercepted),
        intercepted_entries=[hex(a) for a in sorted(intercepted)])
    path = HERE / "native-teardown.json"
    if "--write" in sys.argv:
        path.write_text(json.dumps(doc, indent=2)+"\n")
    else:
        assert json.loads(path.read_text()) == doc
    print(json.dumps(dict(rows=len(rows), runs=len(rows)*2, pins=len(pins),
        executed=len(pins)-len(intercepted), intercepted=len(intercepted), sha256=H(path.read_bytes()))))

if __name__ == "__main__":
    main()
