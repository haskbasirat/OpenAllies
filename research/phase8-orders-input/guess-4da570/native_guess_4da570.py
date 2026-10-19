"""Oracle producer for KKC_GActorCommand::GuessCommandData 0x4DA570.

Runs the ORIGINAL 0x4DA570 (and, natively inside it, 0x4D44E0, 0x579FD0,
0x579F70, 0x4364C0, 0x4D1490, 0x763B90, 0x497220, 0x405030, 0x96CAD6) in the
original AA.exe and in the pinned v14 image, at PC24/PC53/PC64, over
generated inputs:

* a fake 0x40-byte KKC_GActorCommand (+0C flags, +10 target GTID, +14 object),
* a B364C0 GTID table (generation word table+4+2*slot, object
  table+20004+4*slot),
* a fake GameData at [B36530] whose +298 holds the 39 definition pointers
  (+0C type, +2C requirement flags, +3C/+3F/+40 control bytes; the other
  control bytes are random noise to prove they are not read),
* actor/target objects with synthetic vtables: +34, +108, +138, +13C, +140
  are Python seams returning case values (raw EAX; the caller tests AL);
  every other slot traps,
* Organization (+7C: +28 members, +2C count), Element (+80: +1C byte), Body
  (+60: +10/+14 binary32), Settlement (+98: +14 parent, +18/+1C vector),
  Construction (+9C), Builder (+A8), Actor+100 words and ActorData (+4E0,
  +504 head-first list) laboratory memory,
* the ActorCommand enum at B3CBA0 constructed by the ORIGINAL static
  constructor 0x4DA9D0(1)/0x4DA2F0/0x4DA1E0/0x496F60 (CString seams only),
* the variable lookup 0x4B2E60 (registry B39938) as a seam returning case
  records: kind 1 SwapRightClickWithAltRightClick (byte at +0) and kind 4
  HotkeyAltRightClickCommand (+10 -> UTF-16 text).

Seams: 0x6F7810 operator new (laboratory heap), 0x6F7430 free (recorded),
0x401B00 CString(wchar*) and 0x4012C0 CStringData clone (laboratory string
headers, refcount at -4, manager vtable +4 free), 0x9738A9 _getptd (returns
a ptd whose +64 is the image's own [B35444] initial locale info, i.e. the CRT
"C" locale, lc_handle[LC_CTYPE]=0), 0x4B2E60, 0x404F60 (recorded). Rows whose
native run dereferences null or overflows the laboratory stack are kept with
`fault` and must be errors in Rust.

  python native_guess_4da570.py --write   # create (refuses to overwrite)
  python native_guess_4da570.py           # replay: byte-identical check
"""
from pathlib import Path
import hashlib, json, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm  # noqa: E402
from unicorn import UcError  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ESP  # noqa: E402

OUT = HERE / 'native-guess-4da570.json'
CONTROLS = ROOT / 'research/skirmish-20261016-player-orders/command-ui/native-actual-controls.json'
ENTRY = 0x4DA570
STATICS = (0xB3CB94, 0xB3CB98, 0xB3CB9C)
ENUM = 0xB3CBA0
FUNCTIONS = {'4DA570': (0x4da570, 0x4da9c1), '4D44E0': (0x4d44e0, 0x4d4819), '579FD0': (0x579fd0, 0x57a02c),
             '579F70': (0x579f70, 0x579fcd), '4364C0': (0x4364c0, 0x436509), '4D1490': (0x4d1490, 0x4d14bb),
             '763B90': (0x763b90, 0x763bb2), '497220': (0x497220, 0x497272), '405030': (0x405030, 0x40506f),
             '96CAD6': (0x96cad6, 0x96cb83), '4DA9D0': (0x4da9d0, 0x4daa9a), '4DA2F0': (0x4da2f0, 0x4da563),
             '4DA1E0': (0x4da1e0, 0x4da2e4), '496F60': (0x496f60, 0x49700f), '766DE0': (0x766de0, 0x766e37)}
SWAP, HOTKEY = 'SwapRightClickWithAltRightClick', 'HotkeyAltRightClickCommand'


def wide(m, p, limit=400):
    out = []
    for i in range(limit):
        c = struct.unpack('<H', m.u.mem_read(p + 2 * i, 2))[0]
        if c == 0:
            break
        out.append(c)
    return out


class Lab(nm.Machine):
    """native_machine.Machine over either pinned image, plus the seams above."""

    def __init__(self, precision, image):
        super().__init__(precision)
        self.image = image
        if image is nm.modified:
            u = self.u
            u.mem_unmap(nm.original.base, (nm.original.image_size + 0xfff) & ~0xfff)
            u.mem_map(nm.modified.base, (nm.modified.image_size + 0xfff) & ~0xfff)
            for vs, rva, raw, off in nm.modified.sections:
                u.mem_write(nm.modified.base + rva, nm.modified.b[off:off + raw])
        self.frees = []
        self.strings = []
        self.lookups = []
        self.logs = []
        self.events = []
        self.records = {}
        self.names = {}
        self.seam(0x6F7810, lambda m: m.ret(m.alloc(max(m.arg(1), 4))))
        self.seam(0x6F7430, self._free)
        self.seam(0x401B00, self._cstring)
        self.seam(0x4012C0, self._clone)
        self.seam(0x9738A9, self._ptd)
        self.seam(0x4B2E60, self._lookup)
        self.seam(0x404F60, self._log)
        self.manager = self.alloc(0x20)
        self.w(self.manager, self.array([0, self.stub(self._string_free)] + [0] * 6))
        self.ptd = self.alloc(0x100)
        self.trap = self.stub(self._trap)
        self.vt = {name: self.stub(fn) for name, fn in
                   [('34', self._vt34), ('108', self._vt108), ('138', self._rel('138')), ('13c', self._rel('13c')),
                    ('140', self._rel('140'))]}
        self.observe(0x4DA770, lambda m: m.events.append(['swap']))
        self.observe(0x4DA7CA, lambda m: m.events.append(['hotkey']))
        self.observe(0x4DA8BB, self._candidates)
        # Original enum static constructor 9CD7E0: 4DA9D0(1) on B3CBA0.
        self.call(0x4DA9D0, ecx=ENUM, args=(1,), pop=4)
        count = self.r(ENUM + 8)
        self.enum_entries = []
        for i in range(count):
            e = self.r(self.r(ENUM + 4) + 4 * i)
            self.enum_entries.append([self.r(e), ''.join(map(chr, wide(self, self.r(e + 4)))), self.r(e + 8)])
        self.table = self.alloc(0x20004 + 4 * 0x10000)
        self.w(0xB364C0, self.table)
        self.game = self.alloc(0x300)
        self.w(0xB36530, self.game)
        self.mark = self.heap
        self.case = None

    def alloc(self, size, align=16):
        # The laboratory heap is reused per case: every block starts zeroed.
        p = super().alloc(size, align)
        self.u.mem_write(p, bytes(max(size, 4)))
        return p

    # seams ----------------------------------------------------------------
    def _free(self, m):
        m.frees.append(m.arg(1))
        m.ret(0)

    def _cstring(self, m):
        this, src = m.ecx(), m.arg(1)
        chars = wide(m, src)
        m.strings.append(''.join(map(chr, chars)))
        h = m.alloc(0x10 + 2 * (len(chars) + 1))
        m.w(h, m.manager)
        m.w(h + 4, len(chars))
        m.w(h + 8, len(chars))
        m.w(h + 12, 1)
        for i, c in enumerate(chars):
            m.u.mem_write(h + 0x10 + 2 * i, struct.pack('<H', c))
        m.w(this, h + 0x10)
        m.ret(this, pop=4)

    def _clone(self, m):
        h = m.arg(1)
        m.w(h + 12, m.r(h + 12) + 1)
        m.ret(h)

    def _string_free(self, m):
        m.frees.append(('string', m.arg(1)))
        m.ret(0, pop=4)

    def _ptd(self, m):
        m.w(m.ptd + 0x64, m.r(0xB35444))
        m.ret(m.ptd)

    def _lookup(self, m):
        name = ''.join(map(chr, wide(m, m.arg(1))))
        kind = m.arg(2)
        m.lookups.append([name, kind, m.ecx()])
        m.ret(m.records.get((name, kind), 0), pop=8)

    def _log(self, m):
        args = [m.arg(i) for i in range(1, 7)]
        m.logs.append({'logger': args[0], 'file': ''.join(map(chr, wide(m, args[1]))), 'line': args[2],
                       'function': ''.join(map(chr, wide(m, args[3]))),
                       'format': ''.join(map(chr, wide(m, args[4]))), 'handle': args[5]})
        m.events.append(['log', args[5]])
        m.ret(0)

    def _trap(self, m):
        raise AssertionError('unexpected virtual call')

    def _vt34(self, m):
        o = m.names[m.ecx()]
        i = m.arg(1)
        m.events.append(['34', o, i])
        m.ret(m.case['objects'][str(o)]['allowed34'][i], pop=4)

    def _vt108(self, m):
        o = m.names[m.ecx()]
        m.events.append(['108', o])
        k = m.case['objects'][str(o)]['kingdom108']
        m.ret(m.kingdoms[k] if k else 0)

    def _rel(self, name):
        def fn(m):
            o = m.names[m.ecx()]
            k = m.kingdom_names.get(m.arg(1), 0)
            m.events.append([name, o, k])
            m.ret(m.case['objects'][str(o)]['rel' + name][str(k)], pop=4)
        return fn

    def _candidates(self, m):
        sp = m.u.reg_read(UC_X86_REG_ESP)
        node, out = m.r(sp + 0x28), []
        while node:
            out.append(m.def_names.get(m.r(node), m.r(node)))
            node = m.r(node + 4)
        m.events.append(['candidates', out])

    # case memory ------------------------------------------------------------
    def build(self, case):
        self.heap = self.mark
        self.case = case
        for a in STATICS:
            self.w(a, 0)
        self.events, self.lookups, self.logs, self.frees, self.strings = [], [], [], [], []
        self.u.mem_write(self.table, bytes(0x20004 + 4 * 0x10000))
        defs = []
        for d in case['definitions']:
            p = self.alloc(0x48)
            self.u.mem_write(p, bytes(case['noise'][:0x48]))
            self.w(p + 0x0C, d['type'])
            self.w(p + 0x2C, d['flags'])
            for off, key in [(0x3C, 'b3c'), (0x3F, 'b3f'), (0x40, 'b40')]:
                self.wb(p + off, d[key])
            defs.append(p)
        self.def_pointers = {p: i for i, p in enumerate(defs)}
        self.w(self.game + 0x298, self.array(defs))
        self.kingdoms = {int(k): self.alloc(0x10) for k in case['kingdoms']}
        self.kingdom_names = {v: k for k, v in self.kingdoms.items()}
        # ActorData definitions.
        self.defp = {int(k): self.alloc(0x600) for k in case['actor_data']}
        self.def_names = {v: k for k, v in self.defp.items()}
        for k, d in case['actor_data'].items():
            p = self.defp[int(k)]
            self.w(p + 0x4E0, self.defp[d['r4e0']] if d['r4e0'] else 0)
            head = 0
            for target in reversed(d['builds']):
                n = self.alloc(8)
                self.w(n, self.defp[target])
                self.w(n + 4, head)
                head = n
            self.w(p + 0x504, head)
        self.objp = {int(k): self.alloc(0x600) for k in case['objects']}
        self.names = {v: k for k, v in self.objp.items()}
        vt = self.array([self.trap] * 0x80)
        for off, name in [(0x34, '34'), (0x108, '108'), (0x138, '138'), (0x13C, '13c'), (0x140, '140')]:
            self.w(vt + off, self.vt[name])
        for k, o in case['objects'].items():
            p = self.objp[int(k)]
            self.w(p, vt)
            self.w(p + 4, self.defp[o['def4']] if o['def4'] else 0)
            if o['body'] is not None:
                b = self.alloc(0x40)
                self.w(b + 0x10, o['body'][0])
                self.w(b + 0x14, o['body'][1])
                self.w(p + 0x60, b)
            if o['org'] is not None:
                org = self.alloc(0x100)
                self.w(org + 0x28, self.array([self.objp[x] if x else 0 for x in o['org']]))
                self.w(org + 0x2C, len(o['org']))
                self.w(p + 0x7C, org)
            if o['element'] is not None:
                e = self.alloc(0x40)
                self.wb(e + 0x1C, o['element'])
                self.w(p + 0x80, e)
            if o['settlement'] is not None:
                s = self.alloc(0x40)
                st = o['settlement']
                self.w(s + 0x14, self.objp[st['parent']] if st['parent'] else 0)
                self.w(s + 0x18, self.array([self.objp[x] if x else 0 for x in st['children']]))
                self.w(s + 0x1C, len(st['children']))
                self.w(p + 0x98, s)
            if o['construction']:
                self.w(p + 0x9C, self.alloc(0x10))
            if o['builder']:
                self.w(p + 0xA8, self.alloc(0x10))
            self.w(p + 0x100, o['flags100'])
        for e in case['table']:
            self.u.mem_write(self.table + 4 + 2 * e['slot'], struct.pack('<H', e['generation']))
            self.w(self.table + 0x20004 + 4 * e['slot'], self.objp[e['object']] if e['object'] else 0)
        self.set_variables(case['variables'])
        c = case['command']
        self.cmd = self.alloc(0x40)
        self.w(self.cmd, 0xA28450)
        self.w(self.cmd + 0x0C, c['flags'])
        self.w(self.cmd + 0x10, c['handle'])
        self.w(self.cmd + 0x14, 0x5A5A0014)

    def set_variables(self, v):
        self.records = {}
        if v['swap'] is not None:
            r = self.records[(SWAP, 1)] = self.records.get((SWAP, 1)) or self.alloc(0x20)
            self.wb(r, v['swap'])
        if v['hotkey'] is not None:
            r = self.records[(HOTKEY, 4)] = self.alloc(0x20)
            text = self.alloc(2 * (len(v['hotkey']) + 1))
            for i, ch in enumerate(v['hotkey']):
                self.u.mem_write(text + 2 * i, struct.pack('<H', ch))
            self.w(r + 0x10, text)

    def run(self, actor, arg2):
        self.events = []
        try:
            value = self.call(ENTRY, ecx=self.cmd, args=(self.objp[actor], arg2), pop=8, count=400_000)
        except (AssertionError, UcError) as e:
            nulls = [list(map(hex, a)) for a in self.null_accesses]
            self.null_accesses = []
            return {'fault': {'null': nulls, 'error': type(e).__name__}, 'events': self.events}
        flags = self.r(self.cmd + 0x0C)
        obj = self.r(self.cmd + 0x14)
        return {'fault': None, 'result': self.def_pointers.get(value) if value else None,
                'unknown_return': value not in self.def_pointers and value != 0,
                'flags': flags, 'object14': None if obj == 0x5A5A0014 else self.def_names.get(obj, obj),
                'events': self.events}


# case generation ----------------------------------------------------------------
def actual_definitions():
    run = json.loads(CONTROLS.read_text())['runs'][0]
    by = {a['ids']: a['ui_flags_3c_to_41_and_position_44'] for a in run['authored']}
    out = []
    for d in run['definitions']:
        c = by[d['ids']]
        out.append({'ids': d['ids'], 'type': d['mind_type'], 'flags': d['behavior_flags'],
                    'b3c': c[0], 'b3f': c[3], 'b40': c[4]})
    return out


ACTUAL = actual_definitions()
EAX_TRUE = [1, 1, 1, 0x101, 0xFFFFFF01, 0x80]
EAX_FALSE = [0, 0, 0, 0x100, 0xFFFFFF00]
MASK_BITS = [0x10, 0x20, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000]
FLOATS = [0x3F800000, 0x40000000, 0x3F000000, 0x00000000, 0x80000000, 0x7FC00000, 0x7F800001, 0x00000001,
          0xFF800000, 0x7F800000, 0x42C80000, 0x42C80001]


def eax(rng, truth):
    return rng.choice(EAX_TRUE if truth else EAX_FALSE)


def synthetic_definitions(rng):
    out = []
    for i in range(39):
        flags = 0
        for b in MASK_BITS:
            if rng.random() < 0.35:
                flags |= b
        if rng.random() < 0.2:
            flags |= rng.choice([0x1, 0x40, 0x80, 0x10000, 0x40000, 0x50000, 0x4000, 0x8000, 0x1000000])
        out.append({'ids': None, 'type': rng.choice([i, i, i, 4, rng.randrange(0, 64)]), 'flags': flags,
                    'b3c': rng.choice([0, 0, 1, 2]), 'b3f': rng.choice([0, 1, 1, 0x80]),
                    'b40': rng.choice([0, 1, 1, 0x40])})
    return out


def obj(rng, **kw):
    o = {'def4': None, 'body': None, 'org': None, 'element': None, 'settlement': None, 'construction': False,
         'builder': False, 'flags100': rng.choice([0, 0x200000, 0x1, 0x200001, 0xFFFFFFFF, 0xFFDFFFFF]),
         'kingdom108': 0, 'allowed34': [0] * 39, 'rel138': {}, 'rel13c': {}, 'rel140': {}}
    o.update(kw)
    return o


ENUM_NAMES = [d['ids'] for d in ACTUAL]
HOTKEYS = ['retreat', 'RETREAT', 'Kill', 'move', 'MoVe', 'construct', 'group_behavior_disable', 'unknown', '',
           'retreat ', ' retreat', 'retreat\0kill', 'killé', 'kİll', 'stop', 'repair', 'guard']


def text(s):
    return [ord(c) for c in s]


def random_case(rng, kind):
    case = {'kind': kind, 'noise': [rng.randrange(256) for _ in range(0x48)]}
    case['definitions'] = ACTUAL if rng.random() < 0.6 else synthetic_definitions(rng)
    nk = rng.choice([0, 1, 2])
    case['kingdoms'] = [str(k) for k in range(1, nk + 1)]
    kingdoms = [0] + list(range(1, nk + 1))
    nd = rng.randrange(1, 7)
    case['actor_data'] = {}
    for d in range(1, nd + 1):
        case['actor_data'][str(d)] = {'r4e0': rng.choice([None] + list(range(1, nd + 1))),
                                      'builds': [rng.randrange(1, nd + 1) for _ in range(rng.choice([0, 0, 1, 2, 3]))]}
    objects = {}
    n = rng.randrange(2, 7)
    for k in range(1, n + 1):
        objects[str(k)] = obj(rng, def4=rng.randrange(1, nd + 1))
    actor, target = 1, rng.choice([1] + list(range(2, n + 1)))
    for k in objects:
        o = objects[k]
        o['kingdom108'] = rng.choice(kingdoms)
        if rng.random() < 0.5:
            o['body'] = [rng.choice(FLOATS), rng.choice(FLOATS)]
        o['construction'] = rng.random() < 0.3
        o['element'] = rng.choice([None, 0, 1, 1, 0x80])
        o['builder'] = rng.random() < 0.4
    # Organizations and settlements over later objects only (finite natively).
    for k in range(1, n + 1):
        later = list(range(k + 1, n + 1))
        if later and rng.random() < 0.35:
            objects[str(k)]['org'] = [rng.choice([None] + later) for _ in range(rng.randrange(0, 4))]
        if later and rng.random() < 0.2:
            objects[str(k)]['settlement'] = {'parent': rng.choice([None] + later),
                                             'children': [rng.choice(later) for _ in range(rng.randrange(0, 3))]}
    a = objects[str(actor)]
    density = rng.choice([0.1, 0.3, 0.6, 1.0])
    a['allowed34'] = [eax(rng, rng.random() < density) for _ in range(39)]
    for k in kingdoms:
        a['rel138'][str(k)] = eax(rng, rng.random() < 0.4)
        a['rel13c'][str(k)] = eax(rng, rng.random() < 0.4)
        a['rel140'][str(k)] = eax(rng, rng.random() < 0.4)
    flags = 0
    for bit in [0, 2, 3, 10]:
        if rng.random() < 0.5:
            flags |= 1 << bit
    if rng.random() < 0.3:
        flags |= rng.choice([0x2, 0x10, 0x40, 0x80, 0x200, 0x800, 0x10000, 0x80000000])
    slot = rng.randrange(1, 0x10000)
    gen = rng.randrange(0, 0x10000)
    tmode = rng.choice(['ok', 'ok', 'ok', 'generation', 'null'])
    case['table'] = [{'slot': slot, 'generation': gen if tmode != 'generation' else gen ^ 1,
                      'object': None if tmode == 'null' else target}]
    case['command'] = {'flags': flags, 'handle': (gen << 16) | slot if rng.random() < 0.9 else rng.randrange(1 << 32)}
    case['variables'] = {'swap': rng.choice([0, 0, 1, 2, 0x80]),
                         'hotkey': text(rng.choice(HOTKEYS))}
    case['objects'] = objects
    case['actor'] = actor
    case['arg2'] = rng.choice([0, 1])
    return case


def construct_case(rng):
    """Type-4 branch: actual construct, target only, single selection."""
    case = random_case(rng, 'construct')
    case['definitions'] = ACTUAL
    case['command']['flags'] = 1 | rng.choice([0, 0x400, 0x10, 0x2, 0x40])
    t = case['table'][0]
    if rng.random() < 0.85:
        t['generation'] = case['command']['handle'] >> 16
        t['slot'] = case['command']['handle'] & 0xffff
        if t['object'] is None and rng.random() < 0.7:
            t['object'] = rng.choice(list(map(int, case['objects'])))
    a = case['objects']['1']
    a['allowed34'][4] = eax(rng, True)
    a['builder'] = rng.random() < 0.8
    case['arg2'] = 0 if rng.random() < 0.9 else 1
    case['variables']['swap'] = rng.choice([0, 0, 1])
    if rng.random() < 0.5:
        # Matching required-object definition reachable through the lists.
        target = case['table'][0]['object']
        if target:
            td = case['objects'][str(target)]['def4']
            if td:
                cand = rng.choice(list(case['actor_data']))
                case['actor_data'][cand]['r4e0'] = td
                owner = rng.choice(list(case['objects']))
                od = case['objects'][owner]['def4']
                if od:
                    case['actor_data'][str(od)]['builds'].append(int(cand))
    return case


def fault_cases(rng):
    out = []
    # Missing variable records (4BE150 [null], [null+10]).
    c = random_case(rng, 'missing-swap')
    c['variables']['swap'] = None
    out.append(c)
    c = random_case(rng, 'missing-hotkey')
    c['variables']['hotkey'] = None
    c['variables']['swap'] = 0
    c['command']['flags'] |= 0x400
    out.append(c)
    # Construct without a resolvable target at 4DA8EB ([null+4]).
    for flags in (0, 0x400, 0x10):
        c = construct_case(rng)
        c['command']['flags'] = flags
        c['command']['handle'] = 0
        out.append(c)
    # Element-less filtered member, null Settlement child, recursion cycle.
    for mode in ('element', 'child', 'cycle', 'cycle-org'):
        c = construct_case(rng)
        objs = c['objects']
        objs['9'] = obj(rng, def4=None)
        objs['10'] = obj(rng, def4=None, element=1)
        if mode == 'element':
            objs['1']['org'] = [10, None, 9]
        elif mode == 'child':
            objs['1']['settlement'] = {'parent': None, 'children': [10, None]}
        elif mode == 'cycle':
            objs['1']['settlement'] = {'parent': 10, 'children': []}
            objs['10']['settlement'] = {'parent': 1, 'children': []}
        else:
            objs['1']['org'] = [10]
            objs['10']['org'] = [1]
            objs['1']['element'] = 1
        out.append(c)
    return out


def repeat_cases(rng):
    """Two calls on one machine: B3CB9C caches the record pointers only."""
    out = []
    for i in range(8):
        c = random_case(rng, 'repeat')
        c['command']['flags'] = rng.choice([0x400, 0x404, 0x401, 0x405, 0])
        c['second_variables'] = {'swap': rng.choice([0, 1]), 'hotkey': text(rng.choice(HOTKEYS))}
        out.append(c)
    return out


def cases():
    rng = random.Random(0x4DA570)
    out = [random_case(rng, 'random') for _ in range(420)]
    out += [construct_case(rng) for _ in range(160)]
    # Location/actor branches with the actual table and every hotkey.
    for h in HOTKEYS:
        for flags in (0x4, 0x404, 0x1, 0x401, 0x5, 0x405, 0xC, 0x40C):
            c = random_case(rng, 'hotkey')
            c['definitions'] = ACTUAL
            c['variables']['hotkey'] = text(h)
            c['variables']['swap'] = 0
            c['command']['flags'] = flags
            c['objects']['1']['allowed34'] = [1] * 39
            out.append(c)
    out += fault_cases(rng)
    out += repeat_cases(rng)
    out += settlement_cases(rng)
    return out


def settlement_cases(rng):
    """4D44E0 Settlement+18 children and +14 parent merges with new values."""
    out = []
    for _ in range(24):
        c = construct_case(rng)
        c['kind'] = 'settlement'
        objs = c['objects']
        n, nd = len(objs), len(c['actor_data'])
        pid, cid = n + 1, n + 2
        c['actor_data'][str(nd + 1)] = {'r4e0': None, 'builds': []}
        c['actor_data'][str(nd + 2)] = {'r4e0': rng.choice([None, 1]), 'builds': [nd + 1, rng.randrange(1, nd + 2)]}
        objs[str(pid)] = obj(rng, def4=nd + 2, builder=True, element=1)
        objs[str(cid)] = obj(rng, def4=rng.randrange(1, nd + 3), builder=rng.random() < 0.7, element=1)
        objs['1']['settlement'] = {'parent': pid, 'children': [cid] if rng.random() < 0.6 else []}
        objs['1']['allowed34'][4] = 1
        out.append(c)
    return out


def enum_rows(m):
    rows = []
    for h in HOTKEYS + ENUM_NAMES + [n.upper() for n in ENUM_NAMES]:
        units = text(h)
        p = m.alloc(2 * (len(units) + 1))
        for i, ch in enumerate(units):
            m.u.mem_write(p + 2 * i, struct.pack('<H', ch))
        cell = m.array([p])
        node = m.call(0x497220, ecx=ENUM + 0x10, args=(cell,), pop=4)
        rows.append({'name': units, 'value': m.r(m.r(node)) if node else None})
    return rows


def produce():
    all_cases = cases()
    runs = []
    visited = {}
    enum_reference = None
    for image_name, image in (('original', nm.original), ('v14', nm.modified)):
        for precision in nm.PRECISIONS:
            m = Lab(precision, image)
            if enum_reference is None:
                enum_reference = m.enum_entries
            assert m.enum_entries == enum_reference
            rows = []
            for case in all_cases:
                m.build(case)
                first = m.run(case['actor'], case['arg2'])
                first['lookups'], first['logs'] = m.lookups, m.logs
                row = {'output': first}
                if 'second_variables' in case and first['fault'] is None:
                    sv = case['second_variables']
                    for (name, kind), r in m.records.items():
                        if name == SWAP:
                            m.wb(r, sv['swap'])
                        else:
                            text_p = m.alloc(2 * (len(sv['hotkey']) + 1))
                            for i, ch in enumerate(sv['hotkey']):
                                m.u.mem_write(text_p + 2 * i, struct.pack('<H', ch))
                            m.w(r + 0x10, text_p)
                    m.lookups, m.logs = [], []
                    m.w(m.cmd + 0x0C, case['command']['flags'])
                    m.w(m.cmd + 0x14, 0x5A5A0014)
                    second = m.run(case['actor'], case['arg2'])
                    second['lookups'], second['logs'] = m.lookups, m.logs
                    row['second'] = second
                rows.append(row)
            enums = enum_rows(m)
            m.verify_v14()
            visited.update(m.visited)
            runs.append({'image': image_name, 'precision': precision, 'rows': rows, 'enum_lookups': enums})
    base = runs[0]
    for r in runs[1:]:
        assert r['rows'] == base['rows'], (r['image'], r['precision'])
        assert r['enum_lookups'] == base['enum_lookups']
    coverage = {}
    for name, (lo, hi) in FUNCTIONS.items():
        coverage[name] = sorted(hex(a) for a in visited if lo <= a < hi)
    head = nm.header(__file__, 'Unicorn execution of original 0x4DA570 with explicit seams (see producer docstring)')
    head.update({'entry': hex(ENTRY), 'enum_entries': enum_reference, 'definitions_actual': ACTUAL,
                 'runs': [{'image': r['image'], 'precision': r['precision']} for r in runs],
                 'identical_across_runs': True, 'cases': all_cases, 'rows': base['rows'],
                 'enum_lookups': base['enum_lookups'], 'coverage': coverage,
                 'instructions': nm.instructions(visited)})
    return head


def main():
    data = json.dumps(produce(), indent=1, sort_keys=True).encode()
    if '--write' in sys.argv:
        if OUT.exists():
            raise SystemExit(f'{OUT} exists; refusing to overwrite')
        OUT.write_bytes(data)
        print('wrote', OUT, hashlib.sha256(data).hexdigest())
    else:
        old = OUT.read_bytes()
        assert old == data, 'replay differs'
        print('replay identical', hashlib.sha256(data).hexdigest())


if __name__ == '__main__':
    main()
