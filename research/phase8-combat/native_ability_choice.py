"""Performer target check 0x57E920(target, kind, strict): the best code over
the Performer's abilities for using one of them on a target.

The whole routine executes unchanged with 0x4C8300 (context), 0x517F60 and
0x516AD0 (enabled-ability iterator). Seams (recorded in order, scripted
results; the range is returned on the x87 stack from binary64 so unstored
precision is exercised): actor virtual +108 (kingdom), target virtuals +84
and +5C, 0x4C8740 (records the context words), 0x4C84B0 and 0x4C8460
(record the point), 0x4CF110 and 0x4C8270.

Usage: python native_ability_choice.py [path/to/AA.exe]
Writes native-ability-choice.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-movement'))
import copy, hashlib, json, random, struct  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b, b2f, d2b  # noqa: E402

KINGDOM, MODIFIER = 0x4b1d, 0x3d0f


def stub(m, fn):
    a = m.stub_next
    m.stub_next += 16
    m.u.mem_write(a, b'\xcc' * 16)
    m.seams[a] = fn
    return a


def run(c):
    m = h.Machine(c['precision'])
    events = []
    actor = m.alloc(0x100)
    perf = m.alloc(0x40)
    m.w32(perf + 4, actor)
    m.w32(actor + 0x20, *c['actor_position'])
    abilities, index = [], {}
    for i, a in enumerate(c['abilities']):
        p = m.alloc(0x40)
        d = m.alloc(0x80)
        m.w32(p + 4, d)
        m.w8(p + 0x18, int(a['enabled']))
        m.w32(d + 0x54, a['flags54'])
        abilities.append(p)
        index[p] = i
    vec = m.alloc(4 * max(1, len(abilities)))
    m.w32(vec, *abilities) if abilities else None
    m.w32(perf + 0x14, vec, len(abilities))

    def v_kingdom(mm):
        assert mm.ecx() == actor
        events.append(['kingdom'])
        mm.ret(0, KINGDOM)

    avt = m.alloc(0x200)
    m.w32(actor, avt)
    m.w32(avt + 0x108, stub(m, v_kingdom))
    target = 0
    if c['target'] is not None:
        target = m.alloc(0x100)
        m.w32(target + 0x20, *c['target']['position'])

        def v_accepts(mm):
            assert mm.ecx() == target
            events.append(['accepts', mm.arg(0)])
            mm.ret(4, int(c['target']['accepts']))

        def v_bombard(mm):
            assert mm.ecx() == target
            events.append(['bombardable', mm.arg(0), mm.arg(1)])
            mm.ret(8, int(c['target']['bombardable']))

        tvt = m.alloc(0x200)
        m.w32(target, tvt)
        m.w32(tvt + 0x84, stub(m, v_accepts))
        m.w32(tvt + 0x5c, stub(m, v_bombard))

    def ability(mm):
        return c['abilities'][index[mm.ecx()]], index[mm.ecx()]

    def s_applies(mm):
        a, i = ability(mm)
        ctx, flag = mm.arg(0), mm.arg(1)
        words = m.r32(ctx, 12)
        assert words[2] == actor and words[7] == target
        assert words[1] == 0 and words[3:7] == [0] * 4 and words[8:] == [0] * 4
        events.append(['applies', i, words[0], int(words[2] == actor), int(words[7] == target), flag])
        mm.ret(8, int(a['applies']))

    def s_reaches(mm):
        a, i = ability(mm)
        assert mm.arg(0) == actor
        events.append(['reaches', i, *m.r32(mm.arg(1), 2)])
        mm.ret(8, int(a['reaches']))

    def s_ready(mm):
        a, i = ability(mm)
        assert mm.arg(0) == actor
        events.append(['ready', i, *m.r32(mm.arg(1), 2)])
        mm.ret(8, int(a['ready']))

    def s_modifier(mm):
        assert mm.ecx() == actor
        events.append(['modifier'])
        mm.ret(0, MODIFIER)

    def s_range(mm):
        a, i = ability(mm)
        assert mm.arg(0) == MODIFIER
        events.append(['range', i])
        mm.ret_float(d2b(a['range']), 'f64', 4)

    for address, fn in [(0x4c8740, s_applies), (0x4c84b0, s_reaches), (0x4c8460, s_ready),
                        (0x4cf110, s_modifier), (0x4c8270, s_range)]:
        m.seams[address] = fn
    result = m.call(0x57e920, ecx=perf, args=(target, c['kind'], int(c['strict'])))
    assert m.final_esp == h.STACK + 16
    assert m.fpu_top() == 0
    return {'result': result, 'events': events}


def cases():
    rnd = random.Random(0x57e920)
    rows = []
    for i in range(900):
        actor = [f2b(rnd.uniform(0, 200)), f2b(rnd.uniform(0, 200))]
        rng = rnd.choice([0.0, 5.0, 10.0, 25.0, 1.0 / 3.0, 40.0 + 2 ** -30])
        if rnd.random() < 0.35:
            # On or near the range circle along an axis.
            r = b2f(f2b(rng))
            tp = [f2b(b2f(actor[0]) + r), actor[1]] if rnd.random() < 0.5 else [actor[0], f2b(b2f(actor[1]) - r)]
            if rnd.random() < 0.5:
                tp[0] = (tp[0] + rnd.choice([-1, 1])) & 0xffffffff if tp[0] & 0x7fffffff else tp[0]
        else:
            tp = [f2b(b2f(actor[0]) + rnd.uniform(-50, 50)), f2b(b2f(actor[1]) + rnd.uniform(-50, 50))]
        target = None
        if rnd.random() < 0.95:
            target = {'position': tp, 'accepts': rnd.random() < 0.9, 'bombardable': rnd.random() < 0.8}
        abilities = []
        for _ in range(rnd.randrange(0, 5)):
            abilities.append({'enabled': rnd.random() < 0.8,
                              'flags54': rnd.choice([0, 8, 8, 0x28, 4, 0x2000c]),
                              'applies': rnd.random() < 0.8, 'reaches': rnd.random() < 0.8,
                              'ready': rnd.random() < 0.75,
                              'range': rnd.choice([rng, rng, rng + 2 ** -40, rng * 0.5, 100.0])})
        rows.append({'name': f'choice{i}', 'kind': rnd.choice([1, 2, 3, 3, 4]),
                     'strict': rnd.random() < 0.6, 'actor_position': actor, 'target': target,
                     'abilities': abilities})
    out = []
    for p in h.PRECISION:
        for c in rows:
            out.append({**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p})
    return out


def main():
    rows = []
    for c in cases():
        try:
            rows.append({'input': c, 'output': run(c)})
        except Exception:
            print(c['name'])
            raise
    counts = {}
    for r in rows:
        counts[r['output']['result']] = counts.get(r['output']['result'], 0) + 1
    print(counts)
    doc = {'schema': 1, 'scope': '0x57E920 Performer target check', 'original_sha256': h.ORIGINAL_SHA,
           'v14_sha256': h.V14_SHA, 'case_count': len(rows), 'cases': rows,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(h.VISITED.items())]}
    out = HERE / 'native-ability-choice.json'
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps({'file': out.name, 'cases': len(rows), 'instructions': len(h.VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
