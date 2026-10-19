"""ActorData+534 end to end under Unicorn, ORIGINAL AA.exe code only:

1. 4624B0 constructor slice 462D3B..462D7B (+530..+554; ebx is the 4624E1 zero).
2. 464BD0 MobileDeployment block 46852F..4686D9 on each ActorData in load
   order (5B4410 child find, 5B54D0 String, 5B5130 Fixed, 5B4D80 Point2 are
   typed-reader seams answering the case's source values; 6F7810 bump new;
   473AD0/4012C0 and the CString release run natively).
3. 475230 deferred walk -> 461840 -> 4605B0 (4BDBA0 is the identity/Find seam).
4. 45DD80 (ActorData vtable A05540 slot 14) on every ActorData with real
   vtables, so a nonnull +534 re-enters the target's own 45DD80 with mode 0.
   Children are empty; 4288B0 FX-list requests are recorded seams.
410C40 data errors stop the case (native LoadData aborts); prior writes stay.
Default compares the fixture; --write regenerates it. The shipped case input is
exported by aa-runtime actor_deployment_534_tests (AA_EXPORT_DEPLOYMENT_INPUT).
"""
from pathlib import Path
import sys, json, struct, hashlib, math
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-conversion'))
import native_machine as nm
from native_machine import Machine, P, V, VISITED
from unicorn.x86_const import *
H = lambda b: hashlib.sha256(b).hexdigest()
f = lambda x: struct.unpack('<I', struct.pack('<f', x))[0]
EPS = 0x38d1b717
NEXT_EPS = EPS + 1
QNAN = 0x7fc00001
INF = 0x7f800000
NEG0 = 0x80000000
VTABLE = 0xa05540
WORDS = [0x534, 0x538, 0x53c, 0x540, 0x544, 0x548, 0x54c, 0x550, 0x554]
FRAME = 0x2f07000


def comp(ids='', health=None, pl=None, ul=None, off=(0, 0), ori=None):
    return {'unpack_to_actor_ids': ids, 'pack_health_min': f(.5) if health is None else health,
            'packing_length': f(10) if pl is None else pl, 'unpacking_length': f(10) if ul is None else ul,
            'packed_position_offset': list(off), 'packed_orientation': list(ori or (f(1), 0))}


def actor(id, mask=1, radius=None, component=None):
    return {'id': id, 'mask_before': mask, 'soft_radius': f(4) if radius is None else radius,
            'component': component}


def run(c):
    m = Machine(c['precision'])
    g = m.alloc(0x900)
    m.w(0xb36530, g)
    defs, owner = {}, {}
    for a in c['actors']:
        p = m.alloc(0x5b0)
        m.w(p, VTABLE)
        m.w(p + 8, m.cstring(a['id']))
        m.w(p + 0x174, a['mask_before'])
        m.w(p + 0x54, a['soft_radius'])
        # 462D3B..462D7B, esi = this, ebx = 4624E1 zero; poison first.
        for o in WORDS:
            m.w(p + o, 0xa5a5a5a5)
        m.u.reg_write(UC_X86_REG_ESI, p)
        m.u.reg_write(UC_X86_REG_EBX, 0)
        m.run(0x462d3b, 0x462d7b)
        assert m.u.reg_read(UC_X86_REG_EIP) == 0x462d7b
        defs[a['id']] = p
        owner[p] = a['id']
    label = lambda p: owner.get(p) if p else None
    reads = {a['id']: [] for a in c['actors']}
    current = {}

    def child(m):  # 5B4410 thiscall(name, flag) ret 8
        key = m.read_wide(m.arg(0))
        assert m.u.reg_read(UC_X86_REG_ECX) == current['reader'] and key == 'MobileDeploymentComponent'
        reads[current['id']].append(['5B4410', key, m.arg(1)])
        m.ret(8, current['component'] if current['source'] is not None else 0)

    def typed(name):
        def seam(m):
            key_arg = 1 if name == '5B4D80' else 0
            key = m.read_wide(m.arg(key_arg))
            assert m.u.reg_read(UC_X86_REG_ECX) == current['component']
            reads[current['id']].append([name, key])
            value = current['source'][key]
            if name == '5B54D0':
                cell = m.alloc(4)
                m.w(cell, m.cstring(value))
                m.ret(4, cell)
            elif name == '5B4D80':
                out = m.arg(0)
                m.w(out, value[0])
                m.w(out + 4, value[1])
                m.ret(8, out)
            else:  # ST0 = the binary32 value through a fresh fld/ret 4 stub
                cell = m.alloc(4)
                m.w(cell, value)
                stub = m.alloc(16)
                m.u.mem_write(stub, b'\xd9\x05' + struct.pack('<I', cell) + b'\xc2\x04\x00')
                m.u.reg_write(UC_X86_REG_EIP, stub)
        return seam
    m.seam(0x5b4410, child)
    for name, va in [('5B54D0', 0x5b54d0), ('5B5130', 0x5b5130), ('5B4D80', 0x5b4d80)]:
        m.seam(va, typed(name))
    m.seam(0x410c40, lambda m: m.throw('DataError', format=m.read_wide(m.arg(0))))
    lookups = []

    def lookup(m):  # 4BDBA0(IDS, category) ret 8
        name = m.read_wide(m.arg(0))
        src = label(m.u.reg_read(UC_X86_REG_ESI))
        lookups.append([src, name, m.arg(1)])
        m.ret(8, defs.get(c.get('aliases', {}).get(name, name), 0))
    m.seam(0x4bdba0, lookup)
    def unexpected(va):
        def seam(m):
            raise RuntimeError(('empty child list reached', hex(va)))
        return seam
    for va in [0x46aec0, 0x46c7a0, 0x4be420]:  # aspects, banner, abilities: all empty
        m.seam(va, unexpected(va))
    trace = []

    def fx(m):  # 4288B0 thiscall(delta, mode) ret 8, ecx = list head
        head = m.u.reg_read(UC_X86_REG_ECX)
        base = max(p for p in owner if p <= head)
        off = head - base
        call = 'viewfx%d' % ((off - 0xbc) // 8) if 0xbc <= off < 0xec else 'actorfx%d' % off
        trace.append({'actor': owner[base], 'call': call, 'args': [m.arg(0), m.arg(1)]})
        m.ret(8, 0)
    m.seam(0x4288b0, fx)
    out = {'error': None, 'reads': reads, 'created': [], 'lookups': lookups, 'requests': []}
    for a in c['actors']:
        p = defs[a['id']]
        current.update(id=a['id'], source=a['component'], reader=m.alloc(4), component=m.alloc(4))
        head = m.r(g + 0x824)
        m.w(FRAME + 0xb0, current['reader'])
        for r, v in [(UC_X86_REG_EBP, p), (UC_X86_REG_ESP, FRAME), (UC_X86_REG_EBX, 0x33333333),
                     (UC_X86_REG_ESI, 0x44444444), (UC_X86_REG_EDI, 0x55555555)]:
            m.u.reg_write(r, v)
        m.run(0x46852f, 0x4686d9)
        if m.r(g + 0x824) != head:
            ref = m.r(g + 0x824)
            assert m.r(ref) == 0xa0347c and m.r(ref + 8) == head and m.r(ref + 12) == p
            out['created'].append([a['id'], m.read_wide(m.r(ref + 4))])
        if m.outcome is not None:
            out['error'] = {'phase': 'load', 'actor': a['id'], **m.outcome}
            break
        assert m.u.reg_read(UC_X86_REG_EIP) == 0x4686d9 and m.u.reg_read(UC_X86_REG_ESP) == FRAME
    if out['error'] is None:
        m.call(0x475230, g + 0x824, [], 0)
        if m.outcome is not None:
            out['error'] = {'phase': 'deferred', **m.outcome}
        else:
            assert m.r(g + 0x824) == 0
    if out['error'] is None:
        for delta, mode in c['requests']:
            for a in c['actors']:
                trace.clear()
                m.call(0x45dd80, defs[a['id']], [delta, mode], 8)
                assert m.outcome is None
                out['requests'].append({'actor': a['id'], 'args': [delta, mode], 'trace': list(trace)})
    out['actors'] = {a['id']: {'174': m.r(defs[a['id']] + 0x174), '20': m.r(defs[a['id']] + 0x20),
                               '534': label(m.r(defs[a['id']] + 0x534)), '548': label(m.r(defs[a['id']] + 0x548)),
                               **{'%x' % o: m.r(defs[a['id']] + o) for o in WORDS if o not in (0x534, 0x548)}}
                     for a in c['actors']}
    return out


def cases():
    out = []
    PCS = ['Single24', 'Double53', 'Extended64']
    hq = lambda **kw: [actor('building', component=comp()), actor('truck', mask=1, component=comp('building', **kw))]
    std = [[1, 1], [3, 0xffffffff]]
    for pc in PCS:
        add = lambda label, actors, req=std, **kw: out.append(
            {'label': pc + '_' + label, 'precision': pc, 'shipped': False, 'actors': actors, 'requests': req, **kw})
        add('hq_pair', hq())
        add('hq_pair_target_loaded_later', list(reversed(hq())), [[0xffffffff, 0], [2, 7]])
        add('non_mobile_and_pair', [actor('plain', mask=0x40000001)] + hq())
        add('two_trucks_one_building', [actor('building', component=comp(health=f(.25), pl=f(8), ul=f(9))),
                                        actor('truck1', component=comp('building', off=(f(1), f(-2)), ori=(0, f(-1)))),
                                        actor('truck2', component=comp('b_alias', ori=(f(.70710677), f(.70710677))))],
            aliases={'b_alias': 'building'})
        add('chain', [actor('c', component=comp()), actor('b', component=comp('c')), actor('a', component=comp('b'))])
        add('elements', [actor('eb', mask=0x101, component=comp()), actor('et', mask=0x101, component=comp('eb'))])
        for name, ori in [('zero', (0, 0)), ('eps', (EPS, 0)), ('neg_eps_eps', (EPS | 0x80000000, EPS)),
                          ('above_eps', (NEXT_EPS, 0)), ('y_above_eps', (0, NEXT_EPS | 0x80000000)),
                          ('neg_zero', (NEG0, NEG0)), ('nan_x', (QNAN, 0)), ('nan_y', (0, QNAN)),
                          ('inf_x', (INF, 0)), ('down', (0, f(-1)))]:
            add('orientation_' + name, hq(ori=ori))
        add('unknown_target', [actor('truck', component=comp('missing'))])
        add('target_without_body', [actor('building', mask=0, component=comp()), actor('truck', component=comp('building'))])
        add('target_not_mobile', [actor('building'), actor('truck', component=comp('building'))])
        add('element_mismatch', [actor('building', mask=0x101, component=comp()), actor('truck', component=comp('building'))])
        add('outside_soft_shape', hq(off=(f(5), 0)))
        add('soft_shape_edge', [actor('building', radius=f(1), component=comp()),
                                actor('truck', component=comp('building', off=(f(.6), f(.8))))])
        add('building_authored_fields', [actor('building', component=comp(health=f(.75), pl=f(7.5), ul=0x3f8ccccd))])
    shipped = HERE / 'shipped-deployment-input.json'
    if shipped.exists():
        src = json.loads(shipped.read_text())
        aliases = {a['reference']: a['find'] for a in src['actors'] if a['reference'] is not None and a['find'] is not None}
        actors = src['actors']  # exported rows verbatim; run() reads id/mask_before/soft_radius/component
        out.append({'label': src['precision'] + '_shipped', 'precision': src['precision'], 'shipped': True,
                    'actors': actors, 'requests': [[1, 1]], 'aliases': aliases})
    return out


def main():
    rows = [{'input': c, 'output': run(c)} for c in cases()]
    for a, b in VISITED.items():
        assert P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b))), hex(a)
    doc = {'schema': 1, 'passed': True, 'scope': __doc__, 'producer_sha256': H(Path(__file__).read_bytes()),
           'base_machine_sha256': H(Path(nm.__file__).read_bytes()), 'original_sha256': H(P.b), 'v14_sha256': H(V.b),
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())], 'cases': rows}
    p = HERE / 'native-actor-deployment-534.json'
    if '--write' in sys.argv:
        p.write_text(json.dumps(doc, indent=1) + '\n')
    else:
        assert json.loads(p.read_text()) == doc, 'native +534 fixture differs'
    print(json.dumps({'cases': len(rows), 'instructions': len(VISITED), 'sha256': H(p.read_bytes())}))


if __name__ == '__main__':
    main()
