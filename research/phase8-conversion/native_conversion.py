"""Class-changed record conversion oracle: the ORIGINAL AA.exe code under
Unicorn (native_machine.py; seams listed in native_machine.SEAM_DOC).

Sections (written to native-conversion.json; `real` to
native-conversion-real.json):
  mismatch  4FAE80 decision table: saved class x definition class x context,
            plus generation mismatch and unknown IDS (null results).
  reads     4FAE80 -> 4EA050/4EF0C0/8589B0/59E300 on class 2 -> 9 records:
            Thing/Object versions, names, variant and scale words, every
            truncation point; at PC24, PC53 and PC64.
  append    a sequence of conversions and skips into one vector (4F9840 and
            its growth 4F60C0/4F5F20/4F4340).
  section   4FC340 over fixture index/data chunks: per-record null results,
            858F50 seeks to record start + index length, construct path seam,
            chunk/version errors.
  replace   4FC600 tail 4FD471..4FD7B1 on vectors built by native 4FAE80:
            A25B1C parameter blocks, definition vf+30/+108/+110 variant rule,
            4F9C10 results and the replaced / can't replace warnings.
  dispatch  original 4F9C10 entry on the A25B1C blocks of `replace` for
            definition classes 0..11 (stops at the class constructor).
  real      every group-1 record of the 27 maps (inventory-round2) through
            native 4FAE80 into one vector per map, then the native tail.

Usage: python native_conversion.py [--real]
"""
import json, struct, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import (Machine, P, VISITED, SEAM_DOC, STACK, Throw, Stop, f32,
                            G_THING_REG, DEF_VTABLES)
from unicorn.x86_const import *
import rmp

MARKER = 0x0BADF00D


def W(*v):
    return struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])


def S(text):
    raw = text.encode('utf-16-le', 'surrogatepass')
    return struct.pack('<H', len(raw) // 2) + raw


def units(text):
    raw = text.encode('utf-16-le', 'surrogatepass')
    return list(struct.unpack('<%dH' % (len(raw) // 2), raw))


def record(ids, klass, saved_id, thing_version=2, name='', word10=0x11223344,
           pos=(f32(1.5), f32(-2.25), f32(0.0)), direction=(f32(0.6), f32(-0.8), f32(0.0)),
           flags=0x12, legacy=0xdeadbeef, obj_version=1, scale=f32(1.0), variant=0, extra=b'\x5a' * 9):
    b = S(ids) + W(klass, saved_id, thing_version) + S(name) + W(word10) + W(*pos) + W(*direction) + W(flags)
    if thing_version == 1:
        b += W(legacy)
    b += W(obj_version, scale)
    if obj_version != 0:
        b += W(variant)
    return b + extra


class Conv(Machine):
    def __init__(self, precision='Double53'):
        super().__init__(precision)
        self.thing_reg = self.alloc(16)
        self.w(G_THING_REG, self.thing_reg)
        self.world = self.alloc(0x200, 0)
        self.defs = {}
        self.labels = {}
        self.construct_consume = 0
        self.create_results = []
        self.blocks = []
        self.seam(0x427bd0, self._lookup)
        self.seam(0x4fb0a9, self._construct)
        self.seam(0x859030, self._open_chunk)
        self.seam(0x858f50, self._seek)

    def define(self, ids, klass, vtable='DynamicDoodadData', variations=0, count=0, label=None):
        d = self.definition(label or ids, klass, vtable, variations, count)
        if ids is not None:
            self.defs[ids] = d
        self.labels[d] = label or ids
        return d

    def _lookup(self, m):  # 427BD0 thiscall(IDS CString data) ret 4
        assert m.u.reg_read(UC_X86_REG_ECX) == m.thing_reg
        ids = ''.join(map(chr, m.cstring_units(m.arg(0))))
        d = m.defs.get(ids, 0)
        m.record('427BD0', ids=ids, found=d != 0)
        m.ret(4, d)

    def _construct(self, m):  # 4FB0A9: class-equal construction path (seam)
        ebp = m.u.reg_read(UC_X86_REG_EBP)
        out = m.r(ebp + 0xc)
        m.record('4FB0A9', saved_class=m.r(ebp - 0x24), consume=m.construct_consume)
        if m.construct_consume:
            m.take(m.construct_consume)
        m.w(out, MARKER)
        m.u.reg_write(UC_X86_REG_EIP, 0x4fb539)

    def _open_chunk(self, m):  # 859030 thiscall(stream, name, required) ret 8
        assert m.u.reg_read(UC_X86_REG_ECX) == m.stream
        name = m.read_wide(m.arg(0))
        found = name in m.chunks
        m.record('859030', chunk=name, required=m.arg(1), found=found)
        if found:
            m.set_payload(m.chunks[name], 0)
        m.ret(8, 1 if found else 0)

    def _seek(self, m):  # 858F50 thiscall(stream, offset) ret 4
        assert m.u.reg_read(UC_X86_REG_ECX) == m.stream
        off = m.arg(0)
        m.record('858F50', offset=off)
        if off > len(m.payload):
            m.throw('SeekPastChunk', offset=off)
        m.at = off
        m._sync()
        m.ret(4)

    # ------------------------------------------------------------- runners
    def reset_case(self):
        self.calls, self.reads, self.outcome = [], [], None
        self.null_accesses.clear()

    def load(self, payload, at, vec, owner=0):
        """4FAE80(this=world, ctx, &out, owner, vec), stream at `at`."""
        self.reset_case()
        self.set_payload(payload, at)
        out = self.alloc(4, 0xa5)
        self.call(0x4fae80, self.world, [self.ctx, out, owner, vec], 0x10)
        return {'out': None if self.outcome else self.r(out), 'consumed': self.at - at,
                'reads': self.reads, 'calls': self.calls, 'outcome': self.outcome}

    def elements(self, vec):
        out = []
        for e in self.vector_records(vec):
            e['definition'] = self.labels.get(e['definition'], hex(e['definition']))
            out.append(e)
        return out

    def tail(self, vec, results, ctx=None):
        """4FC600 4FD471..4FD7B1 on a synthetic frame X (loop esp):
        [X+1C] counter, [X+20] world (4F9C10 this), [X+3C] vector data,
        [X+40] count, [X+80] 4FC600 argument (5A1040 context)."""
        self.reset_case()
        self.create_results = list(results)
        self.blocks = []
        self.seam(0x4f9c10, self._create)
        X = STACK - 0x800
        self.u.mem_write(X - 0x40, b'\xa5' * 0x100)
        self.w(X + 0x20, self.world)
        self.w(X + 0x3c, self.r(vec))
        self.w(X + 0x40, self.r(vec + 4))
        self.w(X + 0x80, ctx or self.ctx)
        self.u.reg_write(UC_X86_REG_ESP, X - 0x24)
        self.u.reg_write(UC_X86_REG_EBX, 0)
        self.run(0x4fd471, 0x4fd7b1)
        del self.seams[0x4f9c10]
        if self.outcome is None:
            assert self.u.reg_read(UC_X86_REG_EIP) == 0x4fd7b1
            assert self.u.reg_read(UC_X86_REG_ESP) == X
        return {'blocks': self.blocks, 'calls': self.calls, 'outcome': self.outcome}

    def _create(self, m):  # 4F9C10 thiscall(world, &params) ret 4 (seam)
        assert m.u.reg_read(UC_X86_REG_ECX) == m.world
        p = m.arg(0)
        words = [m.r(p + 4 * i) for i in range(10)]
        raw = bytes(m.u.mem_read(p, 0x28))
        result = m.create_results.pop(0)
        block = {'vtable': words[0], 'definition': m.labels.get(words[1], hex(words[1])),
                 'position': words[2:5], 'direction': words[5:7], 'name': m.cstring_units(words[7]),
                 'variant': words[8], 'requested_id': words[9], 'result': result}
        m.blocks.append(block)
        m.record('4F9C10', index=len(m.blocks) - 1, result=result)
        block['_raw'] = raw
        m.ret(4, MARKER if result else 0)


def clean(rows):
    for r in rows:
        r.pop('_raw', None)
    return rows


def gen_id(slot, generation):
    return (generation << 16) | slot


# ======================================================================
def section_mismatch():
    m = Conv()
    m.set_generation(7, 3)
    sid = gen_id(7, 3)
    cases = []
    classes_saved = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0xffffffff]
    for dclass in range(1, 11):
        m.defs = {}
        m.define('tree', dclass)
        for sclass in classes_saved:
            for ctx in (False, True):
                vec = m.new_vector()
                payload = record('tree', sclass, sid, name='nm', variant=2)
                res = m.load(payload, 0, vec if ctx else 0)
                cases.append({'saved_class': sclass, 'definition_class': dclass, 'context': ctx,
                              'payload': payload.hex(), 'out': res['out'], 'consumed': res['consumed'],
                              'calls': res['calls'], 'outcome': res['outcome'],
                              'vector': m.elements(vec)})
    # null results before the class compare
    m.defs = {}
    m.define('tree', 9)
    for label, ids, saved in (('generation_mismatch', 'tree', gen_id(7, 4)),
                              ('generation_mismatch_slot', 'tree', gen_id(8, 3)),
                              ('unknown_ids', 'bush', sid)):
        vec = m.new_vector()
        payload = record(ids, 2, saved)
        res = m.load(payload, 0, vec)
        cases.append({'label': label, 'saved_class': 2, 'definition_class': 9, 'context': True,
                      'payload': payload.hex(), 'out': res['out'], 'consumed': res['consumed'],
                      'calls': res['calls'], 'outcome': res['outcome'], 'vector': m.elements(vec)})
    return cases


def section_reads():
    out = []
    names = ['', 'n', 'tree_name_01', 'x' * 300, 'é中\ud800z']
    variants = [0, 3, 7, 0xffffffff, 0xfffffffb, 0x7fffffff, 0x80000000]
    scales = [f32(1.0), 0x7fc00000, 0x00000001, 0xff800000, 0x80000000]
    specs = []
    for tv in (0, 1, 2, 3, 0xffffffff, 0x80000000):
        for ov in (0, 1, 2, 3, 4, 5, 6, 0xffffffff):
            specs.append({'thing_version': tv, 'obj_version': ov, 'name': 'tree_name', 'variant': 3})
    for nm in names:
        specs.append({'name': nm, 'variant': 5})
    for v in variants:
        for s in scales:
            specs.append({'variant': v, 'scale': s, 'name': 'v'})
    specs.append({'thing_version': 1, 'obj_version': 1, 'name': 'abc', 'variant': 9,
                  'legacy': 0x01020304, 'flags': 0xffffffff, 'word10': 0x80000001})
    base_trunc = [dict(thing_version=1, obj_version=1, name='ab', variant=4, extra=b''),
                  dict(thing_version=2, obj_version=0, name='', extra=b''),
                  dict(thing_version=0, obj_version=5, name='q', variant=1, extra=b'')]
    for pc in ('Single24', 'Double53', 'Extended64'):
        m = Conv(pc)
        m.set_generation(0x1234, 0x55)
        sid = gen_id(0x1234, 0x55)
        m.define('arctic_trees', 9)
        rows = []
        for spec in specs:
            vec = m.new_vector()
            payload = record('arctic_trees', 2, sid, **spec)
            res = m.load(payload, 0, vec)
            rows.append({'spec': {k: (v.hex() if isinstance(v, bytes) else units(v) if k == 'name' else v)
                                  for k, v in spec.items()},
                         'payload': payload.hex(), 'out': res['out'], 'consumed': res['consumed'],
                         'reads': res['reads'], 'calls': res['calls'], 'outcome': res['outcome'],
                         'vector': m.elements(vec)})
        for spec in base_trunc:
            full = record('arctic_trees', 2, sid, **spec)
            for cut in range(len(full)):
                vec = m.new_vector()
                payload = full[:cut]
                res = m.load(payload, 0, vec)
                rows.append({'truncated_from': full.hex(), 'payload': payload.hex(), 'out': res['out'],
                             'consumed': res['consumed'], 'reads': res['reads'], 'calls': res['calls'],
                             'outcome': res['outcome'], 'vector': m.elements(vec)})
        out.append({'precision': pc, 'rows': rows})
    a = json.dumps(out[0]['rows'], sort_keys=True)
    assert all(json.dumps(o['rows'], sort_keys=True) == a for o in out), 'precision-dependent result'
    return out


def section_append():
    m = Conv()
    for s in range(1, 60):
        m.set_generation(s, s % 7)
    m.define('a_tree', 9)
    m.define('b_hedge', 9)
    m.define('c_bridge', 3, 'ActorData')
    vec = m.new_vector()
    steps = []
    for i in range(1, 60):
        ids = ['a_tree', 'b_hedge', 'c_bridge'][i % 3]
        klass = 2 if i % 5 else 6
        payload = record(ids, klass, gen_id(i, i % 7), name='n%d' % i, word10=i, variant=i % 4,
                         pos=(f32(i), f32(-i), f32(i * 0.5)), direction=(i, i + 1, i + 2),
                         thing_version=i % 3, obj_version=i % 6)
        res = m.load(payload, 0, vec)
        steps.append({'payload': payload.hex(), 'out': res['out'], 'consumed': res['consumed'],
                      'calls': res['calls'], 'outcome': res['outcome'], 'count': m.r(vec + 4),
                      'capacity': m.r(vec + 8)})
    return {'steps': steps, 'vector': m.elements(vec)}


def chunks_for(recs, header=0x77, index_version=0, data_version=0):
    data = W(data_version, header) + b''.join(r['bytes'] for r in recs)
    index = W(index_version, len(recs)) + W(*[r.get('length', len(r['bytes'])) for r in recs])
    return data, index


def section_section():
    cases = []
    sid = lambda s: gen_id(s, 2)

    def mk(kind, s, **kw):
        if kind == 'convert':
            b = record('tree', 2, sid(s), name='t%d' % s, variant=s, **kw)
        elif kind == 'skip':
            b = record('tree', 3, sid(s), **kw)
        elif kind == 'unknown':
            b = record('rock', 2, sid(s), **kw)
        elif kind == 'generation':
            b = record('tree', 2, gen_id(s, 9), **kw)
        elif kind == 'construct':
            b = record('house', 9, sid(s), **kw)
        return {'kind': kind, 'bytes': b}

    scenarios = {
        'mixed': [mk('convert', 1), mk('skip', 2), mk('unknown', 3), mk('generation', 4),
                  mk('construct', 5), mk('convert', 6, extra=b'\x11' * 40), mk('convert', 7)],
        # record shorter than its conversion reads: the next record's bytes are
        # read, then 858F50 seeks BACK to start + index length.
        'short_index_length': [mk('convert', 1), dict(mk('convert', 2), length=20), mk('convert', 3)],
        'bad_object_version': [mk('convert', 1), mk('convert', 2, obj_version=6), mk('convert', 3)],
        'empty': [],
    }
    for name, recs in scenarios.items():
        for construct_consume in ((0, 'full') if name == 'mixed' else (0,)):
            m = Conv()
            for s in range(1, 10):
                m.set_generation(s, 2)
            m.define('tree', 9)
            m.define('house', 9)
            data, index = chunks_for(recs)
            m.chunks = {'DYNAMIC_THINGS': data, 'DYNAMIC_THINGS_INDEX': index}
            starts, at = [], 8
            for r in recs:
                starts.append(at)
                at += r.get('length', len(r['bytes']))
            if construct_consume == 'full':
                i = [r['kind'] for r in recs].index('construct')
                m.construct_consume = len(recs[i]['bytes']) - (2 + 2 * len('house') + 8)
            vec = m.new_vector()
            m.reset_case()
            m.call(0x4fc340, m.world, [m.ctx, m.cstring('DYNAMIC_THINGS'), m.cstring('DYNAMIC_THINGS_INDEX'), vec], 0x10)
            cases.append({'scenario': name, 'construct_consume': m.construct_consume,
                          'definitions': {'tree': 9, 'house': 9}, 'generations': {str(s): 2 for s in range(1, 10)},
                          'records': [{'kind': r['kind'], 'hex': r['bytes'].hex(),
                                       'length': r.get('length', len(r['bytes'])), 'start': st}
                                      for r, st in zip(recs, starts)],
                          'data': data.hex(), 'index': index.hex(),
                          'calls': m.calls, 'final_position': m.at, 'outcome': m.outcome,
                          'vector': m.elements(vec)})
    # version / chunk errors of 4FC340 itself
    for name, kw, chunks in (('index_version_1', {'index_version': 1}, None),
                             ('data_version_1', {'data_version': 1}, None),
                             ('no_index_chunk', {}, ['DYNAMIC_THINGS']),
                             ('no_data_chunk', {}, ['DYNAMIC_THINGS_INDEX'])):
        m = Conv()
        m.set_generation(1, 2)
        m.define('tree', 9)
        data, index = chunks_for([mk('convert', 1)], **kw)
        allc = {'DYNAMIC_THINGS': data, 'DYNAMIC_THINGS_INDEX': index}
        m.chunks = {k: v for k, v in allc.items() if chunks is None or k in chunks}
        vec = m.new_vector()
        m.reset_case()
        m.call(0x4fc340, m.world, [m.ctx, m.cstring('DYNAMIC_THINGS'), m.cstring('DYNAMIC_THINGS_INDEX'), vec], 0x10)
        cases.append({'scenario': name, 'definitions': {'tree': 9}, 'generations': {'1': 2},
                      'data': data.hex(), 'index': index.hex(), 'calls': m.calls,
                      'outcome': m.outcome, 'vector': m.elements(vec)})
    return cases


DEF_SPECS = [
    # label, class, vtable, +108, +110
    ('thingdata', 1, 'ThingData', 1, 4),
    ('area_novar', 6, 'ObjectData', 0, 4),
    ('area_var4', 6, 'ObjectData', 1, 4),
    ('doodad_var1', 9, 'DynamicDoodadData', 1, 1),
    ('doodad_var0', 9, 'DynamicDoodadData', 1, 0),
    ('doodad_varneg', 9, 'DynamicDoodadData', 1, 0xffffffff),
    ('doodad_varmin', 9, 'DynamicDoodadData', 1, 0x80000000),
    ('doodad_novar', 9, 'DynamicDoodadData', 0, 3),
    ('actor_var3', 3, 'ActorData', 1, 3),
    ('actor_novar', 3, 'ActorData', 0, 1),
    ('byte2_var5', 9, 'DynamicDoodadData', 2, 5),
]
SAVED_VARIANTS = [0, 2, 3, 4, 5, 0xffffffff, 0xfffffffb, 0x7fffffff, 0x80000000]


def section_replace():
    cases = []
    for scenario in ('all', 'results', 'empty', 'single_null'):
        m = Conv()
        for s in range(1, 200):
            m.set_generation(s, 1)
        for label, klass, vt, var, cnt in DEF_SPECS:
            m.define(label, klass, vt, var, cnt)
        vec = m.new_vector()
        recs = []
        if scenario == 'all' or scenario == 'results':
            n = 0
            for label, *_ in DEF_SPECS:
                for v in SAVED_VARIANTS:
                    n += 1
                    if scenario == 'results' and n > 12:
                        break
                    recs.append(record(label, 2, gen_id(n, 1), name='keep%d' % n, word10=n * 3,
                                       pos=(f32(n), f32(n + 0.5), f32(-n)), direction=(f32(0.25), f32(-n), 0x3f000000 + n),
                                       variant=v, obj_version=1 + n % 5, thing_version=n % 3))
        elif scenario == 'single_null':
            recs.append(record('doodad_var1', 2, gen_id(1, 1), variant=0))
            recs.append(record('doodad_var1', 2, gen_id(2, 1), obj_version=0))
        loads = [m.load(p, 0, vec) for p in recs]
        assert all(l['out'] == 0 and l['outcome'] is None for l in loads)
        elements = m.elements(vec)
        if scenario == 'results':
            results = [i % 3 != 1 for i in range(len(recs))]
        elif scenario == 'single_null':
            results = [False, True]
        else:
            results = [True] * len(recs)
        res = m.tail(vec, results)
        cases.append({'scenario': scenario, 'definitions': [list(d) for d in DEF_SPECS],
                      'payloads': [p.hex() for p in recs], 'vector': elements,
                      'results': results, 'blocks': clean(res['blocks']), 'calls': res['calls'],
                      'outcome': res['outcome']})
    return cases


CTORS = {0x4ea5e0: 2, 0x4dc450: 10, 0x4dcb30: 9, 0x4db610: 6, 0x4dbee0: 7,
         0x4e4dc0: 4, 0x4e66d0: 5, 0x4de350: 8, 0x4d6bb0: 3}


def native_create(m, raw):
    """Original 4F9C10(world, block) on a copy of a tail parameter block;
    stops on entry of the class constructor (restoration agent's port)."""
    p = m.alloc(0x28)
    m.u.mem_write(p, raw)
    m.reset_case()

    def stop(addr):
        def h(m):
            m.record('ctor', address=addr, klass=CTORS[addr], params=m.arg(0) == p)
            m.outcome = {'kind': 'ReachedConstructor', 'address': addr}
            raise Stop()
        return h
    saved = {a: m.seams.get(a) for a in CTORS}
    for a in CTORS:
        m.seam(a, stop(a))
    r = m.call(0x4f9c10, m.world, [p], 4)
    for a, h in saved.items():
        if h is None:
            del m.seams[a]
        else:
            m.seams[a] = h
    return r


def create_code(m, r):
    if m.outcome is None:
        logs = [c['format'] for c in m.calls if c['seam'] == '404ED0']
        return 'null:' + ';'.join(logs) if r == 0 else 'nonnull'
    if m.outcome['kind'] == 'ReachedConstructor':
        return 'ctor:%X' % m.outcome['address']
    return 'fault:' + m.outcome.get('pc', m.outcome['kind'])


def section_vf30():
    """Definition virtual +30 of every Thing data class 4BCA00 creates."""
    m = Conv()
    out = []
    kinds = {0: 0xa20770, 1: 0xa13318, 2: 0xa05540, 3: 0xa0b218, 4: 0xa10210, 5: 0xa20800, 6: 0xa20890,
             7: 0xa20920, 8: 0xa209b0, 9: 0xa20a40, 10: 0xa10e10, 11: 0xa102a0, 12: 0xa20ad0, 13: 0xa20b60,
             14: 0xa210e8, 15: 0xa10330, 16: 0xa20bf0, 17: 0xa10ea0, 18: 0xa133b0, 19: 0xa09b30, 20: 0xa13448,
             21: 0xa0f6e0, 22: 0xa10450, 23: 0xa103c0}
    for kind, vt in kinds.items():
        d = m.alloc(0x200, 0)
        m.w(d, vt)
        m.reset_case()
        r = m.call(m.r(vt + 0x30), d, [], 0)
        out.append({'native_kind': kind, 'vtable': vt, 'function': m.r(vt + 0x30), 'eax': r})
    return out


def section_dispatch():
    """Original 4F9C10 with the parameter blocks the tail builds, for every
    definition class 0..11 (each on an ObjectData-family vtable)."""
    out = []
    for klass in range(0, 12):
        m = Conv()
        m.set_generation(1, 1)
        vt = 'ActorData' if klass in (3, 4, 8) else 'DynamicDoodadData'
        d = m.define('d', 9 if klass == 2 else klass, vt, 1, 2)
        vec = m.new_vector()
        assert m.load(record('d', 2, gen_id(1, 1), variant=1), 0, vec)['out'] == 0
        m.w(m.r(d + 0x18) + 0x1c, klass)  # class 2 itself cannot be converted: set after the load
        res = m.tail(vec, [True])
        raw = res['blocks'][0]['_raw']
        r = native_create(m, raw)
        out.append({'definition_class': klass, 'vtable': vt, 'block': clean(res['blocks'])[0],
                    'calls': m.calls, 'outcome': m.outcome,
                    'returned': None if m.outcome else r, 'code': create_code(m, r)})
    return out


# ======================================================================
def real_definitions():
    path = HERE / 'real-definitions.json'
    if not path.exists():
        raise SystemExit('real-definitions.json missing: run the Rust dump test first (README)')
    return json.loads(path.read_text())


def section_real():
    inv = json.loads((HERE.parent / 'phase8-definitions/inventory-round2.raw.json').read_text())
    defs = real_definitions()['definitions']
    out = []
    t0 = time.time()
    for mp in sorted(inv['maps'], key=lambda x: x['map']):
        group = [f for f in mp['failures'] if f['error'].startswith('source class ')]
        if not group:
            continue
        r = rmp.Rmp(rmp.MAPS / mp['map'])
        data = r.chunk('DYNAMIC_THINGS')
        lengths = rmp.index_lengths(r.chunk('DYNAMIC_THINGS_INDEX'))
        spans = rmp.records(data, lengths)
        idmgr = r.chunk('IDMGR')
        version, nxt, highest, cap = struct.unpack_from('<4I', idmgr, 0)
        gens = struct.unpack_from('<%dH' % highest, idmgr, 16)
        m = Conv()
        for slot, g in enumerate(gens):
            if g:
                m.set_generation(slot, g)
        for ids in sorted({f['definition'] for f in group}):
            d = defs[ids]
            vt = {21: 'DynamicDoodadData', 2: 'ActorData'}[d['native_kind']]
            m.define(ids, d['runtime_class'], vt, int(d['variations_enabled']), d['aspect_count'], label=ids)
        vec = m.new_vector()
        rows = []
        for f in sorted(group, key=lambda f: f['source_ordinal']):
            o = f['source_ordinal']
            start, length = spans[o]
            res = m.load(data, start, vec)
            assert res['outcome'] is None and res['out'] == 0, (mp['map'], o, res['outcome'])
            assert [c['seam'] for c in res['calls']] == ['427BD0'], res['calls']
            assert res['consumed'] <= length
            rows.append([o, f['saved_id'], start, length, res['consumed'], data[start:start + res['consumed']].hex()])
        elements = m.elements(vec)
        # 4F9C10 result used by the tail: null for the classes whose A25B1C
        # block is refused before construction (dispatch section: 3, 4, 8);
        # non-null ASSUMED once the class constructor is reached (its port and
        # the vf E4/E8 checks after it belong to the restoration agent).
        results = [defs[e['definition']]['runtime_class'] not in (3, 4, 8) for e in elements]
        tail = m.tail(vec, results)
        warnings = [c for c in tail['calls'] if c['seam'] == '5A1040']
        assert tail['outcome'] is None and len(warnings) == len(rows)
        codes = []
        for b, ok in zip(tail['blocks'], results):
            r = native_create(m, b['_raw'])
            codes.append(create_code(m, r))
            assert codes[-1].startswith('ctor:') == ok and (ok or codes[-1].startswith('null:')), codes[-1]
        out.append({'map': mp['map'], 'definitions': {i: defs[i] for i in sorted({f['definition'] for f in group})},
                    'records': rows,
                    'vector': [[e['ids'], e['definition'], e['saved_id'], e['name'], e['word_10'],
                                e['position'], e['direction'], e['variant']] for e in elements],
                    'blocks': [[b['vtable'], b['definition'], b['position'], b['direction'], b['name'],
                                b['variant'], b['requested_id']] for b in clean(tail['blocks'])],
                    'results': results,
                    'warnings': [[c['text'], c['ids']] for c in warnings],
                    'create': codes})
        print(mp['map'], len(rows), '%.0fs' % (time.time() - t0), flush=True)
    return out


def main():
    real = '--real' in sys.argv
    res = {'image': 'AA.exe (executed windows identical in runtime-expanded-v14/AA.exe)',
           'seams': SEAM_DOC}
    if not real:
        for name, fn in (('mismatch', section_mismatch), ('reads', section_reads), ('append', section_append),
                         ('section', section_section), ('replace', section_replace),
                         ('dispatch', section_dispatch), ('vf30', section_vf30)):
            t = time.time()
            res[name] = fn()
            print(name, 'done %.1fs' % (time.time() - t), flush=True)
        res['visited_instructions'] = len(VISITED)
        (HERE / 'native-conversion.json').write_text(json.dumps(res, indent=1))
    else:
        res['real'] = section_real()
        res['visited_instructions'] = len(VISITED)
        (HERE / 'native-conversion-real.json').write_text(json.dumps(res, separators=(',', ':')))


if __name__ == '__main__':
    main()
