"""Unicorn oracle: original 4EDB10 Sun::SetColors and 4EDDC0 Sun deferred update.

Runs the ORIGINAL AA.exe code (image unchanged) on generated Sun objects at
PC24/PC53/PC64. Seam: 50ACC0 (Updatable::AddAfter on World+1E8) is recorded,
links a nonzero event at Sun+4 exactly as the original does, and returns (ret 8). 4EDA30/4EDD30/428170/
428430 run natively. Output: native-sun.json (consumed by the Rust replay test
crates/aa-runtime/src/live_sun_tests.rs).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, header, f32, instructions

WORLD_GLOBAL = 0xB36534
FIELDS = list(range(0x14, 0x70, 4))  # +14 .. +6C

def run_case(precision, case):
    m = Machine(precision)
    calls = []
    def add_after(mm):
        calls.append({'ecx': mm.ecx() - case['_sun'], 'delay': mm.arg(1), 'data': mm.arg(2)})
        # 50ACC0 (gate 6A71D0 true) links the new event at the Updatable's +4 head.
        mm.w(mm.ecx() + 4, 0xE0000000 + len(calls))
        mm.ret(0, pop=8)
    m.seam(0x50ACC0, add_after)
    world = m.alloc(0x300)
    m.w(WORLD_GLOBAL, world)
    m.w(world + 0xE0, case['now'])
    sun = m.alloc(0x90)
    case['_sun'] = sun
    m.w(sun + 4, case['scheduled'])
    for off, v in zip(FIELDS, case['words']):
        m.w(sun + off, v)
    if case['op'] == 'set_colors':
        vecs = [m.array(case['colors'][i]) for i in range(3)]
        m.call(0x4EDB10, ecx=sun, args=(vecs[0], vecs[1], vecs[2], case['duration']), pop=0x10)
    else:
        m.call(0x4EDDC0, ecx=sun, args=(case['data'],), pop=4)
    out = [m.r(sun + off) for off in FIELDS]
    scheduled_after = m.r(sun + 4)
    m.verify_v14()
    return {'words': out, 'calls': calls, 'scheduled_after': scheduled_after != 0, 'visited': len(m.visited)}, m.visited

def gen(rng):
    cases = []
    def col():
        return [f32(rng.choice([0.0, 0.25, 0.5, 0.75, 1.0, rng.random(), rng.random() * 2])) for _ in range(3)]
    for i in range(160):
        target = [col() for _ in range(3)]
        current = [list(c) if rng.random() < 0.25 else col() for c in target]
        words = {}
        for k in range(3):
            for j in range(3):
                words[0x14 + 12 * k + 4 * j] = target[k][j]
                words[0x38 + 12 * k + 4 * j] = current[k][j]
        dur_choices = [0.0, 0.5, 1.0, 2.0, 0.03, -0.5, rng.random() * 3]
        words[0x5C] = f32(rng.choice(dur_choices))
        cd = f32(rng.random()); td = cd if rng.random() < 0.3 else f32(rng.random())
        words[0x60] = cd; words[0x64] = td
        words[0x68] = f32(rng.choice(dur_choices))
        words[0x6C] = f32(rng.choice([0.0, 10.0, 12.5, rng.random() * 30]))
        for off in range(0x70 - 4, 0x70):
            pass
        now = f32(f32 and struct.unpack('<f', struct.pack('<I', words[0x6C]))[0] + rng.choice([0.0, 0.0625, 0.0333, 0.5, 1.0, 3.0]))
        w = [words.get(off, 0) for off in FIELDS]
        if i % 2 == 0:
            cases.append({'op': 'update', 'words': w, 'now': now, 'scheduled': rng.choice([0, 0x1234]), 'data': 0})
        else:
            cases.append({'op': 'set_colors', 'words': w, 'now': now, 'scheduled': rng.choice([0, 0x1234]),
                          'colors': [col() for _ in range(3)] if rng.random() < 0.7 else current,
                          'duration': f32(rng.choice(dur_choices))})
    return cases

def main():
    rng = random.Random(0x5eed)
    cases = gen(rng)
    rows, visited = [], {}
    for precision in ('Single24', 'Double53', 'Extended64'):
        for case in cases:
            result, v = run_case(precision, dict(case))
            visited.update(v)
            rows.append({'precision': precision, 'input': {k: case[k] for k in case if not k.startswith('_')}, 'output': result})
    doc = header(__file__, 'Unicorn execution of original 4EDB10/4EDDC0; seam 50ACC0 recorded')
    doc.update({'cases': rows, 'instructions': instructions(visited),
                'fields': [hex(o) for o in FIELDS]})
    (HERE / 'native-sun.json').write_text(json.dumps(doc, indent=1))
    print('cases', len(rows))

if __name__ == '__main__':
    main()
