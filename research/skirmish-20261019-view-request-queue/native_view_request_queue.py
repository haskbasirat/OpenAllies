"""VWorld request queue (+208 head / +20C tail, cells {view, next}) under
Unicorn, ORIGINAL AA.exe code only:

1. 43EC60 segment 43ED1A..43ED53 (VWorld update): while the head cell's view
   vf74 is true -> 45B720 (view+10 Data vf18(1), view+54 &= ~bit28), then
   50C510 unlinks and frees that head cell. Real 45B720 and 50C510 run.
2. 45BCA0 prefix 45BCA0..45BCCB (VThing uninit): view+54 bit28 -> Data vf18(1),
   clear bit28, then 43C490 -> 4F1430 on [B36538]+208 removes the first cell
   holding this view (real code).
Seams: view vf74 (VObject/VActor/organization views all use 45B140, a tail
jump to Data vf1C; answered per case), Data vf18 (recorded), 6F7430 free
(recorded). Default compares the fixture; --write regenerates it.
"""
from pathlib import Path
import sys, json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-conversion'))
import native_machine as nm
from native_machine import Machine, P, V, VISITED
from unicorn.x86_const import *
H = lambda b: hashlib.sha256(b).hexdigest()
FRAME = 0x2f07000
BIT28 = 0x10000000


def build(m, c):
    vworld = m.alloc(0x300)
    views = {}
    trace = []
    for name, flags in c['views'].items():
        v = m.alloc(0x80)
        data = m.alloc(0x40)
        dvt = m.alloc(0x40)
        m.w(data, dvt)
        m.w(dvt + 0x18, m.stub(lambda mm, name=name: (trace.append({'call': 'release', 'view': name, 'delta': mm.arg(0)}), mm.ret(4, 0))))
        vvt = m.alloc(0x100)
        m.w(v, vvt)

        def ready(mm, name=name):
            count = sum(1 for t in trace if t['call'] == 'vf74' and t['view'] == name)
            answers = c['ready'][name]
            trace.append({'call': 'vf74', 'view': name})
            mm.ret(0, int(answers[min(count, len(answers) - 1)]))
        m.w(vvt + 0x74, m.stub(ready))
        m.w(v + 0x10, data)
        m.w(v + 0x54, flags)
        views[name] = v
    head = tail = 0
    cells = []
    for name in c['queue']:
        cell = m.alloc(8)
        m.w(cell, views[name])
        m.w(cell + 4, 0)
        if tail:
            m.w(tail + 4, cell)
        else:
            head = cell
        tail = cell
        cells.append(cell)
    m.w(vworld + 0x208, head)
    m.w(vworld + 0x20c, tail)
    m.seam(0x6f7430, lambda mm: (trace.append({'call': 'free', 'cell': cells.index(mm.arg(0)) if mm.arg(0) in cells else None}), mm.ret(0)))
    return vworld, views, cells, trace


def state(m, vworld, views, cells):
    names = {v: k for k, v in views.items()}
    out, cell = [], m.r(vworld + 0x208)
    while cell:
        out.append(names[m.r(cell)])
        last = cell
        cell = m.r(cell + 4)
    tail = m.r(vworld + 0x20c)
    return {'queue': out, 'tail': cells.index(tail) if tail in cells else (None if tail == 0 else 'other'),
            'flags': {k: m.r(v + 0x54) for k, v in views.items()}}


def run(c):
    m = Machine(c['precision'])
    vworld, views, cells, trace = build(m, c)
    if c['kernel'] == 'drain':
        m.u.reg_write(UC_X86_REG_ESI, vworld)
        m.u.reg_write(UC_X86_REG_ESP, FRAME)
        m.run(0x43ed1a, 0x43ed53)
        assert m.u.reg_read(UC_X86_REG_EIP) == 0x43ed53 and m.u.reg_read(UC_X86_REG_ESP) == FRAME
        assert m.u.reg_read(UC_X86_REG_ESI) == vworld
    else:
        m.w(0xb36538, vworld)
        m.u.reg_write(UC_X86_REG_ECX, views[c['uninit']])
        m.u.reg_write(UC_X86_REG_ESP, FRAME)
        m.run(0x45bca0, 0x45bccb)
        assert m.u.reg_read(UC_X86_REG_EIP) == 0x45bccb and m.u.reg_read(UC_X86_REG_ESP) == FRAME - 8
    assert m.outcome is None
    return {'trace': trace, **state(m, vworld, views, cells)}


def cases():
    out = []
    F = BIT28 | 0x100 | 1
    for pc in ['Single24', 'Double53', 'Extended64']:
        def add(label, **c):
            out.append({'label': pc + '_' + label, 'precision': pc, **c})
        three = {'a': F, 'b': F, 'c': F}
        add('drain_empty', kernel='drain', views=three, queue=[], ready={'a': [1], 'b': [1], 'c': [1]})
        add('drain_all', kernel='drain', views=three, queue=['a', 'b', 'c'], ready={'a': [1], 'b': [1], 'c': [1]})
        add('drain_head_not_ready', kernel='drain', views=three, queue=['a', 'b', 'c'], ready={'a': [0], 'b': [1], 'c': [1]})
        add('drain_middle_not_ready', kernel='drain', views=three, queue=['a', 'b', 'c'], ready={'a': [1], 'b': [0], 'c': [1]})
        add('drain_last_not_ready', kernel='drain', views=three, queue=['a', 'b', 'c'], ready={'a': [1], 'b': [1], 'c': [0]})
        add('drain_duplicate', kernel='drain', views=three, queue=['a', 'b', 'a', 'c'], ready={'a': [1], 'b': [1], 'c': [1]})
        add('drain_duplicate_second_not_ready', kernel='drain', views=three, queue=['a', 'b', 'a', 'c'], ready={'a': [1, 0], 'b': [1], 'c': [1]})
        add('drain_flag_clear_still_released', kernel='drain', views={'a': 1, 'b': F}, queue=['a', 'b'], ready={'a': [1], 'b': [1]})
        for name, flags in [('flag_clear', 1), ('flag_set', F)]:
            for queue in [[], ['a'], ['a', 'b', 'c'], ['b', 'a', 'c'], ['b', 'c', 'a'], ['b', 'c'], ['a', 'b', 'a']]:
                add('uninit_%s_%s' % (name, ''.join(queue) or 'empty'), kernel='uninit', uninit='a',
                    views={'a': flags, 'b': F, 'c': F}, queue=queue, ready={'a': [1], 'b': [1], 'c': [1]})
    return out


def main():
    rows = [{'input': c, 'output': run(c)} for c in cases()]
    for a, b in VISITED.items():
        assert P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b))), hex(a)
    doc = {'schema': 1, 'passed': True, 'scope': __doc__, 'producer_sha256': H(Path(__file__).read_bytes()),
           'base_machine_sha256': H(Path(nm.__file__).read_bytes()), 'original_sha256': H(P.b), 'v14_sha256': H(V.b),
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())], 'cases': rows}
    p = HERE / 'native-view-request-queue.json'
    if '--write' in sys.argv:
        p.write_text(json.dumps(doc, indent=1) + '\n')
    else:
        assert json.loads(p.read_text()) == doc, 'native view request queue fixture differs'
    print(json.dumps({'cases': len(rows), 'instructions': len(VISITED), 'sha256': H(p.read_bytes())}))


if __name__ == '__main__':
    main()
