"""Original NameList loader 4C19D0 over whole-parser 5BD080 nodes.

The shared definition machine supplies line input, heap/pool/mutex and throw
seams. 4B9F40 is an explicit dictionary localization seam: the ORIGINAL
5B5620 removes '#', calls it, and appends its returned CString. Missing keys
return the original supplied default. No Name/child/getter/vector seam is used.
Run with --write to create native-name-lists.json; default only compares it.
"""
from pathlib import Path
import json, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-definitions'))
from native_machine import Machine as Base, native_windows, P, V, H
from unicorn.x86_const import *


class Machine(Base):
    def __init__(self, lookup):
        self.lookup = lookup
        self.lookups = []
        super().__init__()
        self.w(0xb43b48, self.cstring(''))

    def hook(self, u, pc, size, user):
        if pc == 0x4b9f40:
            key = self.wstr(self.r(self.arg(0)))
            self.lookups.append(key)
            out = self.alloc(4)
            self.w(out, self.cstring(self.lookup[key]) if key in self.lookup else self.r(self.arg(1)))
            self.ret(out, pop=8)
            return
        super().hook(u, pc, size, user)


def snippet(rows, header='NameList', template=''):
    return (template + '[' + header + ']\n{\nIDS = test\nname = "List"\n' + rows + '\n}\n').encode('latin1')


def cases():
    out = [
        ('empty', snippet(''), {}),
        ('single_untyped', snippet('[Name]\nname = "First"'), {}),
        ('duplicates_untyped', snippet('[Name]\nname = "Alpha"\n[Name]\nname = "Bravo"\n[Name]\nname = "Alpha"'), {}),
        ('mixed_other_child', snippet('[Name]\nname = "First"\n[Other]\nname = "Skip"\n[Name]\nname = "Last"'), {}),
        ('case_insensitive', snippet('[nAmE]\nNaMe = "First"\n[NAME]\nNAME = "Last"'), {}),
        ('localization', snippet('[Name]\nname = "#one"\n[Name]\nname = "#missing"\n[Name]\nname = "@#two"'), {'one': 'Translated one', 'two': 'Translated two'}),
        ('quotes_and_null', snippet('[Name]\nname = "A""B"\n[Name]\nname = "@@literal"\n[Name]\nname = null\n[Name]\nname = ""'), {}),
        ('missing_name', snippet('[Name]\nother = "missing"'), {}),
        ('untyped_plain_string', snippet('[Name]\nname = plain'), {}),
        ('declared_string', snippet('[Name]\nstring name = plain'), {}),
        ('declared_localized', snippet('[Name]\nlocalized name = "Literal"'), {}),
        ('declared_null_and_quoted_null', snippet('[Name]\nlocalized name = null\n[Name]\nname = "null"\n[Name]\nname = ""'), {}),
        ('byte_widening', snippet('[Name]\nname = "City \x80\xe9\xff"'), {}),
    ]
    template = '[Template NL]\n{\nstring IDS\nlocalized name\n[list Name]\n{\nlocalized name\n}\n}\n'
    out.append(('template_list', snippet('[Name]\nname = "First"\n[Name]\nname = "Second"', 'NameList template=NL', template), {}))
    root = ROOT.parent / 'reconstruction/assets/original/Data'
    path = root / 'Namelists/namelist_ukraine.tgi'
    # Dictionary values are explicit test inputs, not a substitute for a port of
    # the localization manager. The real source file itself is hash pinned.
    lookup = {'namelist_ukraine_name': 'Ukraine list'}
    for i in range(1, 31):
        key = 'namelist_ukraine_Name_name' + ('' if i == 1 else '_' + str(i))
        lookup[key] = 'fixture city ' + str(i)
    out.append(('real_ukraine', path.read_bytes(), lookup))
    return out


def run(label, source, lookup):
    m = Machine(lookup)
    parsed = m.reparse(source, label + '.tgi')
    row = {'label': label, 'source_hex': source.hex(), 'source_sha256': H(source), 'lookup': lookup}
    if parsed['error'] is not None:
        row['result'] = {'parse_error': parsed['error'], 'names': None}
        return row, m.visited
    root = parsed['root']
    nodes = [m.r(m.r(root + 0x34) + 4 * i) for i in range(m.r(root + 0x38))]
    node = next(n for n in nodes if not m.r(n + 0x20) & 1)
    d = m.alloc(0x24)
    ctor = m.thiscall(0x4c1630, d, [m.cstring('test')])
    assert 'thrown' not in ctor, ctor
    try:
        load = m.thiscall(0x4c19d0, d, [node])
    except Exception:
        print('Native loader stopped at', hex(m.u.reg_read(UC_X86_REG_EIP)), m.lookups, flush=True)
        raise
    names = [m.wstr(m.r(m.r(d + 0x18) + 4 * i)) for i in range(m.r(d + 0x1c))]
    row['result'] = {'error': load.get('thrown'), 'names': names, 'lookups': m.lookups,
                     'count': m.r(d + 0x1c), 'parsed_children': [m.node(n)['name'] for n in
                         [m.r(m.r(node + 0x34) + 4 * i) for i in range(m.r(node + 0x38))]]}
    return row, m.visited


def main():
    rows, visited = [], {}
    for label, source, lookup in cases():
        row, used = run(label, source, lookup)
        rows.append(row)
        visited.update(used)
        print(label, row['result'], flush=True)
    doc = {'schema': 1, 'original_sha256': H(P.b), 'v14_sha256': H(V.b),
           'producer_sha256': H(Path(__file__).read_bytes()),
           'dependencies': {str(p.relative_to(ROOT)).replace('\\', '/'): H(p.read_bytes()) for p in [
               ROOT / 'research/phase8-definitions/native_machine.py',
               ROOT / 'research/phase8-restoration/native_tgi_parser.py',
               ROOT / 'research/phase7-definitions/probe_lists.py',
               ROOT / 'research/phase5-actors/original_creation.py']},
           'scope': __doc__, 'cases': rows, 'native_windows': native_windows(visited)}
    path = HERE / 'native-name-lists.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1) + '\n')
    else:
        assert json.loads(path.read_text()) == doc, 'native name-list fixture differs'
    print(len(rows), 'native cases; fixture SHA256', H(path.read_bytes()))


if __name__ == '__main__':
    main()
