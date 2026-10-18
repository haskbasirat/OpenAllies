"""Transport component requirement: vtable A40100 +20 = 7C7840, executed by
the original KKC_GActor::ValidateComponents 4CFB00 over synthetic actors
whose component table (+60, 31 slots) holds objects with the ORIGINAL
component vtables (from research/phase7-actors/component-dispatch.json).
Seam: the 404ED0 log (recorded)."""
from pathlib import Path
import sys, json
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H

DISPATCH = json.loads((HERE.parents[0] / 'phase7-actors/component-dispatch.json').read_text())
VTABLES = {}
for row in DISPATCH['rows']:
    if row['condition'] == 'default':
        VTABLES.setdefault(row['slot'], int(row['vtable'], 16))


class Validate(Machine):
    def __init__(self, slots):
        super().__init__(b'')
        self.actor = self.alloc(0x200)
        d = self.alloc(0x40)
        self.w(d + 8, self.cstring('transport_test'))
        self.w(self.actor + 4, d)
        for s in slots:
            c = self.alloc(0x80)
            self.w(c, VTABLES[s])
            self.w(self.actor + 0x60 + 4 * s, c)

        def log(m):  # 404ED0(B38B70, file, line, function, fmt, ...) cdecl
            fmt = m.read_wide(m.arg(4))
            m.record('404ED0', line=m.arg(2), format=fmt, args=m.fmt_args(fmt, 5))
            m.ret(0)
        self.seam(0x404ed0, log)


def main():
    rows = []
    for slots in ([24], [3, 24], [0, 3, 24], [0, 2, 24], [3], [0, 2, 3, 5, 6, 24], [17, 24], [3, 6, 17, 24]):
        m = Validate(slots)
        r = m.call(0x4cfb00, m.actor, [])
        assert m.outcome is None
        rows.append({'slots': slots, 'result': r & 0xff, 'calls': m.calls})
    entry = P.range(0xa40100 + 0x20, 4)
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__,
           'transport_vtable_20': hex(int.from_bytes(entry, 'little')),
           'requirement_of_7c7840': 3, 'cases': rows}
    assert doc['transport_vtable_20'] == '0x7c7840'
    path = HERE / 'native-transport-requirement.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(rows), 'results': [r['result'] for r in rows], 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
