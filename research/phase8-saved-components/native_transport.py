"""Original Transport component (slot 24, 0x60 bytes): constructor 58B9A0 then
saved Load 58C810 on the same object, under Unicorn (native_machine.py).

Native code executed: 58B9A0 (4CD440 ResourceVector x2, 45F180, CString
empty), 58C810, 8589B0 version reader, 4C2E60 (ActorData reference), 4C3050
(LayoutData reference), 4C2110 layout ctor, 430240, 4849F0, the layout's
virtual Load 4C3180, ResourceVector virtual Load 4CD3F0 -> 4CD0F0.
Seams (recorded): stream read, 41D290 string read, 6F7810/6F7430, registry
lookups 4BDBA0 (B39F58 class 0x2B) and 463FA0 (B3AB00), and the throws
41DFB0/858AD0. All allocations go through 6F7810.
"""
from pathlib import Path
import sys, json, struct, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H, f32

W = lambda *v: struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])


def S(text):
    raw = text.encode('utf-16-le')
    return struct.pack('<H', len(text)) + raw


ACTORS = {'usa_infantry': 0, 'ger_tank_light': 0, 'gbr_halftrack': 0, 'regiment_usa_infantry': 0}
LAYOUTS = {'regiment_infantry': 3, 'regiment_hq': 0, 'herd_single': 1}


class TransportMachine(Machine):
    def __init__(self, case):
        super().__init__(bytes.fromhex(case['payload']), case['precision'])
        self.case = case
        self.defs = {}
        for name in ACTORS:
            d = self.alloc(0x400)
            self.w(d + 8, self.cstring(name))
            self.defs[name] = d
            self.labels[d] = 'actor:' + name
        self.layouts = {}
        for name, count in LAYOUTS.items():
            d = self.alloc(0x100)
            self.w(d + 8, self.cstring(name))
            self.w(d + 0x20, count)
            self.layouts[name] = d
            self.labels[d] = 'layout:' + name
        self.w(G_THING := 0xb39f58, self.alloc(0x40))
        self.w(0xb3ab00, self.alloc(0x40))
        reg = self.alloc(0x40)
        self.w(reg + 0x28, case['resource_count'])
        self.w(0xb3a660, reg)

        def thing_lookup(m):  # 4BDBA0(CString, class) thiscall
            name = m.cstring_text(m.arg(0))
            assert m.arg(1) == 0x2b
            p = m.defs.get(name, 0)
            m.record('4BDBA0', ids=name, found=bool(p))
            m.ret(8, p)
        self.seam(0x4bdba0, thing_lookup)

        def layout_lookup(m):  # 463FA0(CString) thiscall
            name = m.cstring_text(m.arg(0))
            p = m.layouts.get(name, 0)
            m.record('463FA0', ids=name, found=bool(p))
            m.ret(4, p)
        self.seam(0x463fa0, layout_lookup)

    def label(self, p):
        return None if p == 0 else self.labels.get(p, hex(p))

    def vector(self, base):
        ptr, count, cap = self.r(base), self.r(base + 4), self.r(base + 8)
        n = count if count < 0x80000000 else 0
        return {'count': count if count < 0x80000000 else count - (1 << 32), 'capacity': cap,
                'values': [self.r(ptr + 4 * i) for i in range(n)]}

    def state(self, t):
        layout = self.r(t + 0x2c)
        lay = None
        if layout:
            params = self.vector(layout + 0xc)
            lay = {'refcount': self.r(layout + 4), 'definition': self.label(self.r(layout + 8)),
                   'parameters': [self.label(v) for v in params['values']], 'parameter_count': params['count']}
        return {
            'owner': self.r(t + 4) == self.owner,
            'cost': self.vector(t + 0xc), 'upkeep': self.vector(t + 0x1c),
            'cargo_definition': self.label(self.r(t + 0x28)), 'cargo_layout': lay,
            'word_30': [self.r(t + 0x30), self.r(t + 0x34)],
            'byte_38': self.b(t + 0x38), 'byte_39': self.b(t + 0x39),
            'word_3c': self.r(t + 0x3c), 'word_40': self.r(t + 0x40), 'word_44': self.r(t + 0x44),
            'word_48': self.r(t + 0x48), 'byte_4c': self.b(t + 0x4c),
            'words_50': [self.r(t + 0x50), self.r(t + 0x54)],
            'text_58': self.cstring_text(self.r(t + 0x58)), 'word_5c': self.r(t + 0x5c),
        }

    def run(self):
        self.owner = self.alloc(0x200)
        t = self.alloc(0x60, 0xcd)
        self.call(0x58b9a0, t, [self.owner])
        after_ctor = self.state(t)
        self.call(0x58c810, t, [self.ctx])
        out = {'after_ctor': after_ctor, 'consumed': self.at, 'reads': self.reads,
               'calls': self.calls, 'outcome': self.outcome}
        if self.outcome is None or self.outcome['kind'] != 'NullDereference':
            out['state'] = self.state(t)
        return out


def cases():
    rng = random.Random(0x58c810)
    rows = []

    def rv(count=None, version=0, values=None):
        if values is None:
            values = [f32(rng.choice([0.0, 1.5, 100.0, -3.25, 1e-3])) for _ in range(count)]
        return W(version, len(values) if count is None else count) + W(*values)

    def body(version, ref='', flag=0, layout=None, legacy=None, b4c=0, text='', tail_ok=True,
             rv_a=None, rv_b=None):
        p = W(version) + S(ref) + bytes([flag])
        if flag:
            p += S(layout[0]) + W(layout[1]) + S(layout[2]) + W(len(layout[3])) + b''.join(S(x) for x in layout[3]) \
                if isinstance(layout, tuple) else layout
        p += W(rng.getrandbits(32), rng.getrandbits(32)) + bytes([rng.getrandbits(8), rng.getrandbits(8)])
        p += W(rng.getrandbits(32), rng.getrandbits(32))
        if version == 0:
            items = legacy if legacy is not None else []
            p += W(len(items) if not isinstance(items, int) else items) + (W(*items) if isinstance(items, list) else b'')
        if version >= 4:
            p += W(rng.getrandbits(32))
        p += W(rng.getrandbits(32))
        p += rv_a if rv_a is not None else rv(5)
        p += rv_b if rv_b is not None else rv(5)
        if version >= 3:
            p += bytes([b4c])
            if b4c:
                p += W(rng.getrandbits(32), rng.getrandbits(32))
        if version >= 5:
            p += S(text)
        if version >= 6:
            p += W(rng.getrandbits(32))
        return p

    good_layout = ('regiment_infantry', 0, 'regiment_infantry', ['usa_infantry', '', 'ger_tank_light'])
    for pc in ['Single24', 'Double53', 'Extended64']:
        for version in range(7):
            for resource_count in (0, 5, 7):
                rows.append((pc, resource_count, body(version)))
                rows.append((pc, resource_count, body(version, 'regiment_usa_infantry', 1, good_layout, b4c=1,
                                                      text='uai_sleep')))
    pc = 'Double53'
    # layout variants: second (inner) LayoutData differs; count mismatch fatal; bad inner version
    rows.append((pc, 5, body(6, 'usa_infantry', 0x80, ('herd_single', 0, 'herd_single', ['gbr_halftrack']))))
    rows.append((pc, 5, body(6, 'usa_infantry', 1, ('regiment_infantry', 0, 'herd_single', ['usa_infantry']))))
    rows.append((pc, 5, body(6, 'usa_infantry', 1, ('regiment_infantry', 0, 'regiment_hq', []))))
    rows.append((pc, 5, body(6, 'usa_infantry', 1, ('regiment_infantry', 0, 'regiment_infantry', ['usa_infantry']))))
    rows.append((pc, 5, body(6, 'usa_infantry', 1, ('regiment_infantry', 1, 'regiment_infantry', []))))
    rows.append((pc, 5, body(6, 'usa_infantry', 1, ('regiment_infantry', 0, 'regiment_infantry', ['', 'nope', 'usa_infantry']))))
    rows.append((pc, 5, body(6, '', 1, ('nope_layout', 0, 'regiment_infantry', []))))
    rows.append((pc, 5, body(6, '', 1, ('', 0, 'regiment_infantry', []))))       # 4C2110(null): native null read
    rows.append((pc, 5, body(6, '', 1, ('regiment_hq', 0, '', []))))             # 4C3180 null def: native null read
    # references and legacy list
    rows.append((pc, 5, body(6, 'unknown_actor')))
    rows.append((pc, 5, W(7)))                                                  # bad data version
    for legacy in ([], [7], [1, 2, 3], -1, 0x80000000):
        rows.append((pc, 5, body(0, legacy=legacy)))
    # resource vectors
    rows.append((pc, 5, body(4, rv_a=rv(0), rv_b=rv(9))))
    rows.append((pc, 5, body(4, rv_a=W(1, 0))))                                 # RV version 1: bad data version
    rows.append((pc, 5, body(4, rv_a=W(0, 0xffffffff))))                        # negative count
    rows.append((pc, 7, body(6, rv_a=rv(7), rv_b=rv(7), b4c=0, text='')))
    rows.append((pc, 5, body(5, b4c=0x40, text='x' * 40)))
    # truncation at every prefix of one full payload
    full = body(6, 'regiment_usa_infantry', 1, good_layout, b4c=1, text='ab')
    for cut in range(0, len(full), 3):
        rows.append((pc, 5, full[:cut]))
    out = [{'precision': a, 'resource_count': b, 'payload': c.hex()} for a, b, c in rows]
    return out + real_cases()


def real_cases():
    """Every Transport payload of the 80 original maps (real-payloads.json,
    extracted by payload_dump): component start to the end of the actor tail.
    The constructor's resource count is the payload's own cost-vector count."""
    path = HERE / 'real-payloads.json'
    rows = []
    for t in json.loads(path.read_text())['transports']:
        b = bytes.fromhex(t['payload'])
        at = 4
        n = struct.unpack_from('<H', b, at)[0]
        at += 2 + 2 * n
        assert b[at] == 0  # no carried layout in any shipped map
        at += 1 + 8 + 2 + 8
        version = struct.unpack_from('<I', b, 0)[0]
        if version == 0:
            at += 4 + 4 * struct.unpack_from('<i', b, at)[0]
        if version >= 4:
            at += 4
        at += 4 + 4
        rows.append({'precision': 'Double53', 'resource_count': struct.unpack_from('<I', b, at)[0],
                     'payload': t['payload'], 'source': '%s#%d %s' % (t['map'], t['ordinal'], t['definition'])})
    return rows


def main():
    out = []
    for c in cases():
        m = TransportMachine(c)
        native = m.run()
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__,
           'definitions': {'actor_data': sorted(ACTORS), 'layout_data_parameter_counts': LAYOUTS},
           'visited_instructions': len(VISITED), 'cases': out}
    path = HERE / 'native-transport.json'
    path.write_text(json.dumps(doc, indent=1))
    kinds = {}
    for row in out:
        k = (row['native']['outcome'] or {}).get('kind', 'ok')
        kinds[k] = kinds.get(k, 0) + 1
    print(json.dumps({'cases': len(out), 'outcomes': kinds, 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
