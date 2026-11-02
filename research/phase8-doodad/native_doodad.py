"""Oracle: the DynamicDoodad deferred updates of the original AA.exe.

Runs the ORIGINAL code under Unicorn (research/phase8-ai/native_machine.py, the
accepted image mapped unchanged) on generated inputs and writes
native-doodad.json, which crates/aa-play/tests/doodad_native.rs replays.

Functions run: 0x5902C0 (Disturbable +18) and 0x58FA30 (Breakable +18).
Seams, all recorded: 0x5901F0 (the scan), 0x4F02B0 (GetRNG), 0x85D840 (draw),
0x50ACC0 (re-arm), 0x50B250 (cancel), 0x42A590 (topple action list) and vtable
+0xC of the Disturbable (0x590380, its action list).

Precision: PC53, the simulation thread's.
"""
from pathlib import Path
import json
import random
import struct
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[0] / 'phase8-ai'))
from native_machine import Machine, f32, from_f32, original, sha  # noqa: E402

GWORLD_POINTER = 0xB36534
GWORLD_E0 = 0xE0


def machine():
    m = Machine('Double53')
    m.gw = m.alloc(0x400)
    m.w(GWORLD_POINTER, m.gw)
    return m


def set_seams(m, log, scan, draw):
    m.seam(0x5901F0, lambda m: (log.append(['scan']), m.ret(1 if scan else 0)))
    m.seam(0x4F02B0, lambda m: (log.append(['getter']), m.ret(0x1234)))

    def on_draw(m):
        log.append(['draw'])
        m.ret(draw)
    m.seam(0x85D840, on_draw)

    def on_rearm(m):
        log.append(['rearm', m.arg(1), m.arg(2)])
        m.ret(1, pop=8)
    m.seam(0x50ACC0, on_rearm)
    m.seam(0x50B250, lambda m: (log.append(['cancel']), m.ret(0)))

    def on_action(m):
        log.append(['topple', m.arg(1), m.arg(2)])
        m.ret(0, pop=8)
    m.seam(0x42A590, on_action)


def disturbable(case):
    m = machine()
    log = []
    set_seams(m, log, case['scan'], case['draw'])
    m.w(m.gw + GWORLD_E0, case['now'])
    data = m.alloc(0x40)
    for off, key in [(0x10, 'update_time'), (0x18, 'min'), (0x1C, 'max')]:
        m.w(data + off, case[key])
    act = m.stub(lambda m: (log.append(['act']), m.ret(0)))
    vtable = m.alloc(0x20)
    m.w(vtable + 0xC, act)
    comp = m.alloc(0x40)
    m.w(comp, vtable)
    updatable = comp + 0xC
    # Updatable +8 = component +0x14 definition, +0xC = component +0x18 time.
    m.w(updatable + 8, data)
    m.w(updatable + 0xC, case['next'])
    m.call(0x5902C0, ecx=updatable, args=[0], pop=4)
    return {'log': log, 'next': m.r(updatable + 0xC)}, m


def breakable(case):
    m = machine()
    log = []
    set_seams(m, log, False, 0)
    m.w(m.gw + GWORLD_E0, case['now'])
    data = m.alloc(0x60)
    for off, key in [(0x10, 'update_time'), (0x1C, 'topple'), (0x20, 'bounce'), (0x34, 'sink'), (0x38, 'dead'), (0x3C, 'grow')]:
        m.w(data + off, case[key])
    m.wb(data + 0x30, case['regrow'])
    comp = m.alloc(0x40)
    updatable = comp + 0xC
    m.w(updatable - 8, m.alloc(4))  # component +4 owner
    m.w(updatable + 8, data)
    m.w(updatable + 0xC, case['state'])
    m.w(updatable + 0x10, case['time'])
    m.w(updatable + 0x14, case['grow_start'])
    m.w(updatable + 0x20, case['scale'])
    m.call(0x58FA30, ecx=updatable, args=[0], pop=4)
    return {
        'log': log,
        'state': m.r(updatable + 0xC),
        'time': m.r(updatable + 0x10),
        'grow_start': m.r(updatable + 0x14),
        'scale': m.r(updatable + 0x20),
    }, m


def main():
    rng = random.Random(0x590323)
    f = lambda a, b: f32(rng.uniform(a, b))
    visited = {}
    cases = []
    specials = [0, 1, 0x7fffffff, 0x80000000, 0xffffffff, 0x10000]
    for i in range(240):
        now = f(0, 900) if i % 7 else 0
        c = {
            'scan': bool(i % 3),
            'draw': specials[i] if i < len(specials) else rng.getrandbits(32),
            'now': now,
            'next': (now if i % 5 == 0 else f(0, 900)),
            'update_time': f(0.1, 20),
            'min': f(0, 30),
        }
        c['max'] = f32(from_f32(c['min']) + rng.uniform(0, 60))
        out, m = disturbable(c)
        visited.update(m.visited)
        cases.append({'input': c, 'output': out})
    bcases = []
    for i in range(480):
        state = i % 9
        c = {
            'state': state,
            'now': f(0, 900),
            'update_time': f(0.1, 20),
            'topple': f(0.1, 8),
            'bounce': f(0.1, 8),
            'sink': f(0.1, 40),
            'dead': f(0.1, 40),
            'grow': f(0.1, 40),
            'regrow': i % 2,
            'grow_start': f(0, 900),
            'scale': f(0, 1),
        }
        c['time'] = f32(from_f32(c['now']) - rng.choice([0.0, 0.05, 0.5, 2.0, 9.0, 30.0, 95.0, 300.0]))
        out, m = breakable(c)
        visited.update(m.visited)
        bcases.append({'input': c, 'output': out})
    doc = {
        'program_sha256': sha(original.b),
        'precision': 'Double53',
        'functions': ['0x5902C0', '0x58FA30'],
        'executed_instructions': len(visited),
        'disturbable': cases,
        'breakable': bcases,
    }
    (HERE / 'native-doodad.json').write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='ascii', newline='\n')
    print('disturbable', len(cases), 'breakable', len(bcases), 'instructions', len(visited))


if __name__ == '__main__':
    main()
