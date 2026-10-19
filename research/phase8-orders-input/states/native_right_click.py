"""Original AA.exe right-click order of the default game UI state.

Runs, in both pinned images (original AA.exe and the v14 runtime), the
ORIGINAL 0x620760 (GameInterface+0xD8 vt+0x14) together with the original
command constructor 0x4D9C80, 0x4D9B30 (Alt), 0x4D9BF0 (target), the
KKC_TellSelectedCommandOrder move constructor 0x60C8A0 / destructor
0x61E4B0, the command's deleting destructor 0x4D9CC0 -> 0x4CA2D0, 0x45B7A0
and the VWorld vt+0x90 0x6A71F0. Seams (each records its call):
  0x6F7810 / 0x6F7430   allocator
  0x5CD190              base state input (the event is not a right click on
                        the world/map, or nothing is selected)
  0x4363B0              UIWorldState hovered target (scripted view)
  0x5C0E10              local player (constant)
  0x61E150              KKC_TellSelectedCommandOrder::CanSend (scripted)
  0x61EEF0              KKC_TellSelectedCommandOrder::Send
  0x459600              target flash
  0x5D74B0              minimap world XY under the Input cursor (scripted)
  0x43DB10              destination marker
  GameInterface+0xE4 vt+0x58   facing state begin (0x621230)
  actor vt+0xB4 / vt+0x34, widget vt+0x90 (minimap true, HUD false)
Writes native-right-click.json: per case the inputs and the call trace with
the command words (+0x0C flags, +0x10 word, +0x18 position, +0x20
auxiliary, +0x28 scalar, +0x3C trailing byte) at each order call.
"""
from pathlib import Path
import sys, json, random
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine, f32

PLAYER = 0x2468ACE0
rng = random.Random(0x620760)

def r32(lo, hi):
    return f32(rng.uniform(lo, hi))

class Case:
    def __init__(self, spec):
        self.spec = spec
        self.trace = []
        self.visited = {}
        m = self.m = Machine('Double53')
        m.seam(0x6F7810, lambda m: m.ret(m.alloc(m.arg(1)), 0))
        m.seam(0x6F7430, self.free)
        m.seam(0x5CD190, self.base)
        m.seam(0x4363B0, self.hovered)
        m.seam(0x5C0E10, lambda m: m.ret(PLAYER, 0))
        m.seam(0x61E150, self.can_send)
        m.seam(0x61EEF0, self.send)
        m.seam(0x459600, self.flash)
        m.seam(0x5D74B0, self.map_world)
        m.seam(0x43DB10, self.marker)
        s = spec
        # UI manager and widgets
        ui = m.alloc(0x100)
        m.w(0xB3653C, ui)
        vworld = self.vworld = m.alloc(0x200)
        vvt = m.alloc(0x100)
        m.w(vvt + 0x90, 0x6A71F0)
        m.w(vworld, vvt)
        m.w(0xB36538, vworld)
        for i, w in enumerate(s['pointer_184']):
            m.w(vworld + 0x184 + 4 * i, w)
        uiw = m.alloc(0x60)
        m.w(vworld + 0x13C, uiw)
        widget = 0
        if s['widget'] == 'world':
            widget = vworld
        elif s['widget'] in ('map', 'other'):
            widget = self.map_widget = m.alloc(0x40)
            wvt = m.alloc(0x100)
            on = 1 if s['widget'] == 'map' else 0
            m.w(wvt + 0x90, m.stub(lambda m, on=on: m.ret(on, 0)))
            m.w(widget, wvt)
        m.w(ui + 0x60, widget)
        # input
        inp = m.alloc(0x40)
        m.w(inp + 0x28, s['modifiers'])
        m.w(inp + 0xC, s['cursor'][0])
        m.w(inp + 0x10, s['cursor'][1])
        m.w(0xB39B18, inp)
        m.w(0xB279C0, s['aux'][0])
        m.w(0xB279C4, s['aux'][1])
        # views and actors
        self.views = []
        self.view_index = {}
        for i, v in enumerate(s['views']):
            view = m.alloc(0x40)
            self.views.append(view)
            self.view_index[view] = i
            if v['actor'] is None:
                continue
            a = v['actor']
            actor = m.alloc(0x40)
            avt = m.alloc(0x200)
            m.w(avt + 0xB4, m.stub(lambda m, on=a['b4']: m.ret(1 if on else 0, 0)))
            m.w(avt + 0x34, m.stub(lambda m, can=a['can']: m.ret(1 if m.arg(1) in can else 0, 4)))
            m.w(actor, avt)
            m.w(actor + 0x14, a['handle'])
            m.w(view + 0x14, actor)
        head = 0
        for i in reversed(s['selected']):
            node = m.alloc(8)
            m.w(node, self.views[i])
            m.w(node + 4, head)
            head = node
        m.w(uiw + 8, head)
        # command definitions [[B36530]+0x298][i]
        sim = m.alloc(0x300)
        table = m.alloc(4 * 0x27)
        for i, d in enumerate(s['definitions']):
            if d is None:
                m.w(table + 4 * i, 0)
                continue
            p = m.alloc(0x50)
            m.w(p + 0x2C, d['flags_2c'])
            m.wb(p + 0x40, 1 if d['byte_40'] else 0)
            m.w(table + 4 * i, p)
        m.w(sim + 0x298, table)
        m.w(0xB36530, sim)
        # game interface facing state
        gi = m.alloc(0x100)
        facing = m.alloc(0x40)
        fvt = m.alloc(0x100)
        m.w(fvt + 0x58, m.stub(self.facing_begin))
        m.w(facing, fvt)
        m.w(gi + 0xE4, facing)
        m.w(0xB3974C, gi)
        # event
        ev = m.alloc(0x30)
        m.w(ev + 4, s['event']['flags'])
        m.u.mem_write(ev + 8, s['event']['code'].to_bytes(2, 'little'))
        self.state = m.alloc(0x40)
        self.al = m.call(0x620760, ecx=self.state, args=(ev,), pop=4) & 0xff
        m.verify_v14()

    def command(self, cmd):
        m = self.m
        return {'flags': m.r(cmd + 0xC), 'word': m.r(cmd + 0x10), 'position': [m.r(cmd + 0x18), m.r(cmd + 0x1C)],
                'auxiliary': [m.r(cmd + 0x20), m.r(cmd + 0x24)], 'scalar': m.r(cmd + 0x28), 'trailing': m.rb(cmd + 0x3C),
                'command_08': m.r(cmd + 8), 'object_14': m.r(cmd + 0x14)}

    def free(self, m):
        self.trace.append(['free'])
        m.ret(0, 0)

    def base(self, m):
        assert m.ecx() == self.state
        self.trace.append(['base'])
        m.ret(1, 4)

    def hovered(self, m):
        t = self.spec['target']
        self.trace.append(['hovered'])
        m.ret(0 if t is None else self.views[t], 0)

    def can_send(self, m):
        assert m.arg(1) == PLAYER
        self.trace.append(['can_send', self.command(m.r(m.ecx() + 4))])
        m.ret(1 if self.spec['can_send'] else 0, 4)

    def send(self, m):
        assert m.arg(1) == PLAYER
        self.trace.append(['send', self.command(m.r(m.ecx() + 4))])
        m.ret(0, 4)

    def flash(self, m):
        self.trace.append(['flash', self.view_index[m.ecx()]])
        m.ret(0, 0)

    def map_world(self, m):
        assert m.ecx() == self.map_widget
        out = m.arg(1)
        self.trace.append(['map_world', [m.arg(2), m.arg(3)]])
        m.w(out, self.spec['map_xy'][0])
        m.w(out + 4, self.spec['map_xy'][1])
        m.ret(out, 0xC)

    def marker(self, m):
        assert m.ecx() == self.vworld
        self.trace.append(['marker', [m.arg(1), m.arg(2)]])
        m.ret(0, 8)

    def facing_begin(self, m):
        self.trace.append(['facing', self.command(m.r(m.arg(1))), m.arg(2)])
        m.ret(0, 8)

def gen():
    cases = []
    for k in range(400):
        nviews = rng.randint(0, 5) if k % 10 == 0 else rng.randint(1, 5)
        views = []
        for i in range(nviews):
            if rng.random() < 0.15:
                views.append({'actor': None})
                continue
            can = sorted(rng.sample(range(0x27), rng.randint(0, 6)))
            views.append({'actor': {'b4': rng.random() < 0.85, 'can': can, 'handle': rng.randrange(1, 1 << 32)}})
        selected = rng.sample(range(nviews), rng.randint(0 if k % 8 == 0 else 1, nviews)) if nviews else []
        definitions = []
        for i in range(0x27):
            r = rng.random()
            if r < 0.15:
                definitions.append(None)
            else:
                flags = rng.randrange(0, 1 << 16)
                if rng.random() < 0.7:
                    flags &= ~0x20
                definitions.append({'flags_2c': flags, 'byte_40': rng.random() < 0.5})
        widget = rng.choice(['world', 'world', 'world', 'map', 'other', 'none'])
        event = rng.choice([{'code': 0x101, 'flags': 0x81}] * 14 + [{'code': 0x101, 'flags': 0x82}, {'code': 0x100, 'flags': 0x81},
                            {'code': 0x109, 'flags': 0x81}, {'code': 0x101, 'flags': 0xA1}, {'code': 0x101, 'flags': 0xC1}])
        target = None
        if nviews and rng.random() < 0.5:
            target = rng.randrange(nviews)
        cases.append({
            'views': views, 'selected': selected, 'definitions': definitions, 'widget': widget, 'event': event,
            'target': target, 'modifiers': rng.randrange(0, 8), 'can_send': rng.random() < 0.7,
            'pointer_184': [r32(-2000, 2000), r32(-2000, 2000), r32(-10, 80)],
            'cursor': [rng.randrange(0, 1024), rng.randrange(0, 768)],
            'map_xy': [r32(0, 4000), r32(0, 4000)], 'aux': [r32(-1, 1), r32(-1, 1)],
        })
    return cases

def main():
    rows = []
    visited = {}
    for spec in gen():
        c = Case(spec)
        visited.update(c.m.visited)
        rows.append({'args': spec, 'native': {'al': c.al, 'trace': c.trace}})
    doc = nm.header(__file__, 'Unicorn 2.1.4, original and v14 images (PC53; 0x620760 has no x87 arithmetic)')
    doc['rows'] = rows
    doc['executed'] = nm.instructions(visited)
    if '--dry' in sys.argv:
        from collections import Counter
        print(len(rows), 'rows', len(visited), 'instructions,', sum(1 for a in visited if 0x620760 <= a < 0x620AFD), 'in 0x620760')
        print(Counter(' '.join(t[0] for t in r['native']['trace']) for r in rows))
        return
    out = HERE / 'native-right-click.json'
    out.write_text(json.dumps(doc, indent=0))
    print(len(rows), 'rows', len(visited), 'instructions ->', out)

if __name__ == '__main__':
    main()
