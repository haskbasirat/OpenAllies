"""Actual 464BD0 MoverComponent TrackInfo loop (465DA2..465F79).

Runs the ORIGINAL loop bytes from 465DA2 (esi=0, ebx=Mover node, ebp=ActorData)
to the 465F79 exit, including 45E880 growth (96D18D/96BE33 iterators, 4BDDB0
ctor, 4BDDA0 dtor), 405030 hash, 497220 lookup and _wcsicmp 96CAD6 over the
B3B54C track-type table. The table is laid out as 4BDFF0(this=B3B53C, 1) builds
it from static init 9CD190 (+10 buckets, +14 = 7 buckets; 4BDEE0 entries
{value, name, 0}: DISCRETE=0 then CONTINUOUS=1), bucketed with the REAL 405030.
ActorData starts in the 4624B0 constructor state (+25C one 4BDDB0 entry, +260=0,
+264=1).

Explicit transport seams (recorded with their arguments): 5B4220 child count,
42BE10 child accessor, 5B54D0/5B5380/5B4AE0/5B4D80 typed getters, 405B80
CString assignment, 405D60 format, 4763C0 GameData-relative path, 5A77E0
texture, 6F7810/6F7430 allocation, 410C40 (native throws) and _getptd 9738A9
(returns a thread block whose +64 locinfo is the process B35444 C locale; the
only setlocale calls, 7F1965/7F1A39, change LC_NUMERIC only). Each case runs twice
with two allocation fill patterns; bytes that differ are UNWRITTEN by native
code. No game file is written.
"""
from pathlib import Path
import sys, json, struct, hashlib

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / 'phase8-ai'))
from native_machine import Machine, original, modified, sha, instructions, header, STACK
from unicorn.x86_const import (UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_EBP, UC_X86_REG_ESP,
                               UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EIP, UC_X86_REG_FPCW)

FILLS = (0xa5, 0x5a)
GAMEDATA = 0x00b36530
TABLE = 0xb3b53c


class Stop(Exception):
    pass


class Lab(Machine):
    def wstr(self, text):
        data = text.encode('utf-16-le') + b'\0\0'
        p = self.alloc(len(data))
        self.u.mem_write(p, data)
        return p

    def rstr(self, p):
        out = bytearray()
        while True:
            pair = bytes(self.u.mem_read(p + len(out), 2))
            if pair == b'\0\0':
                return out.decode('utf-16-le')
            out += pair

    def cstring(self, text):
        """Fake CString: data pointer with the native -0xC length word."""
        data = text.encode('utf-16-le') + b'\0\0'
        block = self.alloc(0x10 + len(data))
        self.w(block, 0)
        self.w(block + 4, len(text))
        self.w(block + 8, len(text))
        self.w(block + 12, 1)
        self.u.mem_write(block + 0x10, data)
        return block + 0x10

    def esp(self):
        return self.u.reg_read(UC_X86_REG_ESP)


def expand(fmt, prefix):
    out, i, used = '', 0, False
    while i < len(fmt):
        ch = fmt[i]
        if ch != '%':
            out += ch
            i += 1
            continue
        nxt = fmt[i + 1:i + 2]
        if nxt == '%':
            out += '%'
        elif nxt == 's' and not used:
            out += prefix
            used = True
        else:
            raise ValueError('lab format outside %s/%%')
        i += 2
    return out


def relative(folder, value):
    if not value:
        return ''
    if value.startswith('/'):
        return value.lstrip('/')
    return f'{folder}/{value}' if folder else value


def build_table(m):
    """4BDFF0 layout: this+10 buckets, this+14 count 7, 4BDEE0 entry {v,name,0}."""
    count = 7
    buckets = m.alloc(4 * count)
    for i in range(count):
        m.w(buckets + 4 * i, 0)
    m.w(TABLE + 0x10, buckets)
    m.w(TABLE + 0x14, count)
    for value, name in ((0, 'DISCRETE'), (1, 'CONTINUOUS')):
        entry = m.alloc(12)
        m.w(entry, value)
        m.w(entry + 4, m.cstring(name))
        m.w(entry + 8, 0)
        holder = m.alloc(4)
        m.w(holder, m.r(entry + 4))
        h = m.call(0x405030, args=[holder], pop=0)
        node = m.alloc(8)
        slot = buckets + 4 * (h % count)
        m.w(node, entry)
        m.w(node + 4, m.r(slot))
        m.w(slot, node)


def run(precision, c, fill):
    m = Lab(precision)
    calls = []
    build_table(m)
    frame = STACK - 0x400
    m.u.mem_write(frame, bytes([fill]) * 0x200)
    for off in (0x10, 0x30, 0x44):
        m.w(frame + off, m.cstring(''))
    node = m.alloc(0x40)
    children = [m.alloc(0x10) for _ in range(max(len(c['entries']), 1))]
    data = m.alloc(0x600)
    m.u.mem_write(data, bytes([fill]) * 0x600)
    m.w(data + 8, m.wstr(c['id']))
    m.w(data + 0xb0, m.cstring(c['prefix']))
    first = m.alloc(4 + 0x24)
    m.u.mem_write(first, bytes([fill]) * (4 + 0x24))
    m.w(first, 1)
    m.call(0x4bddb0, ecx=first + 4)
    m.w(data + 0x25c, first + 4)
    m.w(data + 0x260, 0)
    m.w(data + 0x264, 1)
    m.w(0xb36530, GAMEDATA)
    state = {'child': None}

    def child_of(p):
        assert p in children, hex(p)
        return children.index(p)

    def entry_of(m, key):
        return c['entries'][child_of(m.ecx())][key]

    def count(m):
        assert m.ecx() == node and m.rstr(m.arg(1)) == 'TrackInfo'
        calls.append(['5B4220', 'TrackInfo'])
        m.ret(len(c['entries']), 4)

    def accessor(m):
        a = m.ecx()
        assert a == frame + 0x1c and m.r(a) == node and m.rstr(m.r(a + 8)) == 'TrackInfo'
        assert m.r(a + 0xc) == len(c['entries'])
        state['child'] = m.r(a + 4)
        m.ret(children[m.r(a + 4)])

    def get_string(m):
        key = m.rstr(m.arg(1))
        assert child_of(m.ecx()) == state['child']
        calls.append(['5B54D0', key])
        holder = m.alloc(4)
        m.w(holder, m.cstring(entry_of(m, key)))
        m.ret(holder, 4)

    def assign(m):
        dest, src = m.ecx(), m.arg(1)
        m.w(dest, m.r(src))
        m.ret(dest, 4)

    def fmt(m):
        out, f, prefix = m.arg(1), m.rstr(m.arg(2)), m.rstr(m.arg(3))
        assert out == frame + 0x30
        calls.append(['405D60', f, prefix])
        m.w(out, m.cstring(expand(f, prefix)))
        m.ret(0)

    def relative_path(m):
        assert m.ecx() == GAMEDATA and m.arg(1) == frame + 0x30
        value = m.rstr(m.r(m.arg(1)))
        calls.append(['4763C0', value])
        m.w(m.arg(1), m.cstring(relative(c['folder'], value)))
        m.ret(0, 4)

    def texture(m):
        assert m.arg(1) == frame + 0x30
        path, flags = m.rstr(m.r(m.arg(1))), m.arg(2)
        calls.append(['5A77E0', path, flags])
        m.ret(c['entries'][state['child']]['texture'])

    def flag(m):
        key = m.rstr(m.arg(1))
        calls.append(['5B5380', key])
        m.ret(0xabcdef00 | entry_of(m, key), 4)

    def get_float(m):
        key = m.rstr(m.arg(1))
        calls.append(['5B4AE0', key])
        m.ret_st0_f32(entry_of(m, key), pop=4)

    def point(m):
        out, key = m.arg(1), m.rstr(m.arg(2))
        calls.append(['5B4D80', key])
        x, y = entry_of(m, key)
        m.w(out, x)
        m.w(out + 4, y)
        m.ret(out, 8)

    def error(m):
        calls.append(['410C40', m.rstr(m.arg(1)), m.rstr(m.arg(2)), m.rstr(m.arg(3))])
        raise Stop()

    allocations = []

    def alloc(m):
        size = m.arg(1)
        calls.append(['6F7810', size])
        p = m.alloc(size + 16)
        m.u.mem_write(p, bytes([fill]) * (size + 16))
        allocations.append(p)
        m.ret(p)

    def free(m):
        calls.append(['6F7430', 'previous' if m.arg(1) in [first] + allocations else 'other'])
        m.ret(0)

    def lookup_store(m):
        calls.append(['store+0', m.u.reg_read(UC_X86_REG_EAX) - (1 << 32)
                      if m.u.reg_read(UC_X86_REG_EAX) >> 31 else m.u.reg_read(UC_X86_REG_EAX)])

    ptd = m.alloc(0x100)
    m.w(ptd + 0x64, m.r(0xb35444))

    def getptd(m):
        # _getptd transport: this thread's locinfo is the process __ptlocinfo
        # B35444 (static C locale; AA.exe only ever sets LC_NUMERIC).
        m.ret(ptd)

    for address, fn in ((0x9738a9, getptd), (0x5b4220, count), (0x42be10, accessor), (0x5b54d0, get_string), (0x405b80, assign),
                        (0x405d60, fmt), (0x4763c0, relative_path), (0x5a77e0, texture), (0x5b5380, flag),
                        (0x5b4ae0, get_float), (0x5b4d80, point), (0x410c40, error), (0x6f7810, alloc),
                        (0x6f7430, free)):
        m.seam(address, fn)
    m.observe(0x465eb3, lookup_store)
    u = m.u
    u.reg_write(UC_X86_REG_FPCW, {'Single24': 0x007f, 'Double53': 0x027f, 'Extended64': 0x037f}[precision])
    u.reg_write(UC_X86_REG_ESP, frame)
    u.reg_write(UC_X86_REG_EBX, node)
    u.reg_write(UC_X86_REG_EBP, data)
    u.reg_write(UC_X86_REG_ESI, 0x51515151)
    u.reg_write(UC_X86_REG_EDI, 0x61616161)
    thrown = False
    try:
        u.emu_start(0x465da2, 0x465f79, count=2_000_000)
    except Stop:
        thrown = True
    assert not m.null_accesses
    if not thrown:
        assert u.reg_read(UC_X86_REG_EIP) == 0x465f79 and m.esp() == frame
    m.verify_v14()
    array, n, capacity = m.r(data + 0x25c), m.r(data + 0x260), m.r(data + 0x264)
    entries = [bytes(m.u.mem_read(array + 0x24 * i, 0x24)) for i in range(n)]
    return dict(thrown=thrown, calls=calls, count=n, capacity=capacity, header=m.r(array - 4), entries=entries), m.visited


def entry(kind, texture_name, texture, mirrored, pcg, offset, size, fade):
    return {'track_type': kind, 'track_texture': texture_name, 'texture': texture, 'track_texture_mirrored': mirrored,
            'track_placement_pcg': pcg, 'track_offset': offset, 'track_size': size, 'fade_sec': fade}


def cases():
    foot, tire, tread = '/Effects/Tracks/footstep.tga', '/Effects/Tracks/tiretracks.tga', '/Effects/Tracks/treadtracks.tga'
    out = [
        dict(name='biped', entries=[entry('DISCRETE', foot, 0x7e001000, 1, 0x3f000000, [0x3dcccccd, 0xbdcccccd], [0x3e800000, 0x3e800000], 0x40800000),
                                    entry('DISCRETE', foot, 0x7e001000, 0, 0x3f800000, [0xbdcccccd, 0xbdcccccd], [0x3e800000, 0x3e800000], 0x40800000)]),
        dict(name='case-folded-five', entries=[entry(t, tread if i % 2 else tire, 0x7e002000 + i, i & 1, 0x3f266666, [0x3ecccccd + i, 0xbf800000],
                                                     [0x3eb33333, 0x3eb33333 + i], 0x40a00000) for i, t in
                                               enumerate(['continuous', 'Continuous', 'CONTINUOUS', 'discrete', 'DiScReTe'])]),
        dict(name='unknown-type-after-stores', entries=[entry('CONTINUOUS', tire, 0x7e003000, 0, 0, [0, 0], [0x3e800000, 0x3e800000], 0x40800000),
                                                        entry('TREAD', tread, 0x7e003010, 1, 0x3f000000, [1, 2], [3, 4], 0x40400000),
                                                        entry('DISCRETE', foot, 0x7e003020, 1, 0, [0, 0], [0, 0], 0)]),
        dict(name='empty-type', entries=[entry('', foot, 0, 1, 0, [0, 0], [0x3eaaa9fc, 0x3eaaa9fc], 0x40400000)]),
        dict(name='trailing-space-type', entries=[entry('DISCRETE ', foot, 0x7e004000, 1, 0, [0, 0], [0, 0], 0)]),
        dict(name='prefix-format-null-texture', entries=[entry('CONTINUOUS', '%s_tracks.tga', 0, 0, 0x3f59999a, [0x80000000, 0], [0x3f000000, 0x3f800000], 0x40800000),
                                                         entry('CONTINUOUS', 'relative/tracks.tga', 0x7e005000, 1, 0x3f59999a, [0, 0x80000000], [0x3f000000, 0x3f800000], 0x40800000)]),
        dict(name='none', entries=[]),
    ]
    for c in out:
        c['id'] = 'lab_' + c['name'].replace('-', '_')
        c['prefix'] = 'units/lab/base'
        c['folder'] = 'Units/Lab'
    return out


def merge(a, b):
    return [x if x == y else None for x, y in zip(a, b)]


def main():
    rows, visited = [], {}
    for precision in ('Single24', 'Double53', 'Extended64'):
        for c in cases():
            a, va = run(precision, c, FILLS[0])
            b, vb = run(precision, c, FILLS[1])
            visited.update(va)
            visited.update(vb)
            for key in ('thrown', 'calls', 'count', 'capacity', 'header'):
                assert a[key] == b[key], (c['name'], key)
            rows.append(dict(precision=precision, input=c, output=dict(
                thrown=a['thrown'], calls=a['calls'], count=a['count'], capacity=a['capacity'], header=a['header'],
                entries=[merge(x, y) for x, y in zip(a['entries'], b['entries'])])))
    by = {}
    for r in rows:
        by.setdefault(r['input']['name'], []).append(r['output'])
    for name, outs in by.items():
        assert all(o == outs[0] for o in outs), name
    pins = []
    for va, size in [(0xa08db4, 22), (0xa08d98, 28), (0xa08d68, 46), (0xa08d40, 40), (0xa08d24, 26), (0xa08d0c, 22),
                     (0xa08cf8, 18), (0xa08dcc, 20), (0xa214fc, 18), (0xa214e4, 22), (0xa08ca8, 78), (0x9cd190, 0x18), (0x4be096, 0x1d)]:
        raw = original.range(va, size)
        assert raw == modified.range(va, size)
        pins.append(dict(va=hex(va), bytes=raw.hex()))
    doc = header(__file__, __doc__)
    doc.update(rows=rows, instructions=instructions(visited), data_pins=pins)
    out = HERE / 'native-track-info.json'
    text = json.dumps(doc, indent=1) + '\n'
    if '--write' in sys.argv:
        out.write_text(text)
    else:
        assert json.loads(out.read_text()) == json.loads(text), 'fixture differs from a fresh oracle run'
    print(json.dumps(dict(rows=len(rows), instructions=len(doc['instructions']),
                          sha256=hashlib.sha256(out.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
