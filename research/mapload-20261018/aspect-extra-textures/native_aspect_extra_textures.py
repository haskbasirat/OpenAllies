"""Actual Aspect extra (loaded-Model) textures, original and v14 AA.exe, all PC.

search: real 5A79F0 (859B70/85A550/859940/85ABB0/85A1A0 run natively) over a
  named texture and an ObjectData+B4 search-path list; 5A77E0 is the required
  Texture lookup seam (records name + flags, answers from the row's hit set),
  85A030 is a recording identity seam (only reached by '.'-relative paths).
extra: real 46AE20 -> 46ADD0/46A450 (+48 {data,count,cap} vector, element
  ctor 469CF0, CRT eh vector iterators) -> 5A4ED0/5A5590/5A55A0 -> 46A180(,0)
  -> 469D10 -> 4763C0 over sequences of calls on ONE Aspect. 5A79F0 is the
  Texture request seam (records name and owner, returns a fake Texture whose
  vf10 records delta/mode, or NULL). Heap is a poisoned (0xA5) service, so
  element +0 that no store wrote stays visible as 0xA5A5A5A5.
block: real 46AF70 with Model vf18 and 46AE20 as recording seams (the seam
  either sets +54 or not) and 855D30 as the assertion seam.
No Texture/Model load, 5A77E0 resource lookup or file-system claim.
"""
from pathlib import Path
import sys, json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
sys.path.insert(0, str(HERE.parent / 'aspect-texture-rename'))
import native_machine as nm
import native_aspect_texture_rename as rn

POISON32 = 0xa5a5a5a5


def path_services(m, s, trace):
    """405D60 printf (%s/%% only) and 424C10 TrimLeft('/') for 4763C0."""
    def fmt(mm):
        out, f_ = mm.arg(1), rn.read_c(mm, mm.arg(2))
        text, i, argi = [], 0, 3
        while i < len(f_):
            if f_[i] == 0x25 and i + 1 < len(f_) and f_[i + 1] == 0x73:
                text += rn.read_c(mm, mm.arg(argi))
                argi += 1
                i += 2
            elif f_[i] == 0x25 and i + 1 < len(f_) and f_[i + 1] == 0x25:
                text.append(0x25)
                i += 2
            else:
                assert f_[i] != 0x25
                text.append(f_[i])
                i += 1
        trace.append(['format_405d60', rn.text_of(f_)])
        mm.w(out, s.data(text))
        mm.ret()
    m.seam(0x405d60, fmt)

    def trim(mm):
        assert mm.arg(1) == 0x2f
        o = mm.ecx()
        t = rn.read_c(mm, mm.r(o))
        while t and t[0] == 0x2f:
            t = t[1:]
        mm.w(o, s.data(t))
        mm.ret(o, 4)
    m.seam(0x424c10, trim)


def heap(m, freed):
    def new(mm):
        p = mm.alloc(mm.arg(1))
        mm.u.mem_write(p, b'\xa5' * mm.arg(1))
        mm.ret(p)
    m.seam(0x6f7810, new)
    m.seam(0x6f7430, lambda mm: (freed.append(mm.arg(1)), mm.ret()))


def ctext(m, obj):
    return rn.text_of(rn.read_counted(m, m.r(obj))[:-1])


# --------------------------------------------------------------------------- 5A79F0
def search_rows():
    rows = []

    def add(label, name, paths, hits, final=0x7e0f0000, owner=True):
        rows.append(dict(label=label, name=name, paths=paths, hits=hits, final=final, owner=owner))
    n = 'Buildings/GBR/GBRBrigadeEngineer/GBRBrigadeEngineerBase.dds'
    add('direct hit', n, ['Shared/SharedTextures'], [n])
    add('first search path hit', 'Units/USA/USATruck/Treds.dds', ['Shared/SharedTextures'],
        ['Shared/SharedTextures/Treds.dds'])
    add('second search path hit', 'Units/USA/X/woodbeams.DDS',
        ['Shared/SharedTextures/Buildings/EasternEurope', 'Shared/SharedTextures'],
        ['Shared/SharedTextures/woodbeams.DDS'])
    add('all miss final flags0', 'Units/USA/X/Missing.dds', ['Shared/SharedTextures', 'Units/USA/USATruck'], [])
    add('empty path entry skipped', 'A/B/c.dds', ['', 'Shared'], ['Shared/c.dds'])
    add('null owner', 'A/B/c.dds', None, [])
    add('empty list', 'A/B/c.dds', [], [])
    add('name without directory', 'c.dds', ['Shared'], ['Shared/c.dds'])
    add('backslash name', 'Units\\GBR\\c.dds', ['Shared'], ['Shared/c.dds'])
    add('trailing slash path', 'A/c.dds', ['Shared/'], ['Shared/c.dds'])
    add('drive colon name', 'C:c.dds', ['Shared'], ['Shared/c.dds'])
    add('dot relative path', 'A/B/c.dds', ['.\\Sub'], ['A/B/Sub/c.dds'])
    add('final hit only', 'A/B/c.dds', ['Shared'], [], final=0x7e0f1230)
    return rows


def run_search(image, pc, row, pins):
    m = rn.machine(image, pc)
    s = rn.Strings(m)
    freed = []
    heap(m, freed)
    calls = []
    hits = {h: 0x7e100000 + 0x10 * i for i, h in enumerate(row['hits'])}

    def lookup(mm):
        name, flags = ctext(mm, mm.arg(1)), mm.arg(2)
        calls.append(['5a77e0', name, flags])
        if flags == 0:
            mm.ret(row['final'])
        else:
            mm.ret(hits.get(name, 0))
    m.seam(0x5a77e0, lookup)
    m.seam(0x85a030, lambda mm: (calls.append(['85a030', ctext(mm, mm.arg(1))]), mm.ret()))
    owner = 0
    if row['owner'] and row['paths'] is not None:
        owner = m.alloc(8)
        nodes = [m.alloc(8) for _ in row['paths']]
        for i, (p, path) in enumerate(zip(nodes, row['paths'])):
            m.w(p, s.data(rn.units(path)))
            m.w(p + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        m.w(owner, nodes[0] if nodes else 0)
        m.w(owner + 4, nodes[-1] if nodes else 0)
    result = m.call(0x5a79f0, 0, [s.cstring(row['name']), owner], pop=0)
    pins.update(m.visited)
    return dict(result=result, calls=calls)


# --------------------------------------------------------------------------- 46AE20
GBR_DIR = 'Buildings/GBR/GBRBrigadeEngineer/'
MODELS = {
    'gbr': [(GBR_DIR + 'GBRBrigadeEngineerBase.dds', 'GBRBrigadeEngineerBase.dds'),
            (GBR_DIR + 'GBRBrigadeEngineerGround.dds', 'GBRBrigadeEngineerGround.dds')],
    'one': [('Units/X/one.dds', 'one.dds')],
    'three': [('Units/X/a.dds', 'a.dds'), ('Units/X/b.dds', 'b.dds'), ('Units/X/c.dds', 'c.dds')],
    'five': [('Units/X/t%d.dds' % i, 't%d.dds' % i) for i in range(5)],
    'nine': [('U/n%d.dds' % i, 'n%d.dds' % i) for i in range(9)],
    'none': [],
    'rename': [('Units/X/DeathArcticBase.dds', 'DeathArcticBase.dds'), ('Units/X/Base.dds', 'Base.dds')],
    'absolute': [('/Shared/abs.dds', 'abs.dds')],
    'empty': [('Units/X/a.dds', 'a.dds'), ('', '')],
}


def extra_sequences():
    s = lambda model, delta=1, mode=0, reset=False: dict(model=model, delta=delta, mode=mode, reset=reset)
    gbr = dict(folder='', renames=[['DeathArcticBase', 'Base']])
    yield dict(gbr, label='gbr_brigade_engineer request then same-count request',
               steps=[s('gbr', 1, 0), s('gbr', 2, 1), s('five', 1, 0, True), s('one', 3, 0, True),
                      s('none', 1, 1, True)])
    yield dict(gbr, label='growth from constructor capacity 1',
               steps=[s('one'), s('gbr', reset=True), s('nine', reset=True), s('three', reset=True)])
    yield dict(gbr, label='first allocation two', steps=[s('gbr', 0xffffffff, 1)])
    yield dict(gbr, label='first allocation three', steps=[s('three', 7, 0)])
    yield dict(gbr, label='zero textures first', steps=[s('none'), s('one')])
    yield dict(gbr, label='count change without 469FE0 reset', steps=[s('gbr'), s('three'), s('three')])
    yield dict(folder='Weapons', renames=[], label='nonempty GameData+4 folder',
               steps=[s('gbr'), s('absolute', reset=True)])
    yield dict(folder='', renames=[['X', 'Y'], ['DeathArcticBase', 'Base']], label='rename list flag0',
               steps=[s('rename')])
    yield dict(gbr, label='empty model texture name gives NULL Texture vf10', steps=[s('empty')])


def run_extra(image, pc, seq, pins):
    m = rn.machine(image, pc)
    s = rn.Strings(m)
    freed = []
    heap(m, freed)
    events = []
    path_services(m, s, events)
    ctx = m.alloc(8)
    m.w(ctx + 4, s.data(rn.units(seq['folder'])))
    m.w(0xb36530, ctx)
    aspect = m.alloc(0x6c)
    m.u.mem_write(aspect, b'\xa5' * 0x6c)
    m.w(aspect + 0, s.data(rn.units('GBRBrigadeEngineer/GBRBrigadeEngineer')))
    m.w(aspect + 4, s.data([]))
    head = 0
    for a, b in seq['renames']:  # list order given head first
        node = m.alloc(12)
        m.w(node, s.data(rn.units(a)))
        m.w(node + 4, s.data(rn.units(b)))
        m.w(node + 8, 0)
        if head == 0:
            head = node
        else:
            last = head
            while m.r(last + 8):
                last = m.r(last + 8)
            m.w(last + 8, node)
    m.w(aspect + 0x3c, head)
    search = m.alloc(8)
    m.w(search, 0)
    m.w(search + 4, 0)
    m.w(aspect + 0x44, search)
    m.w(aspect + 0x18, 0)
    m.w(aspect + 0x48, 0)
    m.w(aspect + 0x4c, 0)
    m.w(aspect + 0x50, 1)
    m.wb(aspect + 0x54, 0)
    textures = {}
    tex_vt = m.alloc(0x20)

    def vf10(mm):
        events.append(['vf10', textures[mm.ecx()], mm.arg(1), mm.arg(2)])
        mm.ret(0, 8)
    m.w(tex_vt + 0x10, m.stub(vf10))

    def request(mm):
        name = ctext(mm, mm.arg(1))
        events.append(['5a79f0', name, mm.arg(2) == search])
        if name == '':
            mm.ret(0)
            return
        t = m.alloc(16)
        m.w(t, tex_vt)
        textures[t] = 'tex%d' % len(textures)
        mm.ret(t)
    m.seam(0x5a79f0, request)
    models = {}
    for key, names in MODELS.items():
        model = m.alloc(0x90)
        m.u.mem_write(model, b'\xa5' * 0x90)
        arr = m.alloc(8 * max(len(names), 1))
        for i, (full, raw) in enumerate(names):
            m.w(arr + 8 * i, s.data(rn.units(full)))
            m.w(arr + 8 * i + 4, s.data(rn.units(raw)))
        m.w(model + 0x7c, len(names))
        m.w(model + 0x80, arr)
        models[key] = model
    out = []
    for step in seq['steps']:
        if step['reset']:
            #469FE0 context change: +4C = 0 and +54 = 0 (+48 data retained).
            m.w(aspect + 0x4c, 0)
            m.wb(aspect + 0x54, 0)
        m.w(aspect + 0x28, models[step['model']])
        before = len(events)
        try:
            m.call(0x46ae20, aspect, [step['delta'], step['mode']], pop=8)
        except AssertionError:
            assert m.null_accesses, 'unexpected native stop'
            pins.update(m.visited)
            out.append(dict(step=step, outcome='null_texture_vf10', events=events[before:]))
            return out
        data, count, cap = m.r(aspect + 0x48), m.r(aspect + 0x4c), m.r(aspect + 0x50)
        elements = []
        if data:
            assert m.r(data - 4) == cap
            for i in range(cap):
                p = m.r(data + 8 * i)
                kind = textures.get(p) or ('unwritten' if p == POISON32 else ('null' if p == 0 else hex(p)))
                elements.append([kind, ctext(m, data + 8 * i + 4)])
        out.append(dict(step=step, outcome='ok', events=events[before:], data=bool(data), count=count,
                        capacity=cap, requested=m.rb(aspect + 0x54), elements=elements))
    pins.update(m.visited)
    return out


# --------------------------------------------------------------------------- 46AF70
def block_cases():
    for loaded in [0, 1]:
        for requested in [0, 1]:
            for latch in [0, 1]:
                for sets in [True, False]:
                    if requested and not sets:
                        continue
                    yield dict(loaded=loaded, requested=requested, latch=latch, sets=sets, count=5)


def run_block(image, pc, c, pins):
    m = rn.machine(image, pc)
    events = []
    aspect = m.alloc(0x6c)
    m.u.mem_write(aspect, b'\xa5' * 0x6c)
    model = m.alloc(0x90)
    vt = m.alloc(0x40)
    m.w(model, vt)
    m.w(model + 8, 0x410 | c['loaded'])
    m.w(vt + 0x18, m.stub(lambda mm: (events.append(['vf18', mm.ecx() == model]), mm.ret())))
    m.w(aspect + 0x28, model)
    m.w(aspect + 0x18, c['count'])
    m.wb(aspect + 0x54, c['requested'])
    m.wb(0xb3a747, c['latch'])

    def extra(mm):
        events.append(['46ae20', mm.ecx() == aspect, mm.arg(1), mm.arg(2)])
        if c['sets']:
            mm.wb(aspect + 0x54, 1)
        mm.ret(0, 8)
    m.seam(0x46ae20, extra)

    def assertion(mm):
        events.append(['855d30', rn.text_of(rn.read_c(mm, mm.arg(1))), mm.arg(3)])
        mm.ret(0x2a)
    m.seam(0x855d30, assertion)
    m.call(0x46af70, aspect, [], pop=0)
    pins.update(m.visited)
    return dict(events=events, requested=m.rb(aspect + 0x54), latch=m.rb(0xb3a747))


def main():
    pins = {}

    def every(fn, row):
        outs = [fn(image, pc, row, pins) for pc in nm.PRECISIONS for image in ['original', 'v14']]
        assert all(o == outs[0] for o in outs), row
        return outs[0]
    search = [dict(input=r, native=every(run_search, r)) for r in search_rows()]
    extra = [dict(input=q, native=every(run_extra, q)) for q in extra_sequences()]
    block = [dict(input=c, native=every(run_block, c)) for c in block_cases()]
    for a, b in pins.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b)), hex(a)
    doc = {**nm.header(__file__, __doc__), 'precisions': list(nm.PRECISIONS), 'images': ['original', 'v14'],
           'helper_sha256': hashlib.sha256(Path(rn.__file__).read_bytes()).hexdigest(),
           'harness_sha256': hashlib.sha256((ROOT / 'research/phase8-ai/native_machine.py').read_bytes()).hexdigest(),
           'search': search, 'extra': extra, 'block': block, 'instructions': nm.instructions(pins)}
    path = HERE / 'native-aspect-extra-textures.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1))
    else:
        assert json.loads(path.read_text()) == doc
    print(json.dumps(dict(search=len(search), extra=len(extra), block=len(block),
                          machines=6 * (len(search) + len(extra) + len(block)), pins=len(pins),
                          sha256=hashlib.sha256(path.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
