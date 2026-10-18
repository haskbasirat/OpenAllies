"""Static map of GameData (ctor 48A8E0) manager fields -> constructor ->
installed vtable -> vf0/vf8/vf18/vf1C (read-only analysis of AA.exe)."""
import re, json, sys
import xref
P = xref.P
ls = xref.dis(0x48a8e0, 1400)
ls = [l for l in ls if int(l.split()[0], 16) < 0x48b556]
out = {}
for i, l in enumerate(ls):
    m = re.search(r'call 0x([0-9a-f]+)$', l)
    if not m:
        continue
    target = int(m.group(1), 16)
    prev = ls[max(0, i - 6):i]
    lea = [re.search(r'lea ecx, \[esi \+ (0x[0-9a-f]+)\]', p) for p in prev]
    lea = [x for x in lea if x]
    nxt = ls[i + 1:i + 5]
    store = [re.search(r'mov dword ptr \[esi \+ (0x[0-9a-f]+)\], eax', n) for n in nxt]
    store = [x for x in store if x]
    field = None
    if store and any('mov ecx, eax' in p for p in prev[-3:]):
        field = int(store[0].group(1), 16)
    elif lea:
        field = int(lea[-1].group(1), 16)
    if field is None:
        continue
    # vtable installed by the ctor: first 'mov dword ptr [esi|ecx|eax], 0xa.....'
    vt = None
    for c in xref.dis(target, 120):
        mm = re.search(r'mov dword ptr \[(esi|edi|ecx|eax|ebx)\], 0x(a[0-9a-f]{5})$', c)
        if mm:
            vt = int(mm.group(2), 16)
        if c.split()[2:3] == ['ret'] or ' ret' in c:
            break
    if vt is None:
        continue
    slots = {k: hex(xref.r32(vt + k)) for k in (0, 8, 0x18, 0x1c, 0x24)}
    out.setdefault(hex(field), []).append({'ctor': hex(target), 'vtable': hex(vt), **{'vf%x' % k: v for k, v in slots.items()}})
json.dump(out, sys.stdout, indent=1)
