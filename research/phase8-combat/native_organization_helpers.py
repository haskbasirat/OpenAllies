"""Small OrganizationComponent helpers: 0x578850 (member notification),
0x578430 (target id +D4), 0x578D50 (AddRecruitItem, +34 slot) and 0x577FA0.

Whole routines execute unchanged (0x578D50 with its diagnostic 0x404F90,
whose body returns because the log level at 0xB38B7C is 0). The only seam is
0x4CEF70 on a member (recorded with its arguments) for 0x578850.

Usage: python native_organization_helpers.py [path/to/AA.exe]
Writes native-organization-helpers.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-movement'))
import copy, hashlib, json, random  # noqa: E402
import harness as h  # noqa: E402

OWNER = 0x0a0b0c0d


def run(c):
    m = h.Machine('Single24')
    events = []
    comp = m.alloc(0x100)
    owner = m.alloc(0x40)
    m.w32(comp + 4, owner)
    m.w32(owner + 0x14, 0x00090009)
    e = c['entry']
    if e == 'notify':
        members = {}
        ptrs = []
        for i, mem in enumerate(c['members']):
            if mem is None:
                ptrs.append(0)
                continue
            p = m.alloc(0x100)
            m.w32(p + 0x78, mem['w78'])
            m.w32(p + 0xa4, mem['wa4'])
            members[p] = i
            ptrs.append(p)
        arr = m.alloc(4 * max(1, len(ptrs)))
        if ptrs:
            m.w32(arr, *ptrs)
        m.w32(comp + 0x28, arr, len(ptrs))
        msg = m.alloc(0x20)
        m.w32(msg + 8, c['kind'], c['word'])
        m.w8(msg + 0x10, int(c['handled']))

        def s_notify(mm):
            events.append(['notify', members[mm.ecx()], mm.arg(0), int(mm.arg(1) == owner)])
            mm.ret(8)

        m.seams[0x4cef70] = s_notify
        m.call(0x578850, ecx=comp, args=(msg,))
        assert m.final_esp == h.STACK + 8
        return {'events': events}
    if e == 'target':
        m.w32(comp + 0xd4, c['before'])
        obj = 0
        if c['object'] is not None:
            obj = m.alloc(0x100)
            m.w32(obj + 0x14, c['object']['gtid'])
            m.w32(obj + 0xc0, c['object']['c0'])
        m.call(0x578430, ecx=comp, args=(obj,))
        assert m.final_esp == h.STACK + 8
        return {'target': m.r32(comp + 0xd4)}
    if e == 'recruit':
        arr = m.alloc(4 * len(c['recruits']))
        m.w32(arr, *c['recruits'])
        m.w32(comp + 0x34, arr)
        item = m.alloc(0x40)
        idef = m.alloc(0x40)
        m.w32(item + 4, idef)
        m.w32(idef + 8, 0x12345678)
        m.call(0x578d50, ecx=comp, args=(item, c['slot']))
        assert m.final_esp == h.STACK + 12
        return {'recruits': [1 if w == item else w for w in m.r32(arr, len(c['recruits']))]}
    inner = m.alloc(0x40)
    m.w32(inner + 0x18, c['inner18'])
    m.w32(comp + 0x18, inner)
    m.w32(comp + 0x40, c['w40'], c['w44'])
    r = m.call(0x577fa0, ecx=comp)
    return {'ready': r & 0xff}


def cases():
    rnd = random.Random(0x578850)
    rows = []
    for i in range(200):
        members = [None if rnd.random() < 0.2 else {'w78': rnd.choice([0, 0x55]), 'wa4': rnd.choice([0, 0x66])}
                   for _ in range(rnd.randrange(0, 7))]
        rows.append({'name': f'notify{i}', 'entry': 'notify', 'members': members,
                     'kind': rnd.choice([7, 7, 3, 0]), 'word': rnd.getrandbits(32),
                     'handled': rnd.random() < 0.2})
    for i in range(80):
        rows.append({'name': f'target{i}', 'entry': 'target', 'before': rnd.choice([0, 0x00050005]),
                     'object': None if rnd.random() < 0.3 else {'gtid': 0x00070031, 'c0': rnd.choice([0, 1])}})
    for i in range(40):
        n = rnd.randrange(1, 6)
        rows.append({'name': f'recruit{i}', 'entry': 'recruit', 'recruits': [rnd.choice([0, 9]) for _ in range(n)],
                     'slot': rnd.randrange(n)})
    for i in range(60):
        rows.append({'name': f'ready{i}', 'entry': 'ready', 'w40': rnd.choice([0, 1, 0x80000000]),
                     'w44': rnd.choice([0, 0, 5]), 'inner18': rnd.choice([0, 3, -1, 0x7fffffff, -0x80000000])})
    return rows


def main():
    rows = []
    for c in cases():
        try:
            rows.append({'input': c, 'output': run(c)})
        except Exception:
            print(c['name'])
            raise
    doc = {'schema': 1, 'scope': '0x578850 0x578430 0x578D50 0x577FA0', 'original_sha256': h.ORIGINAL_SHA,
           'v14_sha256': h.V14_SHA, 'case_count': len(rows), 'cases': rows,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(h.VISITED.items())]}
    out = HERE / 'native-organization-helpers.json'
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps({'file': out.name, 'cases': len(rows), 'instructions': len(h.VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
