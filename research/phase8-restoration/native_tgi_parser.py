"""Original TGC_DataFileParser (5BD080 and its 18-handler table 5BCC80 /
lexer table 5BEFD0) executed under Unicorn on whole source texts.

Only the line source is a seam: 857210 returns the next physical line of the
supplied bytes (each byte widened to UTF-16, terminating LF kept) and the
stream's virtual +48 reports EOF. Native code trims (4749C0), classifies,
tokenizes and builds the node tree. The produced tree is read back from the
native layout: node name CString +0, children vector +34/+38, field hash map
+40/+44 (entries {field, next}, field name at field+4).
"""
from pathlib import Path
import sys, json, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase7-definitions'))
sys.path.insert(0, str(HERE))
from probe_lists import Machine as Base
from original_creation import P, V, H
from unicorn.x86_const import *

EOF_STUB = 0x301f600
THROW = 0x96bef5  # __CxxThrowException@8


class Parse(Exception):
    pass


class Machine(Base):
    def __init__(self, lines):
        super().__init__()
        self.lines = list(lines)
        self.at = 0
        self.thrown = None
        self.messages = []
        # parser handler/lexer tables are filled by their static initializers.
        self.invoke(0x5bcc80, 0xb3db80, [])
        self.invoke(0x5befd0, 0xb3dbdc, [])
        # Explicit global seams instead of null-page reads: the text-key debug
        # flag behind B39940+340 is off, and the markup registry B36530+3BC
        # (494000) is empty.
        # The base harness's [B39BC8] object is 16 bytes; the template registry
        # hash map lives at +24/+28 (5BF0D0), so give it a full-size object.
        old_manager = self.r(0xb39bc8)
        manager = self.alloc(0x100)
        self.w(manager + 4, self.r(old_manager + 4))
        self.w(0xb39bc8, manager)
        settings = self.alloc(0x400)
        flag = self.alloc(16)
        self.w(settings + 0x340, flag)
        self.w(0xb39940, settings)
        manager = self.r(0xb36530) or self.alloc(0x600)
        registry = self.alloc(0x40)
        self.w(manager + 0x3bc, registry)
        self.w(0xb36530, manager)
        import null_guard
        null_guard.install(self)

    def hook(self, u, pc, size, user):
        if pc == 0x857210:
            out = self.arg()
            if self.at < len(self.lines):
                line = self.lines[self.at]
                self.at += 1
                ok = 1
            else:
                line = ''
                ok = 0
            p = self.string([b for b in line])
            # assign to the caller's CString (old buffer leaks harmlessly)
            self.w(out, p)
            self.ret(ok, pop=8)
        elif pc in (0x96e19f, 0x96e173, 0x96e0f8):  # _malloc/__nh_malloc/__heap_alloc (cdecl)
            self.ret(self.alloc(max(1, self.arg())))
        elif pc == 0x97259a:  # _calloc
            n = max(1, self.arg() * self.arg(1))
            q = self.alloc(n)
            self.u.mem_write(q, bytes(n))
            self.ret(q)
        elif pc == 0x97117c:  # _realloc
            old, n = self.arg(), self.arg(1)
            q = self.alloc(max(1, n))
            if old:
                self.u.mem_write(q, bytes(self.u.mem_read(old, n)))
            self.ret(q)
        elif pc == 0x96d331:  # _free
            self.ret()
        elif pc == 0x5bf0d0 and self.r(u.reg_read(UC_X86_REG_ECX) + 0x28) == 0:
            # Seam: the global template registry (hash map at +24/+28) starts
            # empty but constructed; give it buckets, then run native code.
            reg = u.reg_read(UC_X86_REG_ECX)
            buckets = self.alloc(4 * 97)
            self.u.mem_write(buckets, bytes(4 * 97))
            self.w(reg + 0x24, buckets)
            self.w(reg + 0x28, 97)
        elif pc in (0x857e10, 0x857ee0):
            # recursive mutex acquire/release around shared pools (single thread here)
            self.ret()
        elif pc == EOF_STUB:
            self.ret(int(self.at >= len(self.lines)))
        elif pc == 0x401b00:
            # CString from wchar*: keep native behaviour via base handler, but
            # remember messages built for exceptions.
            try:
                self.messages.append(self.text(self.arg()))
            except Exception:
                pass
            super().hook(u, pc, size, user)
        elif pc == THROW:
            self.thrown = self.messages[-6:]
            u.emu_stop()
        else:
            super().hook(u, pc, size, user)

    def parse(self):
        resource = self.alloc(0x100)
        self.w(resource + 0x38, self.string([ord(c) for c in 'oracle.tgi']))
        stream = self.alloc(0x40)
        vt = self.alloc(0x100)
        self.w(stream, vt)
        self.w(vt + 0x48, EOF_STUB)
        self.error = None
        self.w(self.stack, self.stop)
        self.w(self.stack + 4, stream)
        self.u.reg_write(UC_X86_REG_ESP, self.stack)
        self.u.reg_write(UC_X86_REG_ECX, resource)
        self.u.emu_start(0x5bd080, self.stop, count=20_000_000)
        assert not self.null_accesses, self.null_accesses
        if self.thrown is not None:
            return {'error': self.thrown, 'line': self.at}
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))
        return {'error': None, 'root': self.node(self.r(resource + 0x3c))}

    def cstr(self, p):
        if p == 0:
            return None
        n = self.r(p - 12)
        return bytes(self.u.mem_read(p, n * 2)).decode('utf-16-le', 'replace')

    def node(self, p, depth=0):
        assert depth < 64
        children = [self.node(self.r(self.r(p + 0x34) + 4 * i), depth + 1) for i in range(self.r(p + 0x38))]
        fields = []
        buckets, count = self.r(p + 0x40), self.r(p + 0x44)
        if buckets:
            for b in range(count):
                e = self.r(buckets + 4 * b)
                while e:
                    f = self.r(e)
                    vt = self.r(f)
                    value = self.r(f + 0x10)
                    fields.append({'name': self.text(self.r(f + 4)), 'line': self.r(f + 0xc), 'vtable': hex(vt),
                                   'value': self.cstr(value) if vt == 0xa48770 else value})
                    e = self.r(e + 4)
        return {'name': self.cstr(self.r(p)), 'label': self.cstr(self.r(p + 4)), 'flags': hex(self.r(p + 0x20)),
                'fields': fields, 'children': children}


def lines_of(data: bytes):
    out = []
    start = 0
    while start < len(data):
        end = data.find(b'\n', start)
        if end == -1:
            out.append(data[start:])
            break
        out.append(data[start:end + 1])
        start = end + 1
    return out


def snippet_cases():
    D = Path('E:/Games/Axis and Alies/reconstruction/assets/original/Data')
    carrier = (D / 'Units/USA/usa_carrier.tgi').read_bytes()
    a = carrier.index(b'\t[RecruitComponent]')
    b = carrier.index(b'\t[View]')
    trig = (D / 'Game/trigger_elements.tgi').read_bytes().split(b'\n')
    line355 = trig[354] + b'\n'
    rows = {
        'carrier_size_delayed_body': b'[A]\r\n{\r\n' + carrier[a:b] + b'}\r\n',
        'delayed_then_parent_property': b'[A]\n{\n[Size]\na=1\n{\nb=2\n}\nc=3\n[View]\nv=1\n}\n',
        'delayed_on_latest_sibling': b'[A]\n{\n[Size]\na=1\n[Inner]\nx=1\n{\ny=2\n}\nz=3\n}\n',
        'delayed_top_level': b'[A]\na=1\n{\nb=2\n}\nc=3\n[B]\nq=1\n',
        'blank_lines_then_body': b'[A]\n{\n[S]\n\n{\nb=2\n}\n}\n',
        'delayed_with_nested_child': b'[A]\n{\n[S]\na=1\n{\nb=2\n[T]\nt=1\n}\nc=3\n}\n',
        'trigger_line_355': b'[E]\r\n{\r\n' + line355 + b'\tnext = 1\r\n}\r\n',
    }
    for i, v in enumerate([b'"x"y', b'"x" "y"', b'"x""y"', b'"x" z', b'"unterminated', b'"x"" ; comment', b'x"y"',
                           b'"x""', b'""""', b'"a\\"b"', b'"x"   ', b'"#k""', b'w1 w2', b'"x";c', b'"@abc"', b'"#key"',
                           b'"plain text"', b'""', b'"null"', b'"@"', b'" spaced "', b'null', b'"a;b"', b'"x" ; "y"',
                           b'"t\tx"', b'"@@x"', b'"a@b"', b'x"y;z"']):
        rows[f'value_{i}'] = b'[E]\r\n{\r\n\ta = ' + v + b'\r\n\tz = 1\r\n}\r\n'
    for i, v in enumerate([b'localized a = null', b'localized a = NULL', b'localized a = "#k"', b'localized a = "@#k"',
                           b'string a = "x"', b'string a = x', b'string a = null', b'localized a = word',
                           b'localized a = "x" y']):
        rows[f'declaration_{i}'] = b'[Template E]\r\n{\r\n\t' + v + b'\r\n}\r\n'
    return rows


def tree(node):
    return {'name': node['name'], 'template': int(node['flags'], 16) & 1 == 1,
            'fields': sorted([{'name': f['name'], 'localized': f['vtable'] == '0xa48770',
                               'value': f['value'] if f['vtable'] == '0xa48770' else None} for f in node['fields']],
                             key=lambda f: f['name'].lower()),
            'children': [tree(c) for c in node['children']]}


def build_fixture():
    rows = []
    for name, src in snippet_cases().items():
        m = Machine(lines_of(src))
        r = m.parse()
        rows.append({'name': name, 'source_hex': src.hex(),
                     'native': {'error': r['error'][-1] if r['error'] else None,
                                'tree': None if r['error'] else [tree(c) for c in r['root']['children']]}})
        print(name, rows[-1]['native']['error'])
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b),
           'scope': 'Whole original 5BD080 line loop, 5BCC80 handler table, 5BEFD0 lexer table, 5BDF80 value scan, 5BEE60/5BDC00 quote rule. Seams: line source 857210/EOF, pool mutex no-ops, CRT heap, empty template registry buckets. Tree is the parsed node structure: names, template flag, field names, localized (vtable A48770) field text.',
           'cases': rows}
    out = HERE / 'native-tgi-parser.json'
    out.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(rows), 'sha256': H(out.read_bytes())}))


if __name__ == "__main__" and len(sys.argv) > 1 and sys.argv[1] == "--fixture":
    build_fixture()
elif __name__ == "__main__":
    text = sys.argv[1].encode('latin-1').decode('unicode_escape').encode('latin-1')
    m = Machine(lines_of(text))
    print(json.dumps(m.parse(), indent=1))
