"""HQ component (slot 30): constructor 574490 and saved Load 574040 under
Unicorn (native_machine.py), on synthetic owner/Kingdom/nation objects.

Executed natively: 574490 (4DB020 base, 529C30, 430240 resize of the +14
slot vector to ThingData+530 (1 when <= 0)), 574040 (50BCD0 Updatable with
the GWorld+284 table and 50B250, 8589B0 version <= 3, the +10 badge index
check against [Kingdom+234]+40 through the owner's virtual +108 (GActor
4D1E80 = +E8), version 1 legacy list 4D1AF0/4D0890 (read and freed), item
records 5737E0 with the 573750 slot iterator and 766DE0 overflow vector).
Seams (recorded): owner virtual +24 (GActor 4D99D0, the display name) and
the 404F60 log; stream reads, allocation (0xA5 fill) and free from
native_machine.py.
"""
from pathlib import Path
import sys, json, struct
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H
from unicorn.x86_const import UC_X86_REG_ECX

GACTOR_VT = 0xa273f0


def signed(v):
    return v - (1 << 32) if v >> 31 else v


class HqMachine(Machine):
    def __init__(self, c):
        super().__init__(bytes.fromhex(c['payload']))
        self.c = c
        world = self.alloc(0x400)
        self.w(0xb36534, world)
        table = self.alloc(16)
        slots = self.alloc(4 * (c['table_max'] + 2))
        for i in range(c['table_max'] + 2):
            self.w(slots + 4 * i, 0xdead0000 + i if i in c['occupied'] else 0)
        self.w(table, c['table_max'])
        self.w(table + 4, slots)
        self.w(world + 0x284, table)
        self.table_slots = slots
        self.owner = self.alloc(0x200)
        self.w(self.owner, GACTOR_VT)
        d = self.alloc(0x600)
        self.w(d + 0x530, c['slots_530'] & 0xffffffff)
        self.w(self.owner + 4, d)
        if c['kingdom'] is not None:
            k = self.alloc(0x400)
            self.w(self.owner + 0xe8, k)
            if c['kingdom'].get('badges') is not None:
                f = self.alloc(0x80)
                self.w(f + 0x10, self.cstring(c['kingdom']['nation']))
                self.w(f + 0x40, c['kingdom']['badges'] & 0xffffffff)
                self.w(k + 0x234, f)

        def name(m):  # 4D99D0(&out) thiscall: GActor display name
            assert m.u.reg_read(UC_X86_REG_ECX) == m.owner
            m.w(m.arg(0), m.cstring(m.c['owner_name']))
            m.record('owner_name')
            m.ret(4, m.arg(0))
        self.seam(0x4d99d0, name)

        def log(m):  # 404F60(B38B70, file, line, function, fmt, ...)
            fmt = m.read_wide(m.arg(4))
            e = m.record('404F60', line=m.arg(2), function=m.read_wide(m.arg(3)), format=fmt,
                         args=m.fmt_args(fmt, 5))
            m.logs.append(e)
            m.ret(0)
        self.seam(0x404f60, log)

    def item(self, p):
        return None if p == 0 else [self.r(p), self.r(p + 4), self.r(p + 8)]

    def vector(self, at):
        data, n, cap = self.r(at), self.r(at + 4), self.r(at + 8)
        return {'items': [self.item(self.r(data + 4 * i)) for i in range(n)] if data else [],
                'count': n, 'capacity': cap, 'data_null': data == 0}

    def state(self, hq):
        return {'index': signed(self.r(hq + 0x10)), 'slots': self.vector(hq + 0x14),
                'pending': self.vector(hq + 0x20)}

    def run(self):
        hq = self.alloc(0x2c, 0xa5)
        self.call(0x574490, hq, [self.owner])
        after_ctor = self.state(hq)
        assert self.r(hq) == 0xa3ca4c and self.r(hq + 8) == 0xa3ca24 and self.r(hq + 4) == self.owner
        self.call(0x574040, hq, [self.ctx])
        out = {'after_ctor': after_ctor, 'outcome': self.outcome, 'consumed': self.at, 'calls': self.calls}
        if self.outcome is None or self.outcome['kind'] not in ('NullDereference',):
            out['state'] = self.state(hq)
        uid = self.c['updatable_id']
        if uid is not None and 0 <= uid <= self.c['table_max'] + 1:
            v = self.r(self.table_slots + 4 * uid)
            out['table_slot'] = 'hq+8' if v == hq + 8 else v
        return out


def w32(v):
    return struct.pack('<I', v & 0xffffffff)


def payload(updatable=(1, 3), version=3, index=-1, legacy=None, items=(), count=None):
    b = w32(updatable[0])
    if updatable[0] >= 1:
        b += w32(updatable[1])
    b += w32(version) + w32(index)
    if version == 1:
        vals = legacy or []
        b += w32(len(vals) if count is None else count) + b''.join(w32(x) for x in vals)
        return b
    b += w32(len(items) if count is None else count)
    for it in items:
        if it is None:
            b += b'\0'
            continue
        b += b'\1'
        if version >= 3:
            b += w32(it[0]) + w32(it[1]) + w32(it[2])
        else:
            flag, word = it
            b += bytes([flag]) + w32(word)
    return b


def cases():
    rows = []
    base = dict(table_max=50, occupied=[7], slots_530=3, owner_name='usa_hq',
                kingdom={'badges': 4, 'nation': 'USA'})

    def add(name, p, **kw):
        c = dict(base)
        c.update(kw)
        c['name'] = name
        c['payload'] = p.hex()
        upd = kw.get('_upd', (1, 3))
        c['updatable_id'] = upd[1] if upd[0] >= 1 else None
        c.pop('_upd', None)
        rows.append(c)

    add('empty_v3', payload())
    add('slots_filled', payload(index=2, items=[(0, 5, 9), (1, 0, -1), (2, 7, 7)]))
    add('kind3_and_overflow', payload(index=0, items=[(3, 1, 1), (0, 2, 2), None, (1, 3, 3), (2, 4, 4), (0, 5, 5)]))
    add('count_negative', payload(items=[], count=-3))
    add('slots_530_zero', payload(items=[(0, 1, 2), (0, 3, 4)]), slots_530=0)
    add('slots_530_negative', payload(items=[(0, 1, 2)]), slots_530=-2)
    add('slots_530_five', payload(items=[(4, 1, 2), None, (5, 3, 4)]), slots_530=5)
    add('badge_out_of_range', payload(index=4))
    add('badge_far_out_of_range', payload(index=0x7fffffff))
    add('badge_negative_kept', payload(index=-7))
    add('badge_no_nation', payload(index=9), kingdom={'badges': None})
    add('badge_no_kingdom', payload(index=1), kingdom=None)
    add('no_kingdom_index_unset', payload(index=-1, items=[(0, 1, 1)]), kingdom=None)
    add('version0', payload(version=0, items=[(1, 11), (0, 22), None, (5, 33)]))
    add('version2', payload(version=2, index=1, items=[(1, 11), (1, 12), (1, 13), (1, 14), (0, 15)]))
    add('version1_legacy', payload(version=1, index=2, legacy=[5, 6, 7]))
    add('version1_legacy_negative', payload(version=1, legacy=[], count=-2))
    add('bad_version', payload(version=4))
    add('updatable_v0', payload(updatable=(0, 0)), _upd=(0, 0))
    add('updatable_negative', payload(updatable=(1, -5)), _upd=(1, -5))
    add('updatable_duplicate', payload(updatable=(1, 7)), _upd=(1, 7))
    add('updatable_above_table', payload(updatable=(1, 51)), _upd=(1, 51))
    add('updatable_at_max', payload(updatable=(1, 50)), _upd=(1, 50))
    add('updatable_bad_version', payload(updatable=(2, 0)), _upd=(2, 0))
    add('truncated_item', payload(items=[(0, 1, 2)])[:-3])
    add('truncated_count', payload()[:-2])
    return rows


def main():
    out = []
    for c in cases():
        m = HqMachine(c)
        out.append({'input': c, 'native': m.run()})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-hq.json'
    path.write_text(json.dumps(doc, indent=1), encoding='utf-8')
    for row in out:
        n = row['native']
        print(row['input']['name'], n['outcome'], n['consumed'], [x['seam'] for x in n['calls']],
              n.get('state', {}).get('index'), n.get('table_slot'))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
