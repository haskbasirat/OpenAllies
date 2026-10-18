"""Original GActor::Load tail 4D971B..4D99C2 (after the 4D67F0 refresh): the
+158 required-object check, MapString validation of the saved name key +DC
against the expected name (4D8CA0 / numbered 4D8AB0, wcscmp 96CB83) and the
numbered-name fix (swscanf 96C99A "%d") / assignment (Kingdom 4E4980),
under Unicorn (native_machine.py). Also the Kingdom numbered-name counter
4E4980 itself (with 5C37E0, 7BA3C0, 4CF1A0) on a real +35C hash table.

The tail starts mid-function: the harness rebuilds the frame slots it reads
([esp+20] actor version, [esp+98] saved SEH word, [esp+A4] return, [esp+A8]
load context) and ebx = 0, as at 4D971B. Original code executed: the tail,
4D8CA0, 4D8AB0, 4012C0, 405D60/405B20 (CString::Format), 96CB83, 96C99A,
49AD00, 405B80. Seams (recorded): 4BAA20 map-string lookup and 4B9F40
localize on [B36544], 59E520 context flag, owner virtual +108, 4E4980 in the
tail (stores +170 like 4CF1A0), 5A1040, 858A00, 401B00/856560.
B43B40/B43B48 are the statically constructed empty CStrings (405B10 at 9E7400).
The statically linked CRT runs natively except _getptd 9738A9 (TLS through
imports), seamed to a zeroed per-thread block, the CRT heap (_malloc 96E19F /
_free 96D331: HeapAlloc imports) and the CRT locks _lock 972E9B / _unlock
972DE6 (critical-section imports).
"""
from pathlib import Path
import sys, json, struct
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H, Throw, STOP
from unicorn.x86_const import *

LOCALIZATION = {  # Data/Localization/strings_rtse.tgi
    'actor_name_numbered|ds': '%dth %s', 'actor_name_numbered_st|ds': '%dst %s',
    'actor_name_numbered_nd|ds': '%dnd %s', 'actor_name_numbered_rd|ds': '%drd %s'}


class Names(Machine):
    def __init__(self, c):
        super().__init__(b'')
        self.c = c
        self.w(0xb43b40, self.cstring(''))
        self.w(0xb43b48, self.cstring(''))
        table = self.alloc(0x60010)
        self.w(0xb364c0, table)
        for gid, live in c.get('gtids', {}).items():
            gid = int(gid)
            slot, gen = gid & 0xffff, gid >> 16
            self.u.mem_write(table + 4 + slot * 2, struct.pack('<H', gen if live != 'stale' else gen ^ 1))
            self.w(table + 0x20004 + slot * 4, 0 if live == 'null' else self.alloc(8))
        self.w(0xb36544, self.alloc(0x80))
        d = self.alloc(0x400)
        self.w(d + 8, self.cstring(c['def_name']))
        self.w(d + 0x10, self.cstring(c['display']))
        self.wb(d + 0x1b0, c['numbered'])
        self.kingdom = self.alloc(0x400) if c['kingdom'] else 0

        def kingdom(m):
            m.record('owner+108')
            m.ret(0, m.kingdom)
        a = self.alloc(0x200)
        self.actor = a
        self.w(a, self.vtable({0x108: kingdom}))
        self.w(a + 4, d)
        self.w(a + 0x14, c['gtid'])
        self.w(a + 0x158, c['required_158'])
        self.w(a + 0x170, c['number'])
        self.w(a + 0xe8, self.kingdom)
        self.w(a + 0xdc, self.cstring(c['key']))
        self.w(a + 0xe0, self.cstring(c['text']))
        self.ctx = self.alloc(0x100)

        def lookup(m):  # 4BAA20(&out, &key, create, default) thiscall ret 0x10
            key = m.cstring_text(m.r(m.arg(1)))
            create = m.arg(2)
            found = key in m.c['map_strings']
            m.record('4BAA20', key=key, create=create, found=found)
            entry = 0
            if found:
                entry = m.alloc(0x20)
                m.w(entry, m.alloc(8))
                m.w(entry + 4, 1000)
            m.w(m.arg(0), entry)
            m.ret(0x10, m.arg(0))
        self.seam(0x4baa20, lookup)

        def localize(m):  # 4B9F40(&key, &default) thiscall ret 8 -> CString*
            key = m.cstring_text(m.r(m.arg(0)))
            text = m.c['localization'].get(key)
            m.record('4B9F40', key=key, found=text is not None)
            if text is None:
                m.ret(8, m.arg(1))
            else:
                obj = m.alloc(4)
                m.w(obj, m.cstring(text))
                m.ret(8, obj)
        self.seam(0x4b9f40, localize)

        def context(m):  # 59E520 thiscall on the load context
            m.record('59E520', result=m.c['strings_from_stream'])
            m.ret(0, int(m.c['strings_from_stream']))
        self.seam(0x59e520, context)

        def assign(m):  # 4E4980(actor) thiscall on Kingdom, ret 4; 4CF1A0 stores +170
            this = m.u.reg_read(UC_X86_REG_ECX)
            m.record('4E4980', this='kingdom' if this == m.kingdom and this else hex(this), result=m.c['assign'])
            m.w(m.arg(0) + 0x170, m.c['assign'])
            m.ret(4)
        self.seam(0x4e4980, assign)
        # CRT environment seams (imports): _getptd, heap, locks.
        self.ptd = self.alloc(0x200)
        self.seam(0x9738a9, lambda m: m.ret(0, m.ptd))
        self.seam(0x96e19f, lambda m: m.ret(0, m.alloc(m.arg(0), 0xa5)))
        self.seam(0x96d331, lambda m: m.ret(0))
        self.seam(0x972e9b, lambda m: m.ret(0))
        self.seam(0x972de6, lambda m: m.ret(0))

    def run(self):
        F = 0x2f00000
        self.u.mem_write(F, b'\0' * 0x100)
        self.w(F + 0x20, self.c['version'])
        self.w(F + 0x98, 0)
        self.w(F + 0xa4, STOP)
        self.w(F + 0xa8, self.ctx)
        self.u.reg_write(UC_X86_REG_ESP, F)
        self.u.reg_write(UC_X86_REG_ESI, self.actor)
        self.u.reg_write(UC_X86_REG_EBX, 0)
        self.u.reg_write(UC_X86_REG_EDI, 0)
        self.u.reg_write(UC_X86_REG_EBP, 0)
        try:
            self.u.emu_start(0x4d971b, STOP, count=2000000)
        except Throw:
            pass
        if self.outcome is None:
            assert self.u.reg_read(UC_X86_REG_EIP) == STOP
            assert self.u.reg_read(UC_X86_REG_ESP) == F + 0xac
        a = self.actor
        return {'outcome': self.outcome, 'calls': self.calls,
                'key': self.cstring_text(self.r(a + 0xdc)), 'text': self.cstring_text(self.r(a + 0xe0)),
                'number': self.r(a + 0x170)}


class Counter(Machine):
    """4E4980 on a Kingdom whose +35C table is {buckets, bucket count,
    element count, rehash byte}; all native."""
    def __init__(self, buckets):
        super().__init__(b'')
        self.kingdom = self.alloc(0x400)
        t = self.kingdom + 0x35c
        self.w(t, self.alloc(4 * buckets))
        for i in range(buckets):
            self.w(self.r(t) + 4 * i, 0)
        self.w(t + 4, buckets)
        self.w(t + 8, 0)
        self.wb(t + 0xc, 0)
        self.defs = {}

    def assign(self, definition):
        if definition not in self.defs:
            self.defs[definition] = self.alloc(0x40)
        a = self.alloc(0x200)
        self.w(a + 4, self.defs[definition])
        self.call(0x4e4980, self.kingdom, [a])
        return self.r(a + 0x170)


def base(**kw):
    c = {'version': 14, 'gtid': 0x10077, 'required_158': 0, 'key': '', 'text': '', 'number': 0,
         'def_name': 'usa_infantry_co', 'display': 'Infantry Company', 'numbered': 0, 'kingdom': True,
         'assign': 3, 'strings_from_stream': False, 'map_strings': [], 'localization': dict(LOCALIZATION),
         'gtids': {}}
    c.update(kw)
    return c


def cases():
    rows = []
    add = lambda name, **kw: rows.append({'name': name, **base(**kw)})
    # +158 required object
    add('required_live', required_158=0x20005, gtids={str(0x20005): 'live'})
    add('required_stale', required_158=0x20005, gtids={str(0x20005): 'stale'})
    add('required_null', required_158=0x20005, gtids={str(0x20005): 'null'})
    add('required_unknown', required_158=0x30009)
    # MapString key: equal to the expected name -> deleted
    add('key_equals_display', key='ms_1', text='Infantry Company')
    add('key_equals_display_case', key='ms_1', text='infantry company')
    for n in (1, 2, 3, 4, 11, 12, 13, 21, 22, 23, 101, 111, 112, 0, 100, 1000001, -1, -12, -10, -100, -1000, 0x7fffffff, -0x80000000):
        add('numbered_name_%d' % n, key='ms_1', number=n & 0xffffffff, text='%d? Infantry Company' % n)
    for n, text in ((1, '1st Infantry Company'), (2, '2nd Infantry Company'), (3, '3rd Infantry Company'),
                    (4, '4th Infantry Company'), (11, '11th Infantry Company'), (21, '21st Infantry Company'),
                    (112, '112th Infantry Company'), (0, '0th Infantry Company'), (-10, '-10th Infantry Company'),
                    (-100, '-100th Infantry Company'), (1000001, '1000001st Infantry Company')):
        add('numbered_equal_%d' % n, key='ms_1', number=n, text=text)
    add('numbered_missing_localization', key='ms_1', number=2, text='', localization={})
    # not equal: map string present / missing / context flag
    add('key_kept_present', key='ms_2', text='Custom', map_strings=['ms_2'])
    add('key_kept_missing', key='ms_3', text='Custom')
    add('key_kept_stream_strings', key='ms_3', text='Custom', strings_from_stream=True)
    # numbered names (ActorData+1B0)
    for version in (13, 14):
        for text in ('12th Infantry', '  7', '-3x', 'abc', '0', '+5', '2147483648', '4294967301', '', '\t\n 9z', '- 4', '007'):
            add('fix_v%d_%r' % (version, text), version=version, numbered=1, key='ms_4', text=text)
    # _swscanf "%d" classification of non-ASCII whitespace/digits (CRT tables)
    # (a char >= 0x100 reaching the whitespace test calls GetStringTypeW: OS,
    #  not AA.exe code; the port refuses such inputs, so none are run here)
    U = chr
    for i, text in enumerate((U(0xa0) + '7', '7' + U(0x3000), '7' + U(0xff17), '7' + U(0x667), '7' + U(0xa0),
                              U(0xb) + '5', U(0x1c) + '5', U(0x85) + '5', U(0xb2), U(0xb2) + '7', '99999999999',
                              U(0xa0) + '-' + U(0xa0) + '1', '-' + U(0xa0) + '1', U(0xffff) + '7', '-')):
        add('scan_unicode_%d' % i, version=13, numbered=1, key='ms_4', text=text)
    add('fix_with_number_set', version=13, numbered=1, key='ms_4', text='5', number=9)
    add('assign_no_key', numbered=1)
    add('assign_key_kept', numbered=1, key='ms_5', text='Custom', map_strings=['ms_5'])
    add('assign_no_kingdom', numbered=1, kingdom=False)
    add('numbered_has_number', numbered=1, number=4)
    add('numbered_v13_empty_key', version=13, numbered=1, text='15')
    add('everything', version=13, numbered=1, required_158=0x20005, gtids={str(0x20005): 'live'}, key='ms_6',
        text='31st Infantry Company', number=0, map_strings=[])
    return rows


def main():
    out = []
    for c in cases():
        m = Names(c)
        out.append({'input': c, 'native': m.run()})
    counters = []
    for buckets in (1, 7):
        m = Counter(buckets)
        seq = ['a', 'b', 'a', 'c', 'a', 'b', 'd', 'e', 'f', 'a', 'g', 'h', 'b']
        counters.append({'buckets': buckets, 'definitions': seq, 'numbers': [m.assign(d) for d in seq],
                         'element_count': m.r(m.kingdom + 0x35c + 8)})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__,
           'visited_instructions': len(VISITED), 'cases': out, 'counters': counters}
    path = HERE / 'native-actor-names.json'
    path.write_text(json.dumps(doc, indent=1), encoding='utf-8')
    kinds = {}
    for row in out:
        k = (row['native']['outcome'] or {}).get('kind', 'ok')
        kinds[k] = kinds.get(k, 0) + 1
    print(json.dumps({'cases': len(out), 'outcomes': kinds, 'counters': [c['numbers'] for c in counters],
                      'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
