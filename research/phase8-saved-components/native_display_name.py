"""ActorData +10 display name exactly as the original stores it: ThingData
47D960 (duplicate-name log 410C40, +10 = 5B5620(node, "name"), +14 = 1) and
TGC_DataFileNode::GetLocalized 5B5620 (5B4750 field lookup; missing field ->
TGC_DataFileNodeException "Node doesn't contain field."; a value starting
with '#' -> Right(len-1) 41D390 -> 4B9F40([B36544]; key, B43B48)).

The nodes come from the ORIGINAL TGI parser 5BD080 run on each snippet
(research/phase8-restoration/native_tgi_parser.py, used read-only). Seams:
4B9F40 (string table lookup: a dict, every key recorded) and the 410C40 log.
B43B48 is the static empty CString.
"""
from pathlib import Path
import sys, json
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-restoration'))
from native_tgi_parser import Machine as Parser, lines_of, P, V, H
from unicorn.x86_const import UC_X86_REG_ECX

TABLE = {'k1': 'Infantry Company', 'unit_name': 'Panzer Regiment', '': 'EMPTY-KEY-TEXT'}


class Machine(Parser):
    def __init__(self, lines, table):
        super().__init__(lines)
        self.table = table
        self.lookups = []
        self.exception_text = None
        self.logs = []
        self.w(0xb43b48, self.string([]))
        self.w(0xb36544, self.alloc(0x40))

    def hook(self, u, pc, size, user):
        if pc == 0x4b9f40:  # thiscall(&key, &default) ret 8 -> CString*
            key = self.cstr(self.r(self.arg()))
            text = self.table.get(key)
            self.lookups.append({'key': key, 'found': text is not None})
            if text is None:
                self.ret(self.arg(1), pop=8)
            else:
                obj = self.alloc(4)
                self.w(obj, self.string([ord(c) for c in text]))
                self.ret(obj, pop=8)
        elif pc == 0x96bef5:  # _CxxThrowException: keep the exception text (+28)
            p = self.r(self.arg() + 0x28)
            self.exception_text = self.cstr(p) if p else None
            super().hook(u, pc, size, user)
        elif pc == 0x410c40:  # cdecl log(fmt, ...)
            self.logs.append({'format': self.cstr_w(self.arg()), 'ids': self.cstr(self.arg(1))})
            self.ret()
        else:
            super().hook(u, pc, size, user)

    def cstr_w(self, p):
        out = bytearray()
        while bytes(self.u.mem_read(p + len(out), 2)) != b'\0\0':
            out += self.u.mem_read(p + len(out), 2)
        return out.decode('utf-16-le')


def thing_node(m, root):
    for i in range(m.r(root + 0x38)):
        n = m.r(m.r(root + 0x34) + 4 * i)
        if m.cstr(m.r(n)) == 'Thing':
            return n
    raise AssertionError('no Thing node')


def case(name, src, preset=''):
    m = Machine(lines_of(src), TABLE)
    resource = m.alloc(0x100)
    m.w(resource + 0x38, m.string([ord(c) for c in 'oracle.tgi']))
    stream = m.alloc(0x40)
    vt = m.alloc(0x100)
    m.w(stream, vt)
    m.w(vt + 0x48, 0x301f600)
    m.w(m.stack, m.stop)
    m.w(m.stack + 4, stream)
    from unicorn.x86_const import UC_X86_REG_ESP
    m.u.reg_write(UC_X86_REG_ESP, m.stack)
    m.u.reg_write(UC_X86_REG_ECX, resource)
    m.u.emu_start(0x5bd080, m.stop, count=20_000_000)
    if m.thrown is not None:
        return {'name': name, 'source_hex': src.hex(), 'preset': preset,
                'native': {'outcome': {'kind': 'ParseException', 'text': m.exception_text}, 'stored_utf16': None,
                           'flag_14': None, 'lookups': m.lookups, 'logs': m.logs}}
    node = thing_node(m, m.r(resource + 0x3c))
    data = m.alloc(0x40)
    m.w(data + 8, m.string([ord(c) for c in 'thing_ids']))
    m.w(data + 0x10, m.string([ord(c) for c in preset]))
    m.u.mem_write(data + 0x14, b'\0')
    m.messages = []
    try:
        m.invoke(0x47d960, data, [node])
        outcome = None
    except AssertionError:
        assert m.thrown is not None, 'stopped without a native throw'
        outcome = {'kind': 'TGC_DataFileNodeException', 'text': m.exception_text}
    stored = None
    if outcome is None:
        p = m.r(data + 0x10)
        raw = bytes(m.u.mem_read(p, m.r(p - 12) * 2))
        stored = [int.from_bytes(raw[i:i + 2], 'little') for i in range(0, len(raw), 2)]
    return {'name': name, 'source_hex': src.hex(), 'preset': preset,
            'native': {'outcome': outcome, 'stored_utf16': stored,
                       'flag_14': m.u.mem_read(data + 0x14, 1)[0] if outcome is None else None,
                       'lookups': m.lookups, 'logs': m.logs}}


def snippets():
    T = b'[Template T]\n{\n\tstring IDS\n\tlocalized name\n}\n'
    thing = lambda body: T + b'[Thing template=T]\n{\n\tIDS = x\n' + body + b'}\n'
    rows = [
        ('hash_found', thing(b'\tname = "#k1"\n')),
        ('hash_missing', thing(b'\tname = "#k2"\n')),
        ('literal', thing(b'\tname = "plain text"\n')),
        ('unquoted_null', thing(b'\tname = null\n')),
        ('empty', thing(b'\tname = ""\n')),
        ('at_hash', thing(b'\tname = "@#k1"\n')),
        ('hash_only', thing(b'\tname = "#"\n')),
        ('declared_only', thing(b'')),
        ('utf8_bytes', thing(b'\tname = "Caf\xc3\xa9"\n')),
        ('hash_in_middle', thing(b'\tname = "a#k1"\n')),
        ('hash_case', thing(b'\tname = "#K1"\n')),
        ('template_default', b'[Template T]\n{\n\tstring IDS\n\tlocalized name = "#unit_name"\n}\n'
                             b'[Thing template=T]\n{\n\tIDS = x\n}\n'),
        ('no_name_field', b'[Template T]\n{\n\tstring IDS\n}\n[Thing template=T]\n{\n\tIDS = x\n}\n'),
    ]
    return rows


def main():
    out = []
    for name, src in snippets():
        out.append(case(name, src))
    out.append(case('duplicate_preset', snippets()[2][1], preset='Old Name'))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'string_table': TABLE, 'cases': out}
    path = HERE / 'native-display-name.json'
    path.write_text(json.dumps(doc, indent=1), encoding='utf-8')
    for row in out:
        n = row['native']
        print(row['name'], n['outcome'] and n['outcome']['kind'],
              None if n['stored_utf16'] is None else ''.join(map(chr, n['stored_utf16'])), n['lookups'], n['logs'])
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
