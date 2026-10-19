"""Oracle: command Object-definition references on the Thing DataManager [B39F58].

Runs, in both pinned images at PC24/PC53/PC64, ORIGINAL code only:

A. Registration 0x4B4E60 (the KKC_DataManager append 4BCA00 reaches at
   4BCF66 for every Thing/Unit/Projectile/VFX root): def+0C = [mgr+28],
   vector +24 append (0x766DE0 / 0x4849F0), IDS hash map +30 (0x4915F0 find,
   0x491740 insert, 0x405030 hash, 0x96CAD6 compare). A repeated IDS reaches
   the 0x410C40 throw (seam, recorded, the case stops there).
B. Reader 0x76A960 (61EBA0 flag bit1, 4C2C60 parameter entries): 0x411FA0
   reads 40C7C0([mgr+28]+1) bits (0x40D5D0); 0 <= value < [mgr+28],
   [mgr+24][value] != NULL and its vt+38 nonzero store the definition,
   anything else stores NULL. Every definition object carries the REAL
   ThingData class vtable that 4BCA00's jump table (4BD028, index =
   category+18 native_kind) constructs: vt+38 is executed natively
   (0x4F0500 returns 1 for kinds 2 and 19, 0x4C9FA0 returns 0 otherwise).
   0x855D30 range assertions (statics B39BFC/B39BFD reset per case) are
   recorded seams.
C. Writer 0x76A880 (61E6B0 flag bit1): value = def ? def+0C : [mgr+28],
   width 40C7C0([mgr+28]+1); 0x40D420 is a recorded seam (the packet
   buffer writer is outside this boundary).

Seams: 6F7810/6F7430 heap, 9738A9 _getptd (C locale B35444), 855D30,
410C40, 40D420. The kind->vtable table is the static join of 4BD028 with
each case's constructor vtable store (FINDINGS in README.txt).

  python native_object_reference.py --write   (refuses to overwrite)
  python native_object_reference.py           (replay, byte-identical)
"""
from pathlib import Path
import random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import recruit_lab as lab  # noqa: E402
nm = lab.nm

OUT = HERE / 'native-object-reference.json'
# 4BD028[kind] -> case -> constructor -> final vtable store, computed from the
# original image (static join; the constructors themselves are not executed).
def kind_vtables():
    md = lab.Cs(lab.CS_ARCH_X86, lab.CS_MODE_32)
    table = struct.unpack('<24I', nm.original.range(0x4BD028, 96))
    out = []
    for case in table:
        ctor = None
        for i in md.disasm(nm.original.range(case, 0x40), case):
            if i.mnemonic == 'call' and i.op_str.startswith('0x') and int(i.op_str, 16) != 0x6F7810:
                ctor = int(i.op_str, 16)
                break
        stores = []
        for i in md.disasm(nm.original.range(ctor, 0x600), ctor):
            if i.mnemonic == 'ret':
                break
            if i.mnemonic == 'mov' and i.op_str.startswith('dword ptr [') and '], 0x' in i.op_str:
                reg = i.op_str.split('[')[1].split(']')[0]
                v = int(i.op_str.split(', ')[1], 16)
                if reg in ('esi', 'edi', 'ebx', 'ecx', 'eax', 'ebp', 'edx') and 0x9F0000 <= v < 0xB00000:
                    stores.append(v)
        out.append({'case': case, 'constructor': ctor, 'vtable': stores[-1]})
    return out


KINDS = kind_vtables()
KIND_VTABLES = [k['vtable'] for k in KINDS]
FUNCTIONS = {'76A960': (0x76a960, 0x76a9bc), '76A880': (0x76a880, 0x76a8ba), '411FA0': (0x411fa0, 0x412068),
             '40C7C0': (0x40c7c0, 0x40c895), '40D5D0': (0x40d5d0, 0x40d6a0), '4B4E60': (0x4b4e60, 0x4b4ef8),
             '4F0500': (0x4f0500, 0x4f0506), '4C9FA0': (0x4c9fa0, 0x4c9fa3)}


class Machine(lab.Lab):
    def __init__(self, precision, image):
        super().__init__(precision, image)
        self.events = []
        self.seam(0x6F7810, lambda m: m.ret(m.alloc(max(m.arg(1), 4))))
        self.seam(0x6F7430, lambda m: (m.frees.append(m.arg(1)), m.ret(0)))
        self.ptd = self.alloc(0x100)
        self.seam(0x9738A9, self._ptd)
        self.seam(0x855D30, self._assert)
        self.seam(0x410C40, self._throw)
        self.seam(0x40D420, self._write)
        self.w(0xB43B70, 0)
        self.mark = self.heap

    def _ptd(self, m):
        m.w(m.ptd + 0x64, m.r(0xB35444))
        m.ret(m.ptd)

    def _assert(self, m):
        m.events.append(['assert', m.text(m.arg(1)), m.arg(3)])
        m.ret(0)

    def _throw(self, m):
        m.events.append(['throw', m.text(m.arg(1))])
        raise Thrown()

    def _write(self, m):
        m.events.append(['write', m.r(m.arg(2)), m.arg(3)])
        m.ret(0)

    def thing_manager(self):
        mgr = self.alloc(0x40)
        self.w(0xB39F58, mgr)
        return mgr

    def register(self, names):
        """Part A: 4B4E60 per definition in order."""
        self.heap = self.mark
        self.events = []
        mgr = self.thing_manager()
        self.w(mgr + 0x34, 17)  # bucket count (the native constructor's +34)
        defs, out = [], []
        for name in names:
            d = self.alloc(0x40)
            self.w(d + 8, self.cstring(name))
            self.w(d + 0xC, 0xFFFFFFFF)
            try:
                self.call(0x4B4E60, ecx=mgr, args=(d,), pop=4)
                out.append({'ids': name, 'index_0c': self.r(d + 0xC)})
            except Exception as e:  # the 410C40 seam raised inside the hook
                if not (isinstance(e, Thrown) or isinstance(e.__cause__, Thrown)):
                    raise
                out.append({'ids': name, 'thrown': True})
                break
            defs.append(d)
        count = self.r(mgr + 0x28)
        vector = [self.text(self.r(self.r(self.r(mgr + 0x24) + 4 * i) + 8)) for i in range(count)]
        return {'registrations': out, 'count': count, 'vector': vector, 'events': self.events}

    def read(self, case):
        """Part B: 76A960 on one encoded value."""
        self.heap = self.mark
        self.events = []
        self.w(0xB39BFC, 0)  # both one-shot assertion statics (B39BFC/B39BFD bytes)
        mgr = self.thing_manager()
        entries = []
        for kind in case['kinds']:
            if kind is None:
                entries.append(0)
                continue
            d = self.alloc(0x40)
            self.w(d, KIND_VTABLES[kind])
            entries.append(d)
        self.w(mgr + 0x24, self.array(entries))
        self.w(mgr + 0x28, case['count'])
        data = self.alloc(16)
        bits = case['bits']
        word = case['value'] << case['bit']
        self.u.mem_write(data, struct.pack('<Q', word | case['noise'] << (case['bit'] + bits)))
        stream = self.alloc(0x10)
        self.w(stream + 4, data)
        self.w(stream + 8, 0)
        self.w(stream + 0xC, case['bit'])
        out = self.alloc(4)
        self.w(out, 0xDEADBEEF)
        self.call(0x76A960, args=(stream, out), pop=0)
        p = self.r(out)
        return {'resolved': None if p == 0 else entries.index(p), 'byte': self.r(stream + 8),
                'bit': self.r(stream + 0xC), 'events': self.events}

    def write(self, case):
        """Part C: 76A880 on one definition reference."""
        self.heap = self.mark
        self.events = []
        mgr = self.thing_manager()
        self.w(mgr + 0x28, case['count'])
        ref = self.alloc(4)
        if case['index_0c'] is None:
            self.w(ref, 0)
        else:
            d = self.alloc(0x40)
            self.w(d + 0xC, case['index_0c'])
            self.w(ref, d)
        stream = self.alloc(0x10)
        self.call(0x76A880, args=(stream, ref), pop=0)
        return {'events': self.events}


class Thrown(Exception):
    pass


def width(n):
    """40C7C0 for the guarded range used here (n <= 0x10000)."""
    b = 0
    while (1 << b) < n:
        b += 1
    return b


def cases(rng):
    reg = [['rus_corps'], ['a', 'b', 'c'], ['rus_corps', 'rus_truck_engineer', 'regiment_rus_hq_infantry', 'x' * 30],
           ['dup', 'other', 'DUP'], ['k%d' % i for i in range(40)], []]
    reads = []
    for count in [0, 1, 2, 24, 25, 31, 32, 1000, 1084, 0xFFFF]:
        bits = width(count + 1)
        listed = min(count, 48)
        for _ in range(12 if count > 2 else 6):
            kinds = [rng.choice(list(range(24)) + [None]) for _ in range(listed)]
            if listed:
                kinds[0] = 2
            value = rng.choice([0, count, count + 1, (1 << bits) - 1, rng.randrange(0, max(1, min(count, listed))),
                                rng.randrange(0, 1 << bits) if bits else 0])
            value &= (1 << bits) - 1 if bits else 0
            if value < count and value >= listed:
                value = rng.randrange(0, listed) if listed else 0
            reads.append({'count': count, 'bits': bits, 'kinds': kinds, 'value': value,
                          'bit': rng.randrange(0, 8), 'noise': rng.getrandbits(8)})
    for kind in range(24):
        reads.append({'count': 3, 'bits': 2, 'kinds': [kind, None, 19], 'value': 0, 'bit': 0, 'noise': 0})
    writes = [{'count': c, 'index_0c': i} for c in [0, 1, 24, 1084, 0xFFFF]
              for i in [None, 0, max(c - 1, 0), c, 0x12345678]]
    return reg, reads, writes


def main():
    write = '--write' in sys.argv
    rows, visited = [], {}
    for label, image in lab.IMAGES.items():
        for pc in nm.PRECISIONS:
            reg, reads, writes = cases(random.Random(0x76A960))
            m = Machine(pc, image)
            for names in reg:
                rows.append({'image': label, 'precision': pc, 'part': 'register', 'input': names, 'output': m.register(names)})
            for c in reads:
                rows.append({'image': label, 'precision': pc, 'part': 'read', 'input': c, 'output': m.read(c)})
            for c in writes:
                rows.append({'image': label, 'precision': pc, 'part': 'write', 'input': c, 'output': m.write(c)})
            m.verify_v14()
            visited.update(m.visited)
    doc = nm.header(__file__, 'unicorn original 4B4E60 / 76A960 / 76A880')
    doc['scope'] = __doc__
    doc['kind_vtables'] = [{'kind': k, 'case': hex(v['case']), 'constructor': hex(v['constructor']),
                            'vtable': hex(v['vtable'])} for k, v in enumerate(KINDS)]
    doc['coverage'] = lab.coverage(FUNCTIONS, visited)
    doc['rows'] = rows
    doc['instructions'] = nm.instructions(visited)
    lab.write_or_replay(OUT, doc, write)
    print({k: f"{v['executed']}/{v['instructions']}" for k, v in doc['coverage'].items()}, len(rows), 'rows')


if __name__ == '__main__':
    main()
