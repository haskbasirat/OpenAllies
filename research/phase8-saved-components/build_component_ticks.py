"""Builds component-ticks.json: the per-tick (world queue) surface of every
actor component class built by the factory 4CF480 and of the GObject-family
classes (GThing, GObject, GActor, Kingdom, Team, Group, Area, Camera,
DynamicDoodad), for the tick integration.

Native facts (from the code, see README round 6):
* World queues (GWorld ctor 4F297E..4F29C9): 0 +1AC periodic 0.0001 s,
  1 +1C0 periodic 1 s, 2 +1D4 periodic 5 s (no registration helper found),
  3 +1E8 deferred (callback 4F05C0 -> Updatable virtual +18(data)),
  4 +208 deferred (callback 4F05B0 -> Updatable virtual +1C(data)).
  Periodic dispatch 50B330 calls Updatable virtual +14(elapsed, data).
* Registration helpers (each tests an Updatable gate virtual first):
  50AC00 queue 0 (gate +8), 50AC60 queue 1 (gate +8), 50ACC0 queue 3
  add_after (gate +C), 50AD20 queue 3 add_at (gate +C), 50AD80 queue 4
  add_after (gate +10).
* Updatable vtables: every vtable whose +4 is 50AD80 (43 in AA.exe).

Ownership: every address is looked up in the Rust sources (crates/**/*.rs,
token match of the 6-digit address) and in research/phase8-*/inventory.json.
"""
from pathlib import Path
import sys, json, struct, re, bisect
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from disasm import P
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

NATIVE = ROOT.parent / 'reconstruction/research/native-aa'


def u(a):
    return struct.unpack('<I', P.range(a, 4))[0]


# ---------------------------------------------------------------- functions
starts, sizes = [], {}
for line in (NATIVE / 'functions.tsv').read_text(encoding='utf-8').splitlines()[1:]:
    f = line.split('\t')
    a = int(f[0], 16)
    starts.append(a)
    sizes[a] = int(f[3]) if f[3].isdigit() else 0
starts.sort()


def function_of(a):
    i = bisect.bisect_right(starts, a) - 1
    if i < 0:
        return None
    s = starts[i]
    return s if a < s + max(sizes[s], 1) else None


callers = {}
for line in (NATIVE / 'calls.tsv').read_text(encoding='utf-8').splitlines()[1:]:
    f = line.split('\t')
    if len(f) < 2 or not re.fullmatch(r'[0-9a-f]{8}', f[1]):
        continue
    callers.setdefault(int(f[1], 16), set()).add(int(f[0], 16))

# ---------------------------------------------------------------- code scan
md = Cs(CS_ARCH_X86, CS_MODE_32)
md.skipdata = True
vs, r, size, off = P.sections[0]
INS = list(md.disasm(P.b[off:off + size], P.base + r))

vs1, r1, size1, off1 = P.sections[1]
RD = P.b[off1:off1 + size1]
RD_BASE = P.base + r1
UPDATABLE_VTS = [RD_BASE + i for i in range(0, len(RD) - 8, 4)
                 if struct.unpack_from('<I', RD, i + 4)[0] == 0x50ad80]

HELPERS = {0x50ac00: (0, 'periodic', '+8'), 0x50ac60: (1, 'periodic', '+8'),
           0x50acc0: (3, 'add_after', '+C'), 0x50ad20: (3, 'add_at', '+C'),
           0x50ad80: (4, 'add_after', '+10')}

# component classes: slot, name, constructor, component vtable (verified per
# constructor in round 4; Aircraft's is A3AF78)
COMPONENTS = [
    (0, 'Body', 0x561960, 0xa3b0e8), (1, 'Projectile', 0x580ed0, 0xa3e280),
    (2, 'Character', 0x563670, 0xa3b4d8), (3, 'Mover', 0x576920, 0xa3ce88),
    (4, 'Mind (Organization)', 0x526260, 0xa335a0), (4, 'Mind (Aircraft)', 0x51bff0, 0xa32710),
    (4, 'Mind (Element)', 0x535ba0, 0xa35810), (5, 'Performer', 0x57f4d0, 0xa3df40),
    (6, 'Properties', 0x5812f0, 0xa3e3dc), (7, 'Organization', 0x579bd0, 0xa3d434),
    (8, 'Element', 0x570930, 0xa3c6f0), (9, 'Economy', 0x56f370, 0xa3c690),
    (10, 'Denizen', 0x56b720, 0xa3c14c), (11, 'Supply', 0x58aa50, 0xa400c0),
    (12, 'Wall', 0x58cdb0, 0xa40198), (13, 'Structure', 0x588fb0, 0xa3fac8),
    (14, 'Settlement', 0x5867a0, 0xa3eea0), (15, 'Construction', 0x565fa0, 0xa3b950),
    (16, 'Recruit', 0x581e40, 0xa3e6d0), (17, 'Experience', 0x571690, 0xa3c974),
    (18, 'Builder', 0x562f10, 0xa3b2a8), (19, 'MobileDeployment', 0x575940, 0xa3cd00),
    (20, 'Defender', 0x567cd0, 0xa3bbd4), (21, 'Research', 0x584210, 0xa3eb58),
    (22, 'Detection', 0x56eab0, 0xa3c568), (23, 'Aircraft', 0x55e8e0, 0xa3af78),
    (24, 'Transport', 0x58b9a0, 0xa40100), (25, 'Lair', 0x574fa0, 0xa3cc6c),
    (26, 'Flare', 0x572180, 0xa3c9e4), (27, 'MetaGame', 0x93fec0, 0xac7ed0),
    (28, 'Portal', 0x580120, 0xa3e144), (29, 'City', 0x564160, 0xa3b6f8),
    (30, 'Hq', 0x574490, 0xa3ca4c)]
COMPONENT_SLOTS = {0x0: 'dtor', 0x4: 'InitCreate', 0x8: 'InitExisting', 0xc: 'PostCreate', 0x14: 'vf14',
                   0x20: 'requirement', 0x28: 'Save', 0x2c: 'Load', 0x30: 'PostLoad', 0x34: 'vf34'}
# GObject family: name, saved class, constructor, vtable
OBJECTS = [('GThing', 1, 0x4ef020, 0xa2a680), ('GObject', 2, 0x4ea690, 0xa2a2a0), ('GActor', 3, 0x4d6d70, 0xa273f0),
           ('Kingdom', 4, 0x4e5370, 0xa299d8), ('Team', 5, 0x4e67c0, 0xa29780), ('Area', 6, 0x4db6f0, 0xa28a00),
           ('Camera', 7, 0x4dbf40, 0xa28b88), ('Group', 8, 0x4de490, 0xa29490), ('DynamicDoodad', 9, 0x4dc6f0, 0xa28e90)]
NOOPS = {0x4d1e10: 'ret 8', 0x943800: 'ret 4', 0x6a71d0: 'true', 0x6a71f0: 'false', 0x438ac0: 'true(4)',
         0x4b2830: 'ret', 0x96cac2: 'purecall', 0x4db080: '50B250 clear', 0x4c9fa0: 'ret'}

# ---------------------------------------------------------------- vtable stores
stores = {}   # vtable -> [(site, function, operand)]
for ins in INS:
    if ins.mnemonic == 'mov' and ins.op_str.startswith('dword ptr ['):
        m = re.search(r'\], 0x([0-9a-f]+)$', ins.op_str)
        if m:
            v = int(m.group(1), 16)
            if v in UPDATABLE_VTS:
                stores.setdefault(v, []).append((ins.address, function_of(ins.address), ins.op_str))


ADDRS = [i.address for i in INS]


def instructions(start, end):
    i = bisect.bisect_left(ADDRS, start)
    while i < len(INS) and INS[i].address < end:
        yield INS[i]
        i += 1


def updatable_of(ctor):
    """(offset, vtable) stores of Updatable vtables in the constructor and in
    the constructors it calls (last store per offset wins: derived last)."""
    found = {}
    todo = [(ctor, 0)]
    seen = set()
    order = []
    while todo:
        f, depth = todo.pop()
        if f in seen or depth > 2:
            continue
        seen.add(f)
        order.append(f)
        for ins in instructions(f, f + max(sizes.get(f, 0x300), 1)):
            if ins.mnemonic == 'call' and ins.op_str.startswith('0x') and depth < 2:
                t = int(ins.op_str, 16)
                if t not in HELPERS and t != 0x6f7810:
                    todo.append((t, depth + 1))
    for f in reversed(order):  # bases first
        for ins in instructions(f, f + max(sizes.get(f, 0x300), 1)):
            if ins.mnemonic == 'mov' and ins.op_str.startswith('dword ptr ['):
                m = re.search(r'\[(e[a-z]{2})(?: \+ 0x([0-9a-f]+))?\], 0x([0-9a-f]+)$', ins.op_str)
                if m and int(m.group(3), 16) in UPDATABLE_VTS:
                    found[(f, m.group(1), m.group(2) or '0')] = int(m.group(3), 16)
    return found


# ---------------------------------------------------------------- ownership
TOKEN = re.compile(r'(?i)(?<![0-9a-z_])(?:0x)?0{0,2}([0-9a-f]{6})(?![0-9a-z_])')
crate_cites = {}
for rs in (ROOT / 'crates').rglob('*.rs'):
    if 'target' in rs.parts:
        continue
    text = rs.read_text(encoding='utf-8', errors='replace')
    rel = str(rs.relative_to(ROOT)).replace('\\', '/')
    for m in set(TOKEN.findall(text)):
        crate_cites.setdefault(int(m, 16), set()).add(rel)

inventory = {}
for inv in sorted((ROOT / 'research').glob('phase8-*/inventory.json')):
    d = json.loads(inv.read_text(encoding='utf-8'))
    for f in d.get('functions', []) if isinstance(d, dict) else []:
        try:
            a = int(str(f.get('address')), 16)
        except ValueError:
            continue
        st = f.get('status', f.get('ported'))
        rust = f.get('rust') or f.get('rust_citations') or f.get('crate_refs') or f.get('cited_in') or f.get('crates_citing')
        inventory.setdefault(a, []).append({'inventory': inv.parent.name, 'status': st, 'role': f.get('role'),
                                            'rust': rust})


def owner(a):
    files = sorted(crate_cites.get(a, []))
    return {'address': '%06X' % a, 'noop': NOOPS.get(a), 'cited_in': files,
            'inventory': inventory.get(a, []),
            'state': 'no-op' if a in NOOPS else ('cited' if files else 'unported')}


def vtable_refs():
    refs = {}
    for name, vt, n in TABLES:
        for i in range(n):
            refs.setdefault(u(vt + 4 * i), []).append((name, '%06X' % vt, 4 * i))
    return refs


# ---------------------------------------------------------------- classes
classes = []
TABLES = []
for slot, name, ctor, cvt in COMPONENTS:
    ups = updatable_of(ctor)
    at8 = [v for (f, reg, offs), v in ups.items() if offs == '8']
    uvt = at8[-1] if at8 else None
    TABLES.append((name + ' component', cvt, 14))
    if uvt:
        TABLES.append((name + ' updatable', uvt, 10))
    classes.append({'kind': 'component', 'slot': slot, 'name': name, 'constructor': '%06X' % ctor,
                    'component_vtable': '%06X' % cvt, 'updatable_vtable': '%06X' % uvt if uvt else None,
                    'updatable_stores': ['%06X:%s+%s=%06X' % (f, reg, o, v) for (f, reg, o), v in ups.items()]})
for name, cls, ctor, vt in OBJECTS:
    ups = updatable_of(ctor)
    TABLES.append((name, vt, 0x140 // 4))
    for (f, reg, o), v in ups.items():
        TABLES.append((name + ' updatable +' + o, v, 10))
    classes.append({'kind': 'object', 'saved_class': cls, 'name': name, 'constructor': '%06X' % ctor,
                    'vtable': '%06X' % vt,
                    'updatables': [{'store': '%06X:%s+%s' % (f, reg, o), 'vtable': '%06X' % v} for (f, reg, o), v in ups.items()]})
REFS = vtable_refs()


def updatable_entry(v):
    return {'vtable': '%06X' % v,
            'gate_periodic_8': owner(u(v + 8)), 'gate_deferred_C': owner(u(v + 0xc)), 'gate_queue4_10': owner(u(v + 0x10)),
            'periodic_14 (queues 0/1/2, elapsed, data)': owner(u(v + 0x14)),
            'deferred_18 (queue 3, data)': owner(u(v + 0x18)),
            'deferred_1C (queue 4, data)': owner(u(v + 0x1c))}


# registrations
regs = []
for idx, ins in enumerate(INS):
    if ins.mnemonic == 'call' and ins.op_str.startswith('0x') and int(ins.op_str, 16) in HELPERS:
        h = int(ins.op_str, 16)
        q, kind, gate = HELPERS[h]
        prev = INS[max(0, idx - 10):idx]
        ecx = [p.op_str for p in prev if p.op_str.startswith('ecx,')]
        pushes = [p.op_str for p in prev if p.mnemonic == 'push']
        f = function_of(ins.address)
        regs.append({'site': '%06X' % ins.address, 'helper': '%06X' % h, 'queue': q, 'kind': kind,
                     'function': '%06X' % f if f else None, 'ecx': ecx[-1:] , 'pushes_before': pushes[-3:],
                     'function_in_vtables': [{'table': t, 'vtable': vt, 'slot': '+%X' % s} for t, vt, s in REFS.get(f, [])] if f else []})


def ancestors(f, depth=3):
    out, frontier, seen = [], {f}, {f}
    for d in range(depth):
        nxt = set()
        for x in frontier:
            for c in callers.get(x, ()):
                cf = function_of(c)
                if cf is None or cf in seen:
                    continue
                seen.add(cf)
                if cf in REFS:
                    out.append({'function': '%06X' % cf, 'depth': d + 1,
                                'in_vtables': [{'table': t, 'vtable': vt, 'slot': '+%X' % s} for t, vt, s in REFS[cf]]})
                nxt.add(cf)
        frontier = nxt
    return out


for reg in regs:
    if reg['function'] and not reg['function_in_vtables']:
        reg['vtable_ancestors'] = ancestors(int(reg['function'], 16))
    if reg['function']:
        reg['function_owner'] = owner(int(reg['function'], 16))

for c in classes:
    if c['kind'] == 'component':
        cvt = int(c['component_vtable'], 16)
        c['component_virtuals'] = {name: owner(u(cvt + o)) for o, name in COMPONENT_SLOTS.items()}
        if c['updatable_vtable']:
            c['updatable'] = updatable_entry(int(c['updatable_vtable'], 16))
        names = {c['name'] + ' component', c['name'] + ' updatable'}
    else:
        c['updatables'] = [dict(x, **updatable_entry(int(x['vtable'], 16))) for x in c['updatables']]
        names = {c['name']} | {c['name'] + ' updatable +' + x['store'].split('+')[-1] for x in c['updatables']}
    c['registrations'] = [
        {k: r[k] for k in ('site', 'helper', 'queue', 'kind', 'function', 'ecx', 'pushes_before')}
        | {'via': [x for x in r['function_in_vtables'] if x['table'] in names]
           or [a for a in r.get('vtable_ancestors', []) if any(t['table'] in names for t in a['in_vtables'])]}
        for r in regs
        if any(x['table'] in names for x in r['function_in_vtables'])
        or any(any(t['table'] in names for t in a['in_vtables']) for a in r.get('vtable_ancestors', []))]

attributed = {r['site'] for c in classes for r in c['registrations']}
other_updatables = [dict(updatable_entry(v), stored_by=['%06X:%s' % (s, p) for s, f, p in stores.get(v, [])])
                    for v in UPDATABLE_VTS
                    if not any(c.get('updatable_vtable') == '%06X' % v for c in classes)
                    and not any(x['vtable'] == '%06X' % v for c in classes for x in c.get('updatables', []))]

# summary of the per-tick callbacks nobody ports
todo = []
for c in classes:
    ents = [c['updatable']] if c.get('updatable') else c.get('updatables', [])
    for e in ents:
        for k in ('periodic_14 (queues 0/1/2, elapsed, data)', 'deferred_18 (queue 3, data)', 'deferred_1C (queue 4, data)'):
            o = e[k]
            if o['state'] == 'unported':
                todo.append({'class': c['name'], 'callback': k.split(' ')[0], 'address': o['address']})
            elif o['state'] == 'cited':
                pass
for e in other_updatables:
    for k in ('periodic_14 (queues 0/1/2, elapsed, data)', 'deferred_18 (queue 3, data)', 'deferred_1C (queue 4, data)'):
        if e[k]['state'] == 'unported':
            todo.append({'class': 'updatable ' + e['vtable'], 'callback': k.split(' ')[0], 'address': e[k]['address']})

doc = {'schema': 1, 'producer': 'research/phase8-saved-components/build_component_ticks.py', 'scope': __doc__,
       'queues': [
           {'id': 0, 'world_offset': '+1AC', 'kind': 'periodic', 'period': '0.0001 s (every tick)', 'callback': 'Updatable +14(elapsed, data)', 'add': '50AC00 (gate +8)'},
           {'id': 1, 'world_offset': '+1C0', 'kind': 'periodic', 'period': '1 s (16 buckets)', 'callback': 'Updatable +14(elapsed, data)', 'add': '50AC60 (gate +8)'},
           {'id': 2, 'world_offset': '+1D4', 'kind': 'periodic', 'period': '5 s', 'callback': 'Updatable +14(elapsed, data)', 'add': None},
           {'id': 3, 'world_offset': '+1E8', 'kind': 'deferred', 'callback': '4F05C0 -> Updatable +18(data)', 'add': '50ACC0 add_after / 50AD20 add_at (gate +C)'},
           {'id': 4, 'world_offset': '+208', 'kind': 'deferred', 'callback': '4F05B0 -> Updatable +1C(data)', 'add': '50AD80 add_after (gate +10)'}],
       'classes': classes, 'other_updatable_vtables': other_updatables,
       'registrations_unattributed': [r for r in regs if r['site'] not in attributed],
       'unported_callbacks': todo}
out = HERE / 'component-ticks.json'
out.write_text(json.dumps(doc, indent=1), encoding='utf-8')
print(len(classes), 'classes;', len(UPDATABLE_VTS), 'updatable vtables;', len(regs), 'registrations;',
      len(doc['registrations_unattributed']), 'unattributed;', len(todo), 'unported callbacks')
for t in todo:
    print('  unported', t)
