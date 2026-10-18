"""Static join: 48DB60 root tag -> GameData field -> manager ctor/vtable
(gamedata_managers.py) -> vf0 (node handler), vf8 (translation allowed),
vf18 (Find), vf1C (type Load). Read-only analysis of the original AA.exe."""
import re, json, subprocess, sys
import xref
P = xref.P
def ws(a):
    try:
        return bytes(xref.MEM[a-P.base:a-P.base+200]).decode('utf-16-le','replace').split('\0')[0]
    except Exception:
        return '?'
lines = xref.dis(0x48dc00, 12000)
tags = []
for i, l in enumerate(lines):
    m = re.search(r'push 0x(a[0-9a-f]{5})$', l)
    if m and i + 3 < len(lines) and 'call 0x401b00' in ''.join(lines[i+1:i+4]):
        tag = ws(int(m.group(1), 16))
        src = None
        for k in range(i - 1, max(i - 6, 0), -1):
            mm = re.search(r'(mov|lea) eax, (dword ptr )?\[ebp \+ (0x[0-9a-f]+)\]', lines[k])
            if mm:
                src = (mm.group(1), int(mm.group(3), 16)); break
        tags.append((tag, src))
managers = json.loads(subprocess.check_output([sys.executable, 'gamedata_managers.py']))
out = []
for tag, src in tags:
    row = {'tag': tag, 'field': None, 'kind': None, 'manager': None}
    if src:
        row['kind'], row['field'] = src[0], hex(src[1])
        row['manager'] = managers.get(hex(src[1]))
    out.append(row)
json.dump(out, sys.stdout, indent=1)
