"""Original AA.exe 0x5E0690, the default state's widget routing.

Runs the ORIGINAL 0x5E0690 (both pinned images, PC53; no x87 arithmetic) on
a state object (+4 pressed widget) and the UI manager DAT_00B3653C (+0x60
pressed widget, +0x74 root, +0x8C focus list). Seam: 0x5EFFF0 (the widget
chain), which records the widget it is called on and returns a scripted
answer. Each row: the +4 and UI+0x60 widgets, the event, the native AL,
the +4 afterwards and the chain calls in order. Writes native-route-5e0690.json.
"""
from pathlib import Path
import sys, json, random
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine

rng = random.Random(0x5E0690)
WIDGETS = ['vworld', 'hud_a', 'hud_b', 'minimap', 'root', 'focus']

def run(spec):
    m = Machine('Double53')
    ptr = {w: m.alloc(0x40) for w in WIDGETS}
    name = {v: k for k, v in ptr.items()}
    ui = m.alloc(0x100)
    m.w(0xB3653C, ui)
    m.w(ui + 0x60, ptr[spec['ui_60']] if spec['ui_60'] else 0)
    m.w(ui + 0x74, ptr['root'])
    if spec['focus']:
        node = m.alloc(8)
        m.w(node, ptr['focus'])
        m.w(ui + 0x8C, node)
    else:
        m.w(ui + 0x8C, 0)
    state = m.alloc(0x20)
    m.w(state + 4, ptr[spec['state_4']] if spec['state_4'] else 0)
    calls = []
    def chain(m):
        w = name.get(m.ecx(), hex(m.ecx()))
        calls.append(w)
        m.ret(1 if w in spec['chain_true'] else 0, 4)
    m.seam(0x5EFFF0, chain)
    ev = m.alloc(0x30)
    m.w(ev + 4, spec['flags'])
    m.u.mem_write(ev + 8, spec['code'].to_bytes(2, 'little'))
    al = m.call(0x5E0690, ecx=state, args=(ev,), pop=4) & 0xff
    after = m.r(state + 4)
    m.verify_v14()
    return {'al': al, 'state_4': name.get(after) if after else None, 'chain': calls}, m.visited

def gen():
    cases = []
    choices = [None, 'vworld', 'hud_a', 'hud_b', 'minimap']
    events = [(0x100, 0x81), (0x100, 0x82), (0x108, 0x81), (0x101, 0x81), (0x101, 0x82), (0x109, 0x81), (0x100, 0x80)]
    for k in range(300):
        code, flags = rng.choice(events)
        cases.append({'state_4': rng.choice(choices), 'ui_60': rng.choice(choices), 'code': code, 'flags': flags,
                      'focus': rng.random() < 0.7,
                      'chain_true': [w for w in WIDGETS if rng.random() < 0.3]})
    return cases

def main():
    rows, visited = [], {}
    for spec in gen():
        out, v = run(spec)
        visited.update(v)
        rows.append({'args': spec, 'native': out})
    doc = nm.header(__file__, 'Unicorn 2.1.4, original and v14 images; seam 0x5EFFF0')
    doc['rows'] = rows
    doc['executed'] = nm.instructions(visited)
    n = sum(1 for a in visited if 0x5E0690 <= a < 0x5E075C)
    if '--dry' in sys.argv:
        print(len(rows), 'rows;', n, 'instructions of 0x5E0690')
        return
    out = HERE / 'native-route-5e0690.json'
    out.write_text(json.dumps(doc, indent=0))
    print(len(rows), 'rows;', n, 'instructions of 0x5E0690 ->', out)

if __name__ == '__main__':
    main()
