"""Native oracle for the view blast impulse 0x43B910: every type-2 view
object of the list at [this+94]+100 (linked through +8, iterator 0x42EEC0)
whose virtual +9C holds and whose distance from the blast centre, less its
own virtual +38 radius squared, is under the blast radius squared, gets
0x455510 with the falloff-scaled unit direction (0x43A970) times the
strength.

Original code runs for 0x43B910, 0x42EEC0, 0x43A970 and the table builder
0x869DC0 / 0x8693B0 (its allocation 0x6F7810 answered from the laboratory
heap). Recorded seams: object virtual +9C (answers the case's byte),
virtual +38 (returns the case's binary64 radius as the x87 value) and
0x455510 (records the object, its three impulse words and the fourth
argument). Inputs: lists of 0..6 objects of type 2 and others, random
positions near and far, radii, strengths and blast radii, at PC24, PC53 and
PC64.

Usage: native_view_blast.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP  # noqa: E402


def f64(value):
    return struct.unpack('<Q', struct.pack('<d', value))[0]


def build_cases(rng):
    cases = []
    for _ in range(300):
        cx, cy = rng.uniform(-500, 500), rng.uniform(-500, 500)
        blast = rng.choice((rng.uniform(1.0, 60.0), rng.uniform(0.0, 2.0), 0.0))
        nodes = []
        for _ in range(rng.randint(0, 6)):
            spread = rng.choice((5.0, 40.0, 200.0))
            x = cx + rng.uniform(-spread, spread)
            y = cy + rng.uniform(-spread, spread)
            if rng.random() < 0.08:
                x, y = cx, cy
            radius = rng.choice((0.0, rng.uniform(0.0, 10.0), rng.uniform(0.0, 60.0)))
            if rng.random() < 0.5:
                radius = h.from_f32(h.f32(radius))
            nodes.append({'kind': rng.choice((2, 2, 2, 1, 3)), 'reacts': rng.choice((1, 1, 0, 0x100)),
                          'position': [h.f32(x), h.f32(y)], 'radius': f64(radius)})
        cases.append({'center': [h.f32(cx), h.f32(cy), h.f32(rng.uniform(-5, 5))],
                      'strength': h.f32(rng.uniform(-30.0, 300.0)), 'blast_radius': h.f32(blast),
                      'nodes': nodes, 'empty_holder': rng.random() < 0.05})
    return cases


def run(precision, cases):
    m = h.Machine(precision)

    def new(mm):
        size = mm.arg(0)
        block = mm.alloc(size)
        mm.u.mem_write(block, b'\xA5' * size)
        mm.ret(0, block)
    m.hooks[0x6F7810] = new
    m.call(0x869DC0, pop=0)
    state = {}

    def reacts(mm):
        index = state['index'][mm.reg(UC_X86_REG_ECX)]
        state['events'].append({'reacts': index})
        mm.ret(0, state['case']['nodes'][index]['reacts'])

    def radius(mm):
        index = state['index'][mm.reg(UC_X86_REG_ECX)]
        state['events'].append({'radius': index})
        mm.ret_ext(0, state['case']['nodes'][index]['radius'])

    def apply(mm):
        index = state['index'][mm.reg(UC_X86_REG_ECX)]
        state['events'].append({'apply': index, 'impulse': [mm.arg(0), mm.arg(1), mm.arg(2)],
                                'word': mm.arg(3)})
        mm.ret(16)
    m.hooks[0x455510] = apply
    vtable = m.vtable({0x38: radius, 0x9C: reacts})
    world = m.alloc(0x100)
    holder = m.alloc(0x200)
    m.w(world + 0x94, holder)
    rows = []
    for case in cases:
        state['case'] = case
        state['events'] = []
        state['index'] = {}
        previous = 0
        objects = []
        for index, node in enumerate(case['nodes']):
            obj = m.alloc(0x60)
            m.w(obj, vtable)
            m.w(obj + 0x18, node['kind'])
            m.w(obj + 0x38, node['position'][0])
            m.w(obj + 0x3C, node['position'][1])
            m.w(obj + 8, 0)
            state['index'][obj] = index
            objects.append(obj)
        for a, b in zip(objects, objects[1:]):
            m.w(a + 8, b)
        m.w(holder + 0x100, 0 if case['empty_holder'] or not objects else objects[0])
        c = case['center']
        m.call(0x43B910, ecx=world,
               args=(c[0], c[1], c[2], case['strength'], case['blast_radius']), pop=0x14)
        rows.append(state['events'])
    return rows, m.visited


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    rng = random.Random(0x43B910)
    cases = build_cases(rng)
    visited = {}
    results = {}
    for precision in h.PRECISIONS:
        rows, seen = run(precision, cases)
        results[precision] = rows
        visited.update(seen)
    for index, case in enumerate(cases):
        case['events'] = {p: results[p][index] for p in h.PRECISIONS}
    h.report('native-view-blast-fixtures.json',
             '0x43B910 view blast impulse with 0x42EEC0, 0x43A970 and its table; '
             'virtual +9C, +38 and 0x455510 recorded.',
             cases, visited)


if __name__ == '__main__':
    main()
