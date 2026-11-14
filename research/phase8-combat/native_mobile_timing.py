"""Timed actor state 26: 0x575B20 / 0x575A30 on a component whose +4 is its
actor, and the ratio helper 0x571700.

Whole routines execute unchanged with 0x529C30. Seams (recorded in order,
scripted results; the health fraction is returned on the x87 stack from
binary64 so unstored precision is exercised): 0x4D1350 and 0x4D0230 on the
actor, 0x50B250 and 0x50ACC0 on the component's timer list (+8), 0x4D64F0
on the actor. 0x571700's ST0 is stored as an 80-bit value by a scratch stub.

Usage: python native_mobile_timing.py [path/to/AA.exe]
Writes native-mobile-timing.json next to this script.
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


def run_timed(c):
    m = h.Machine(c['precision'])
    events = []
    world = m.alloc(0x100)
    m.w32(0xb36534, world)
    m.w32(world + 0xe0, c['time'])
    comp = m.alloc(0x40)
    actor = m.alloc(0x200)
    definition = m.alloc(0x600)
    m.w32(comp + 4, actor)
    m.w32(comp + 0x14, c['deadline'])
    m.w32(actor + 4, definition)
    m.w32(actor + 0x100, c['flags'])
    m.w32(definition + 0x54c, c['threshold'], c['delay'], c['cap'])

    def s_refresh(mm):
        assert mm.ecx() == actor
        events.append(['refresh'])
        mm.ret(0)

    def s_health(mm):
        assert mm.ecx() == actor and mm.arg(0) & 0xff == 0
        events.append(['health'])
        mm.ret_float(d2b(c['health']), 'f64', 4)

    def s_clear(mm):
        assert mm.ecx() == comp + 8
        events.append(['clear'])
        mm.ret(0)

    def s_state(mm):
        assert mm.ecx() == actor
        events.append(['state', mm.arg(0), mm.arg(1) & 0xff])
        mm.ret(8)

    def s_timer(mm):
        assert mm.ecx() == comp + 8
        events.append(['timer', mm.arg(0), mm.arg(1)])
        mm.ret(8)

    for address, fn in [(0x4d1350, s_refresh), (0x4d0230, s_health), (0x50b250, s_clear),
                        (0x4d64f0, s_state), (0x50acc0, s_timer)]:
        m.seams[address] = fn
    m.call(0x575a30 if c['entry'] == 'start' else 0x575b20, ecx=comp)
    assert m.final_esp == h.STACK + 4 and m.fpu_top() == 0
    return {'deadline': m.r32(comp + 0x14), 'events': events}


def run_ratio(c):
    m = h.Machine(c['precision'])
    out = m.alloc(16)
    s = m.stub_next
    m.stub_next += 32
    code = (b'\x68' + struct.pack('<I', c['b']) + b'\x68' + struct.pack('<I', c['a']) + b'\xe8'
            + struct.pack('<i', 0x571700 - (s + 15)) + b'\x83\xc4\x08\xdb\x3d' + struct.pack('<I', out) + b'\xc3')
    m.u.mem_write(s, code)
    m.call(s)
    assert m.fpu_top() == 0
    raw = bytes(m.u.mem_read(out, 10))
    return {'extended': raw.hex()}


def cases():
    rnd = random.Random(0x575b20)
    rows = []
    for i in range(500):
        threshold = rnd.choice([0.25, 0.5, 1.0 / 3.0, 0.0, 0.75])
        health = rnd.choice([threshold, threshold + 2 ** -40, threshold - 2 ** -40, 0.1, 0.9,
                             b2f(f2b(threshold) + 1), b2f(f2b(threshold) - 1) if threshold else -0.0])
        time = rnd.choice([0.0, 12.5, 100.1, 1000.3, 4096.75])
        deadline = rnd.choice([time, time + 1.0, time + 0.1, time - 3.0, time + 30.0, 0.0])
        delay = rnd.choice([5.0, 2.5, 10.0, 0.3])
        rows.append({'name': f'timed{i}', 'entry': rnd.choice(['start', 'update', 'update']),
                     'time': f2b(time), 'deadline': f2b(deadline),
                     'flags': rnd.choice([0, 0x2000000, 0x2000000, 0x2000001, 0x4000000]),
                     'threshold': f2b(threshold), 'health': health, 'delay': f2b(delay),
                     'cap': f2b(rnd.choice([delay, delay * 0.5, delay + 1.0, 4.0, 3.0]))})
    ratios = []
    for i in range(300):
        a = rnd.choice([0.0, -1.0, 3.0, 1.0 / 3.0, 7.0, 1e-30, rnd.uniform(0.01, 100)])
        b = rnd.choice([0.0, 1.0, 2.0 / 3.0, -5.0, rnd.uniform(-100, 100)])
        ratios.append({'name': f'ratio{i}', 'entry': 'ratio', 'a': f2b(a), 'b': f2b(b)})
    out = []
    for p in h.PRECISION:
        for c in rows + ratios:
            out.append({**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p})
    return out


def main():
    rows = []
    for c in cases():
        try:
            rows.append({'input': c, 'output': run_ratio(c) if c['entry'] == 'ratio' else run_timed(c)})
        except Exception:
            print(c['name'])
            raise
    print({'cases': len(rows), 'started': sum(1 for r in rows if ['clear'] in r['output'].get('events', []))})
    doc = {'schema': 1, 'scope': '0x575B20 0x575A30 0x571700', 'original_sha256': h.ORIGINAL_SHA,
           'v14_sha256': h.V14_SHA, 'case_count': len(rows), 'cases': rows,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(h.VISITED.items())]}
    out = HERE / 'native-mobile-timing.json'
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps({'file': out.name, 'cases': len(rows), 'instructions': len(h.VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
