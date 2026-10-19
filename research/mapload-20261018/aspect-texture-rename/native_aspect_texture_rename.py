"""Actual Aspect TextureRename source chain, original and v14 AA.exe, all PC.

replace: real 469D10 (CStringT::Replace) on counted UTF-16 CStrings, with
  in-place, Reallocate (refs 1, short allocation) and Fork (refs 2) buffers.
  The CString manager is an explicit child service whose new storage is
  poisoned (0xCDCD units), so a published unit nobody wrote is visible. The
  CRT memmove entry is observed; a negative balance stops the run there.
loader: real 49C670 -> 46A510 -> 469A70 -> 46A8A0 -> 46A180 -> 469D10 ->
  4108C0 -> 4763C0 on labelled typed source fields. Field getters, list
  iteration, heap, 497220 aspect_type map, 85A030 model normalization, 405D60
  printf and 424C10 trim are explicit seams; 5A79F0 is the required Texture
  request seam (records the name and search-path owner, returns a sentinel).
direct: real 46A180(flag 0) on the SAME constructed Aspect, the 46AE20 form.
No Texture/Model load, 5A77E0 lookup or file-system claim.
"""
from pathlib import Path
import sys, json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from unicorn import UcError
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP

POISON = 0xCDCD
MEMMOVE = 0x96bf30


def machine(image, pc):
    m = nm.Machine(pc)
    if image == 'v14':
        old = (nm.original.image_size + 4095) & ~4095
        new = (nm.modified.image_size + 4095) & ~4095
        if new > old:
            m.u.mem_map(nm.original.base + old, new - old)
        for vs, r, size, off in nm.modified.sections:
            if size:
                m.u.mem_write(nm.modified.base + r, nm.modified.b[off:off + size])
    return m


class Strings:
    """CString manager vtable +0 Allocate,+4 Free,+8 Reallocate,+C Nil,+10 Clone."""

    def __init__(self, m):
        self.m = m
        self.events = []
        self.mgr = m.alloc(4)
        vt = m.alloc(0x14)
        m.w(self.mgr, vt)
        m.w(vt + 0x0, m.stub(self.allocate))
        m.w(vt + 0x4, m.stub(self.free))
        m.w(vt + 0x8, m.stub(self.reallocate))
        m.w(vt + 0xc, m.stub(self.nil))
        m.w(vt + 0x10, m.stub(lambda mm: mm.ret(self.mgr)))
        self.nil_block = self.block(0, [], 2)
        m.seam(0x856370, lambda mm: mm.ret(self.mgr))

    def block(self, alloc, units, refs):
        m = self.m
        p = m.alloc(16 + 2 * (alloc + 1))
        m.u.mem_write(p + 16, POISON.to_bytes(2, 'little') * (alloc + 1))
        m.w(p, self.mgr)
        m.w(p + 4, len(units))
        m.w(p + 8, alloc)
        m.w(p + 12, refs)
        m.u.mem_write(p + 16, b''.join(u.to_bytes(2, 'little') for u in units) + b'\0\0')
        return p

    def allocate(self, mm):
        n, size = mm.arg(1), mm.arg(2)
        assert size == 2
        p = self.block(n, [], 1)
        mm.u.mem_write(p + 16, POISON.to_bytes(2, 'little') * (n + 1))
        self.events.append(['allocate', n])
        mm.ret(p, 8)

    def reallocate(self, mm):
        old, n, size = mm.arg(1), mm.arg(2), mm.arg(3)
        assert size == 2 and mm.r(old) == self.mgr
        p = self.block(n, [], 1)
        mm.u.mem_write(p + 16, POISON.to_bytes(2, 'little') * (n + 1))
        keep = 16 + 2 * (min(mm.r(old + 8), n) + 1)
        mm.u.mem_write(p, bytes(mm.u.mem_read(old, keep)))
        mm.w(p + 8, n)
        mm.u.mem_write(old, b'\xdd' * keep)
        self.events.append(['reallocate', n])
        mm.ret(p, 12)

    def free(self, mm):
        self.events.append(['free'])
        mm.ret(0, 4)

    def nil(self, mm):
        mm.w(self.nil_block + 12, mm.r(self.nil_block + 12) + 1)
        mm.ret(self.nil_block)

    def data(self, units, refs=1, extra=0):
        return self.block(len(units) + extra, units, refs) + 16

    def wide(self, text):
        """NUL-terminated raw wide text (a C string argument), or NULL."""
        if text is None:
            return 0
        raw = text.encode('utf-16-le') + b'\0\0'
        p = self.m.alloc(len(raw))
        self.m.u.mem_write(p, raw)
        return p

    def cstring(self, text):
        """CString object (one data pointer) holding text."""
        o = self.m.alloc(4)
        self.m.w(o, self.data(list(units(text))))
        return o


def units(text):
    raw = text.encode('utf-16-le')
    return [int.from_bytes(raw[i:i + 2], 'little') for i in range(0, len(raw), 2)]


def read_c(m, p):
    out = []
    while True:
        u = int.from_bytes(m.u.mem_read(p + 2 * len(out), 2), 'little')
        if u == 0:
            return out
        assert u != POISON, 'C-string read reached poisoned storage'
        out.append(u)
        assert len(out) < 4096


def read_counted(m, data):
    n = m.r(data - 12)
    raw = bytes(m.u.mem_read(data, 2 * n + 2))
    return [int.from_bytes(raw[i:i + 2], 'little') for i in range(0, 2 * n + 2, 2)]


def text_of(us):
    return bytes(b for u in us for b in u.to_bytes(2, 'little')).decode('utf-16-le')


# --------------------------------------------------------------------------- replace
def run_replace(image, pc, row, pins):
    m = machine(image, pc)
    s = Strings(m)
    this = m.alloc(4)
    data = s.data(row['source'], row['refs'], row['extra'])
    m.w(this, data)
    moves = []

    def observe(mm):
        size = mm.arg(3)
        moves.append(size if size < 0x80000000 else size - (1 << 32))
        if size >= 0x80000000:
            mm.u.emu_stop()
    m.observe(MEMMOVE, observe)
    old, new = s.wide(row['old']), s.wide(row['new'])
    try:
        count = m.call(0x469d10, this, [old, new], pop=8)
    except AssertionError:
        assert moves and moves[-1] < 0, 'unexpected native stop'
        pins.update(m.visited)
        return dict(outcome='negative_memmove_balance', memmove_sizes=moves)
    out = read_counted(m, m.r(this))
    pins.update(m.visited)
    published = out[:-1]
    return dict(outcome='uninitialized_published_units' if POISON in published else 'ok',
                count=count, units=published, terminator=out[-1],
                buffer='same' if m.r(this) == data else 'new',
                services=s.events, memmove_sizes=moves)


def replace_rows():
    rows = []

    def add(source, old, new, refs=2, extra=0, label=''):
        rows.append(dict(label=label, source=source if isinstance(source, list) else units(source),
                         old=old, new=new, refs=refs, extra=extra))
    real = ('DeathArcticBase', 'Base')
    for name in ['%sBase.dds', 'GBRBrigadeEngineerBase.dds', 'GBRBrigadeEngineerGround.dds',
                 'GBRBrigadeEngineerDeathArcticBase.dds']:
        add(name, *real, label='gbr_brigade_engineer')
    add('Hedges01Base.dds', 'Hedges01', 'DynamicHedge', label='temperate_hedge')
    add('TemperateHedges03.dds', 'Hedges03', 'DynamicHedge', label='temperate_hedge')
    for refs, extra in [(2, 0), (1, 0), (1, 40)]:
        add('aXbXc', 'X', 'YYY', refs, extra, 'grow')
        add('aXXbXXcXX', 'XX', 'Z', refs, extra, 'shrink')
        add('aXbXc', 'X', 'Y', refs, extra, 'equal')
    add('XaXbX', 'X', '', label='delete')
    add('aaaaa', 'aa', 'b', label='overlap')
    add('aaa', 'a', 'aa', label='replacement contains pattern')
    add('abcabc', 'abcabc', '', label='entire to empty')
    add('Base.dds', 'base', 'X', label='case sensitive')
    add('abc', '', 'x', label='empty old')
    add('abc', None, 'x', label='null old')
    add('abc', 'b', None, label='null new')
    add('abc', 'b', '', label='empty new')
    add('', 'a', 'b', label='empty source')
    add('abab', 'ab', 'abab', label='grow twice')
    add('%s%sBase', '%s', 'Q', label='format text')
    add('\U0001F600xBase', 'x', 'yy', label='surrogate pair')
    nul = units('ab') + [0] + units('ab')
    for new in ['x', 'xyz', '']:
        add(list(nul), 'ab', new, label='embedded NUL')
    add(units('a') + [0] + units('a'), 'a', 'ccc', label='embedded NUL later segment beyond original end')
    add(units('ab') + [0] + units('ab'), 'ab', 'abcdefg', label='embedded NUL later segment beyond original end')
    add(units('xab') + [0] + units('ab'), 'ab', '', label='embedded NUL shrink stale tail')
    add([0] + units('ab'), 'ab', 'c', label='leading NUL')
    return rows


# --------------------------------------------------------------------------- loader
def loader_cases():
    gbr = dict(definition='gbr_brigade_engineer', folder='Buildings/GBR',
               resource_prefix='GBRBrigadeEngineer/GBRBrigadeEngineer',
               fields=dict(ids='aspect_normal', aspect_type='OTHER', aspect_prefix='',
                           model='%s.nif', skin='%sBase.dds', glow='',
                           visual_radius_estimate=3.0, weight=1.0, ground_size=[4.0, 4.0],
                           disable_animations=False),
               renames=[['DeathArcticBase', 'Base']],
               textures=['GBRBrigadeEngineerBase.dds', 'GBRBrigadeEngineerGround.dds',
                         'GBRBrigadeEngineerDeathArcticBase.dds', ''])
    yield gbr
    yield dict(gbr, definition='ordered', renames=[['Base', 'X'], ['X', 'Y'], ['Y', 'Z']],
               fields=dict(gbr['fields'], glow='%sYGlow.dds'),
               textures=['aBase', 'aX', 'aY', 'aZ'])
    yield dict(gbr, definition='absolute aspect prefix', renames=[['Engineer', 'E']],
               fields=dict(gbr['fields'], aspect_prefix='/Abs/Pre', skin='%sEngineerBase.dds',
                           glow='/Shared/Engineer.dds'),
               textures=['/Shared/Engineer.dds', 'Engineer'])
    yield dict(gbr, definition='no rename', renames=[], textures=['x.dds'])
    yield dict(gbr, definition='rename empties skin', renames=[['%sBase.dds', '']],
               textures=['%sBase.dds'])
    yield dict(gbr, definition='empty skin with rename', renames=[['a', 'b']],
               fields=dict(gbr['fields'], skin=''), textures=['a'])
    yield dict(gbr, definition='empty folder', folder='', textures=['DeathArcticBase.dds'])


def run_loader(image, pc, case, pins):
    m = machine(image, pc)
    s = Strings(m)
    trace = []
    heap = []
    f = case['fields']

    def node(fields, children):
        p = m.alloc(4)
        nodes[p] = (fields, children)
        return p
    nodes = {}
    renames = [node({'substring_from': a, 'substring_to': b}, {}) for a, b in case['renames']]
    root = node(f, {'TextureRename': renames, 'Animation': []})
    data = m.alloc(0x180)
    m.u.mem_write(data, b'\xa5' * 0x180)
    m.w(data + 8, s.data(units(case['definition'])))
    m.w(data + 0xb0, s.data(units(case['resource_prefix'])))
    search = m.alloc(4)
    m.w(data + 0xb4, search)
    m.w(data + 0x110, 0)
    m.wb(data + 0xfc, 0)
    context = m.alloc(8)
    m.w(context + 4, s.data(units(case['folder'])))
    m.w(0xb36530, context)

    def key(mm, i=1):
        return text_of(read_c(mm, mm.arg(i)))

    def get_string(mm):
        fields, _ = nodes[mm.ecx()]
        k = key(mm)
        trace.append(['string', k])
        mm.ret(s.cstring(fields[k]), 4)
    m.seam(0x5b54d0, get_string)

    def get_float(mm, pop):
        fields, _ = nodes[mm.ecx()]
        k = key(mm)
        trace.append(['float', k])
        mm.ret_st0_f32(nm.f32(fields[k]), pop)
    m.seam(0x5b4ae0, lambda mm: get_float(mm, 4))
    m.seam(0x5b49d0, lambda mm: get_float(mm, 8))

    def get_point(mm):
        fields, _ = nodes[mm.ecx()]
        out = mm.arg(1)
        k = text_of(read_c(mm, mm.arg(2)))
        trace.append(['point', k])
        mm.w(out, nm.f32(fields[k][0]))
        mm.w(out + 4, nm.f32(fields[k][1]))
        mm.ret(out, 8)
    m.seam(0x5b4d80, get_point)

    def get_bool(mm):
        fields, _ = nodes[mm.ecx()]
        k = key(mm)
        trace.append(['bool', k])
        mm.ret(int(fields[k]), 4)
    m.seam(0x5b5380, get_bool)

    def count(mm):
        _, children = nodes[mm.ecx()]
        k = key(mm)
        trace.append(['count', k])
        mm.ret(len(children[k]), 4)
    m.seam(0x5b4220, count)

    def element(mm):
        it = mm.ecx()
        parent, index, k = mm.r(it), mm.r(it + 4), text_of(read_c(mm, mm.r(it + 8)))
        trace.append(['element', k, index])
        mm.ret(nodes[parent][1][k][index])
    m.seam(0x42be10, element)
    aspect_type = m.alloc(8)
    m.w(aspect_type, aspect_type + 4)
    m.w(aspect_type + 4, 7)
    m.seam(0x497220, lambda mm: (trace.append(['aspect_type', text_of(read_c(mm, mm.r(mm.arg(1))))]),
                                 mm.ret(aspect_type, 4)))

    def new(mm):
        p = mm.alloc(mm.arg(1))
        mm.u.mem_write(p, b'\xa5' * mm.arg(1))
        heap.append(mm.arg(1))
        mm.ret(p)
    m.seam(0x6f7810, new)
    m.seam(0x85a030, lambda mm: (trace.append(['normalize_85a030', text_of(read_c(mm, mm.r(mm.arg(1))))]),
                                 mm.ret()))

    def fmt(mm):
        out = mm.arg(1)
        f_ = read_c(mm, mm.arg(2))
        text, used, i, argi = [], 0, 0, 3
        args = []
        while i < len(f_):
            c = f_[i]
            if c == 0x25 and i + 1 < len(f_) and f_[i + 1] == 0x73:
                a = read_c(mm, mm.arg(argi))
                args.append(text_of(a))
                text += a
                argi += 1
                i += 2
            elif c == 0x25 and i + 1 < len(f_) and f_[i + 1] == 0x25:
                text.append(0x25)
                i += 2
            else:
                assert c != 0x25, 'fixture format outside %s/%%'
                text.append(c)
                i += 1
        trace.append(['format_405d60', text_of(f_), args])
        mm.w(out, s.data(text))
        mm.ret()
    m.seam(0x405d60, fmt)

    def trim(mm):
        assert mm.arg(1) == 0x2f
        o = mm.ecx()
        t = read_c(mm, mm.r(o))
        while t and t[0] == 0x2f:
            t = t[1:]
        mm.w(o, s.data(t))
        mm.ret(o, 4)
    m.seam(0x424c10, trim)
    textures = []

    def texture(mm):
        name = text_of(read_counted(mm, mm.r(mm.arg(1)))[:-1])
        result = 0x7e000010 + 0x10 * len(textures)
        textures.append(dict(name=name, search_owner_is_object_b4=mm.arg(2) == data + 0xb4,
                             result=result))
        trace.append(['texture_5a79f0', name])
        mm.ret(result)
    m.seam(0x5a79f0, texture)
    errors = []
    m.seam(0x410c40, lambda mm: (errors.append(text_of(read_c(mm, mm.arg(1)))), mm.ret()))
    appended = []
    m.seam(0x49c360, lambda mm: (appended.append(mm.arg(1)), mm.ret(0, 4)))
    m.call(0x49c670, data, [root], pop=4)
    assert len(appended) == 1 and not errors, (appended, errors)
    a = appended[0]
    head, tail = m.r(a + 0x3c), m.r(a + 0x40)
    nodes_out, p = [], head
    while p:
        nodes_out.append([text_of(read_c(m, m.r(p))), text_of(read_c(m, m.r(p + 4)))])
        last, p = p, m.r(p + 8)
    tail_is_last = (tail == last) if head else tail == 0
    out = dict(trace=trace, textures=textures, rename_list=nodes_out, tail_is_last_node=tail_is_last,
               skin_30=m.r(a + 0x30), glow_34=m.r(a + 0x34),
               search_44_is_object_b4=m.r(a + 0x44) == data + 0xb4,
               resource_prefix_0=text_of(read_c(m, m.r(a))),
               aspect_prefix_4=text_of(read_c(m, m.r(a + 4))),
               model_20=text_of(read_c(m, m.r(a + 0x20))))
    direct = []
    for name in case['textures']:
        before_trace, before_tex = len(trace), len(textures)
        result = m.call(0x46a180, a, [s.cstring(name), 0], pop=8)
        direct.append(dict(source=name, result=result, trace=trace[before_trace:],
                           textures=textures[before_tex:]))
    out['direct_46a180_flag0'] = direct
    pins.update(m.visited)
    return out


def main():
    pins = {}
    replace = []
    for row in replace_rows():
        outs = {}
        for pc in nm.PRECISIONS:
            for image in ['original', 'v14']:
                outs[(pc, image)] = run_replace(image, pc, row, pins)
        first = next(iter(outs.values()))
        assert all(o == first for o in outs.values()), row
        replace.append(dict(input=row, native=first))
    loader = []
    for case in loader_cases():
        outs = {}
        for pc in nm.PRECISIONS:
            for image in ['original', 'v14']:
                outs[(pc, image)] = run_loader(image, pc, case, pins)
        first = next(iter(outs.values()))
        assert all(o == first for o in outs.values()), case['definition']
        loader.append(dict(input=case, native=first))
    for a, b in pins.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b)), hex(a)
    doc = {**nm.header(__file__, __doc__), 'precisions': list(nm.PRECISIONS), 'images': ['original', 'v14'],
           'replace': replace, 'loader': loader, 'instructions': nm.instructions(pins),
           'harness_sha256': hashlib.sha256((ROOT / 'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
    path = HERE / 'native-aspect-texture-rename.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1))
    else:
        assert json.loads(path.read_text()) == doc
    print(json.dumps(dict(replace=len(replace), loader=len(loader), machines=6 * (len(replace) + len(loader)),
                          pins=len(pins), sha256=hashlib.sha256(path.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
