"""KKC_UAIStateBombard::Activate 0x538120(mode, target) on the element
bombard state.

The whole routine executes unchanged with 0x52A000 (current state test on
the Mind's stack head), 0x529C30 and the diagnostic 0x404F60 (its body
returns because the log level at 0xB38B7C is 0). Seams (recorded in order,
scripted results): 0x4D4FF0 relation on the Mind actor (the third word's
upper bytes are the caller's target argument; only its low byte is the
no-locomotion flag), 0x537AF0 (set target) and 0x52A810 (activate; its EAX
carries garbage upper bytes so only AL counts).

Usage: python native_uai_bombard_activate.py [path/to/AA.exe]
Writes native-uai-bombard-activate.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-movement'))
import copy, hashlib, json, random  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b  # noqa: E402

TARGET = 1


def run(c):
    m = h.Machine(c['precision'])
    events = []
    state = m.alloc(0x40)
    mind = m.alloc(0x200)
    actor = m.alloc(0x100)
    m.w32(state + 4, mind)
    m.w32(state + 0x10, c['state_gtid'])
    m.w32(state + 0x18, 0x11111111, 0x22222222)
    m.w32(mind + 4, actor)
    m.w32(actor + 0x14, 0x00070007)
    m.w32(actor + 0x6c, 0x5000 if c['locomotion'] else 0)
    if c['stack'] is not None:
        node = m.alloc(0x10)
        m.w32(node, state if c['stack'] == 'self' else m.alloc(0x40))
        m.w32(mind + 0x10, node)
    target = 0
    if c['target'] is not None:
        target = m.alloc(0x100)
        m.w32(target + 0x14, c['target']['gtid'])
        m.w32(target + 0x20, *c['target']['position'])

    def ident(p):
        return TARGET if p == target and p else p

    def s_relation(mm):
        assert mm.ecx() == actor
        events.append(['relation', ident(mm.arg(0)), mm.arg(1), mm.arg(2) & 0xff])
        mm.ret(12, c['relation'])

    def s_set(mm):
        assert mm.ecx() == state
        events.append(['set_target', ident(mm.arg(0))])
        mm.ret(4, 0)

    def s_activate(mm):
        assert mm.ecx() == state
        events.append(['activate', mm.arg(0)])
        mm.ret(4, 0xabcdef00 | int(c['activated']))

    for address, fn in [(0x4d4ff0, s_relation), (0x537af0, s_set), (0x52a810, s_activate)]:
        m.seams[address] = fn
    r = m.call(0x538120, ecx=state, args=(c['mode'], target))
    assert m.final_esp == h.STACK + 12
    return {'result': r & 0xff, 'point': m.r32(state + 0x18, 2), 'events': events}


def cases():
    rnd = random.Random(0x538120)
    rows = []
    for i in range(400):
        gtid = rnd.choice([0x00030011, 0x00040011])
        target = None if rnd.random() < 0.05 else {
            'gtid': 0x00030011, 'position': [f2b(rnd.uniform(0, 300)), f2b(rnd.uniform(0, 300))]}
        rows.append({'name': f'activate{i}', 'precision': 'Single24', 'mode': rnd.choice([0, 1, 2, 3]),
                     'state_gtid': gtid, 'stack': rnd.choice([None, 'self', 'self', 'other']),
                     'locomotion': rnd.random() < 0.6, 'target': target,
                     'relation': rnd.choice([0, 0, 5, 5, 1, 2, 3, 4, 6]),
                     'activated': rnd.random() < 0.7})
    return rows


def main():
    rows = []
    for c in cases():
        try:
            rows.append({'input': c, 'output': run(c)})
        except Exception:
            print(c['name'])
            raise
    print({'cases': len(rows), 'true': sum(r['output']['result'] for r in rows)})
    doc = {'schema': 1, 'scope': '0x538120 KKC_UAIStateBombard::Activate', 'original_sha256': h.ORIGINAL_SHA,
           'v14_sha256': h.V14_SHA, 'case_count': len(rows), 'cases': rows,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(h.VISITED.items())]}
    out = HERE / 'native-uai-bombard-activate.json'
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps({'file': out.name, 'cases': len(rows), 'instructions': len(h.VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
