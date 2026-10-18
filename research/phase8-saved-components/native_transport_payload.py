"""Transport payload load 58BA90 (summon effect 4CCA70 and trigger actions)
on a constructed Transport (58B9A0), under Unicorn (native_machine.py).

Native: 58BA90, 4CD440 (two ResourceVectors), 58B2A0 (4CD230 vector copy,
KKC_OrganizationLayout reference swap with the old object's scalar-deleting
destructor 4C2190 when its count reaches 0, +3C filled-slot count), 405B80
(name), 4CCF10 (vector destructors). Seams (recorded): 56F580 upkeep
(writes the case's values into the vector it is given) and 4CF0F0 on the
owner; allocation 6F7810/6F7430.
"""
from pathlib import Path
import sys, json, struct
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H, f32
from unicorn.x86_const import UC_X86_REG_ECX


class PayloadMachine(Machine):
    def __init__(self, c):
        super().__init__(b'')
        self.c = c
        reg = self.alloc(0x40)
        self.w(reg + 0x28, c['resource_count'])
        self.w(0xb3a660, reg)
        self.defs = {}
        for name in ('cargo_regiment', 'inf', 'tank'):
            d = self.alloc(0x400)
            self.w(d + 8, self.cstring(name))
            self.defs[name] = d
            self.labels[d] = name
        self.layout_defs = {}

        def upkeep(m):  # 56F580(out RV, cargo, layout, context) cdecl
            out, cargo, layout, ctx = m.arg(0), m.arg(1), m.arg(2), m.arg(3)
            values = m.c['upkeep']
            buf = m.alloc(4 * max(1, len(values)))
            for i, v in enumerate(values):
                m.w(buf + 4 * i, v)
            m.w(out + 4, buf)
            m.w(out + 8, len(values))
            m.w(out + 0xc, max(1, len(values)))
            m.record('56F580', cargo=m.labels.get(cargo), layout=None if layout == 0 else 'layout', context=ctx)
            m.ret(0)
        self.seam(0x56f580, upkeep)

        def refresh(m):  # 4CF0F0 thiscall on the owner
            m.record('4CF0F0', this='owner' if m.u.reg_read(UC_X86_REG_ECX) == m.owner else 'other')
            m.ret(0)
        self.seam(0x4cf0f0, refresh)

    def layout(self, spec, refcount):
        name, groups, params = spec
        d = self.alloc(0x80)
        self.w(d + 8, self.cstring(name))
        self.w(d + 0x20, len(params))
        g = self.alloc(4 * max(1, len(groups)))
        for i, x in enumerate(groups):
            self.w(g + 4 * i, x)
        self.w(d + 0x28, g)
        self.w(d + 0x2c, len(groups))
        lay = self.alloc(0x18)
        self.w(lay, 0xa232b8)
        self.w(lay + 4, refcount)
        self.w(lay + 8, d)
        pa = self.alloc(4 * max(1, len(params)))
        for i, x in enumerate(params):
            self.w(pa + 4 * i, self.defs[x] if x else 0)
        self.w(lay + 0xc, pa)
        self.w(lay + 0x10, len(params))
        self.w(lay + 0x14, len(params))
        self.labels[d] = 'layoutdata:' + name
        return lay

    def vector(self, base):
        ptr, count = self.r(base), self.r(base + 4)
        return [self.r(ptr + 4 * i) for i in range(count)]

    def run(self):
        c = self.c
        self.owner = self.alloc(0x200)
        t = self.alloc(0x60, 0xcd)
        self.call(0x58b9a0, t, [self.owner])
        old = None
        if c['old_layout']:
            old = self.layout(c['old_layout'][0], c['old_layout'][1])
            self.w(t + 0x2c, old)
        lay = self.layout(c['layout'], 1) if c['layout'] else 0
        name_obj = self.alloc(4)
        self.w(name_obj, self.cstring(c['name']))
        args = [self.defs['cargo_regiment'], name_obj, lay, c['start'][0], c['start'][1], c['drop'][0], c['drop'][1]]
        r = self.call(0x58ba90, t, args) & 0xff
        cur = self.r(t + 0x2c)
        layout_state = None
        if cur:
            pa, n = self.r(cur + 0xc), self.r(cur + 0x10)
            layout_state = {'same_object': cur == lay, 'refcount': self.r(cur + 4),
                            'definition': self.labels.get(self.r(cur + 8)),
                            'parameters': [self.labels.get(self.r(pa + 4 * i)) for i in range(n)]}
        return {'result': r, 'calls': self.calls, 'old_refcount': None if old is None else self.r(old + 4),
                'state': {'cost': self.vector(t + 0xc), 'upkeep': self.vector(t + 0x1c),
                          'cargo_definition': self.labels.get(self.r(t + 0x28)), 'cargo_layout': layout_state,
                          'word_30': [self.r(t + 0x30), self.r(t + 0x34)], 'byte_38': self.b(t + 0x38),
                          'byte_39': self.b(t + 0x39), 'word_3c': self.r(t + 0x3c), 'word_40': self.r(t + 0x40),
                          'word_44': self.r(t + 0x44), 'word_48': self.r(t + 0x48), 'byte_4c': self.b(t + 0x4c),
                          'words_50': [self.r(t + 0x50), self.r(t + 0x54)],
                          'text_58': self.cstring_text(self.r(t + 0x58)), 'word_5c': self.r(t + 0x5c)}}


def cases():
    rows = []
    base = dict(resource_count=5, name='', layout=None, old_layout=None, upkeep=[f32(1.5), 0, f32(2.25), 0, f32(10)],
                start=[f32(10.0), f32(-4.5)], drop=[f32(100.25), f32(55.0)])
    def add(**kw):
        c = dict(base)
        c.update(kw)
        rows.append(c)
    add()
    add(name='Reinforcements', resource_count=7, upkeep=[f32(i + 0.5) for i in range(7)])
    add(resource_count=0, upkeep=[])
    add(layout=('lay_a', [0, 0, 1, 2], ['inf', None, 'tank']))
    add(layout=('lay_b', [1, 1, 1], ['inf', None]))
    add(layout=('lay_c', [], []))
    add(layout=('lay_a', [0, 0, 1, 2], ['inf', 'inf', 'tank']), old_layout=(('old', [0], ['inf']), 1))
    add(layout=None, old_layout=(('old', [0], ['inf']), 2))
    add(layout=('lay_a', [2, 2], [None, None, 'tank']), old_layout=(('old', [0], ['inf']), 3), name='x')
    return rows


def main():
    out = []
    for c in cases():
        m = PayloadMachine(c)
        out.append({'input': c, 'native': m.run()})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-transport-payload.json'
    path.write_text(json.dumps(doc, indent=1), encoding='utf-8')
    for row in out:
        print(row['native']['result'], row['native']['state']['word_3c'], row['native']['old_refcount'],
              row['native']['state']['cargo_layout'], [x['seam'] for x in row['native']['calls']])
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
