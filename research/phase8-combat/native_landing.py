"""Native oracle for the actor landing / placement refresh 4D64A0 and the
siege site 4D11D0. Recorded seams: 562380 (Body), 4F6760 (world), 4D4130
(terrain refresh), 563D50 (Character), 4D1350, 4F2220 (world, returns a
word) and the actor / target virtual +100; the registry is [B364C0].

Usage: native_landing.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


def main():
    rng = random.Random(0x4D64A0)
    m = h.Machine('Double53')
    world = m.alloc(0x100)
    m.w(h.WORLD_ROOT, world)
    registry = m.alloc(0x20004 + 0x40 * 4)
    m.w(h.REGISTRY_ROOT, registry)
    landing, sieges = [], []
    for _ in range(120):
        events = []
        a = m.alloc(0x200)
        names = {a: 'actor'}
        spec = {'flags': rng.choice((0, 1, 0x101, 2)), 'body': rng.random() < 0.6,
                'word_bc': rng.choice((0, 0, 3)), 'character': rng.random() < 0.6}
        m.w(a + 8, spec['flags'])
        m.w(a + 0xBC, spec['word_bc'])
        if spec['body']:
            b = m.alloc(0x40)
            names[b] = 'body'
            m.w(a + 0x60, b)
        if spec['character']:
            c = m.alloc(0x40)
            names[c] = 'character'
            m.w(a + 0x68, c)

        def seam(tag, pop, ev=events, nm=names):
            def run(mm):
                rec = [tag, nm.get(mm.reg(UC_X86_REG_ECX), 'world' if mm.reg(UC_X86_REG_ECX) == world else '?')]
                if tag == '4F6760':
                    rec += [nm.get(mm.arg(0), '?'), mm.arg(1)]
                ev.append(rec)
                mm.ret(pop)
            return run
        m.hooks[0x562380] = seam('562380', 0)
        m.hooks[0x4F6760] = seam('4F6760', 8)
        m.hooks[0x4D4130] = seam('4D4130', 0)
        m.hooks[0x563D50] = seam('563D50', 0)
        m.hooks[0x4D1350] = seam('4D1350', 0)
        m.call(0x4D64A0, ecx=a, pop=0)
        landing.append({**spec, 'events': events})
    for _ in range(120):
        events = []
        a = m.alloc(0x200)
        own = rng.choice((0, 0, 0x1111))
        target_value = rng.choice((0, 0x2222))
        site = rng.getrandbits(32)
        x, y = rng.getrandbits(32), rng.getrandbits(32)
        m.w(a + 0x20, x)
        m.w(a + 0x24, y)
        spec = {'own': own, 'target_value': target_value, 'site': site, 'position': [x, y], 'mind': None}

        def v100(value, tag, ev=events):
            def run(mm):
                ev.append(['virtual100', tag])
                mm.ret(0, value)
            return run
        m.w(a, m.vtable({0x100: v100(own, 'actor')}))
        if rng.random() < 0.75:
            mind = m.alloc(0x40)
            m.w(a + 0x70, mind)
            index = rng.randrange(1, 0x40)
            generation = rng.getrandbits(16)
            live = rng.random() < 0.7
            stale = rng.random() < 0.2
            target = m.alloc(0x40)
            m.w(target, m.vtable({0x100: v100(target_value, 'target')}))
            m.u.mem_write(registry + 4 + index * 2, generation.to_bytes(2, 'little'))
            m.w(registry + 0x20004 + index * 4, target if live else 0)
            gtid = (((generation + 1) & 0xFFFF if stale else generation) << 16) | index
            m.w(mind + 0x28, gtid)
            spec['mind'] = {'gtid': gtid, 'generation': generation, 'live': live}

        def query(mm, ev=events, s=site):
            assert mm.reg(UC_X86_REG_ECX) == world
            ev.append(['4F2220', mm.arg(0), mm.arg(1)])
            mm.ret(8, s)
        m.hooks[0x4F2220] = query
        result = m.call(0x4D11D0, ecx=a, pop=0)
        sieges.append({**spec, 'result': result, 'events': events})
    h.report('native-landing-fixtures.json', '4D64A0 landing refresh and 4D11D0 siege site with recorded seams.',
             landing, m.visited, extra={'sieges': sieges})


if __name__ == '__main__':
    main()
