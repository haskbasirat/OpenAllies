"""Execute the original GWorld removal request 0x4F6760 and its immediate paths.

Executed original code: 0x4F6760 request/dedup, 0x542A00 list prepend,
0x4F53D0 immediate removal, 0x4F5490 immediate destroy, 0x4F5370
availability+classification, 0x4F4C40 availability leaf, 0x4F1430 list-value
removal, 0x50C1B0 registry unlink (empty registry), and for the drain cases the
phase transition 0x4F65B0. Reuses the phase2 world harness (Machine) and its
actor-virtual tracer seams; adds virtual +0x2C (argument recorded), virtual
+0xB4 (scripted; traced only when called from 0x4F5490 - inside 0x4F4C40 it
returns the same value with actor+0xD4 == 0 so the leaf returns early) and
the GTID table 0xB364C0 used by 0x4F5490 to resolve actor+0x158.
"""
from pathlib import Path
import hashlib, json, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase2-world-order'))
import trace_world_dispatch as T
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX

RANGES = {'request_removal': (0x4F6760, 0x4F67F0), 'list_prepend': (0x542A00, 0x542A2A),
          'remove_now': (0x4F53D0, 0x4F5482), 'destroy_now': (0x4F5490, 0x4F5560),
          'availability_and_classification': (0x4F5370, 0x4F53CC)}


class M(T.Machine):
    def __init__(self, phase, mode=0):
        super().__init__(7, phase)
        self.session = self.alloc(0x200)
        self.write(0xB36550, self.session)
        self.write(self.session + 0xE4, mode)
        self.write(self.actor_vtable + 0x2C, 0x21F0000 + 0x2C)
        self.gtid = self.alloc(0x20004 + 4 * 64)
        self.write(0xB364C0, self.gtid)
        self.b4 = {}

    def handle(self, actor, index, generation, target=None, table_generation=None):
        self.write(actor + 0x158, (generation << 16) | index)
        tg = generation if table_generation is None else table_generation
        self.u.mem_write(self.gtid + 4 + index * 2, (tg & 0xFFFF).to_bytes(2, 'little'))
        self.write(self.gtid + 0x20004 + index * 4, target or 0)

    def hook(self, u, pc, size, user):
        if pc == 0x4F6760:
            self.event('request_removal', name=self.names[self.arg(0)], destroy=self.arg(1) & 0xFF)
            return
        if 0x21F0000 <= pc < 0x21F0100:
            slot = pc - 0x21F0000
            ecx = u.reg_read(UC_X86_REG_ECX)
            if slot == 0xB4:
                value = self.b4.get(ecx, False)
                caller = self.read(u.reg_read(T.UC_X86_REG_ESP))
                if 0x4F5490 <= caller < 0x4F5560:
                    self.event('actor_b4', name=self.names[ecx], result=value)
                self.ret(value=int(value))
                return
            if slot == 0x2C:
                self.event('actor_2c', name=self.names[ecx], argument=self.arg())
                self.ret(4)
                return
        super().hook(u, pc, size, user)

    def request(self, actor, destroy):
        self.run(0x4F6760, (actor, int(destroy)))
        return self.u.reg_read(UC_X86_REG_EAX) & 0xFF


def finish(m, name, results, setup):
    lists = {hex(off): m.list_names(off) for off in [0x100, 0x108, 0x110, 0x178, 0x188, 0x1A4]}
    events = [e for e in m.events if e['event'] != 'free_list_node']
    return {'name': name, 'setup': setup, 'results': results, 'events': events,
            'final': {'phase': m.read(m.world + 0xD8), 'lists': lists,
                      'actor_flags': {m.names[a]: m.read(a + 8) for a in m.actors}}}


def cases():
    out = []
    # Deferred: phase 1 tick, phase 4/5/0 with non-editor session.
    for phase in (1, 4, 5, 0):
        m = M(phase, mode=0)
        a = m.actor('A', flags=0x10)
        b = m.actor('B', flags=0x20)
        r = [m.request(a, False), m.request(b, True), m.request(a, True), m.request(b, False)]
        out.append(finish(m, f'deferred_prepend_dedup_phase{phase}', r,
                          {'phase': phase, 'mode': 0, 'requests': [['A', 0], ['B', 1], ['A', 1], ['B', 0]]}))
    # Deferred requests then the phase transition drains newest first.
    m = M(1, mode=0)
    a = m.actor('A', retain=(False,), classification=0xBC)
    b = m.actor('B', retain=(True,), classification=0xC0, flags=0x4)
    c = m.actor('C')
    m.append(0x188, a); m.append(0x178, b)
    r = [m.request(a, False), m.request(b, False), m.request(c, True)]
    m.run(0x4F65B0, (4,))
    out.append(finish(m, 'deferred_lifo_then_transition_drain', r,
                      {'phase': 1, 'mode': 0, 'requests': [['A', 0], ['B', 0], ['C', 1]], 'then_transition': 4}))
    # Immediate removal: phase 0 in session mode 3, phase 3 and phase 6.
    for phase, mode in ((0, 3), (3, 0), (6, 0), (3, 3)):
        m = M(phase, mode=mode)
        a = m.actor('A', retain=(True,), classification=0xBC, flags=0x8)
        b = m.actor('B', retain=(False,), classification=0xC0)
        c = m.actor('C', retain=(False,))
        m.append(0x188, a); m.append(0x178, b)
        r = [m.request(a, False), m.request(b, False), m.request(c, False)]
        out.append(finish(m, f'immediate_remove_phase{phase}_mode{mode}', r,
                          {'phase': phase, 'mode': mode, 'requests': [['A', 0], ['B', 0], ['C', 0]]}))
    # Immediate destroy (0x4F5490): B4 false, B4 true with live handle, stale handle, empty slot.
    m = M(3, mode=0)
    a = m.actor('A', classification=0xBC, flags=0x1)
    b = m.actor('B')
    x = m.actor('X', classification=0xC0, flags=0x40)
    c = m.actor('C')
    d = m.actor('D')
    m.append(0x188, a); m.append(0x178, x)
    m.b4[b] = True; m.handle(b, 5, 0x1234, target=x)
    m.b4[c] = True; m.handle(c, 6, 0x0007, target=x, table_generation=0x0008)
    m.b4[d] = True; m.handle(d, 7, 0x0002, target=0)
    r = [m.request(a, True), m.request(b, True), m.request(c, True), m.request(d, True)]
    out.append(finish(m, 'immediate_destroy_paths', r, {
        'phase': 3, 'mode': 0, 'requests': [['A', 1], ['B', 1], ['C', 1], ['D', 1]],
        'b4': ['B', 'C', 'D'], 'handles': {'B': 'X', 'C': None, 'D': None}}))
    # Already queued object is not processed immediately even in phase 3.
    m = M(1, mode=0)
    a = m.actor('A')
    r = [m.request(a, False)]
    m.write(m.world + 0xD8, 3)
    r.append(m.request(a, True))
    out.append(finish(m, 'queued_object_ignored_in_immediate_phase', r,
                      {'phase': 1, 'mode': 0, 'requests': [['A', 0], ['set_phase', 3], ['A', 1]]}))
    return out


def main():
    ranges = {}
    for name, (a, b) in RANGES.items():
        data = T.NATIVE.range(a, b - a)
        assert data == T.ACCEPTED.range(a, b - a), name
        ranges[name] = {'start': hex(a), 'end_exclusive': hex(b), 'sha256': T.digest(data), 'unchanged_in_v14': True}
    rows = cases()
    report = {'schema': 1, 'passed': True, 'source_sha256': T.digest(T.NATIVE.b),
              'producer_sha256': T.digest(Path(__file__).read_bytes()), 'method': __doc__,
              'fpu_control_word': '0x037f', 'ranges': ranges, 'cases': rows}
    target = HERE / 'native-removal-request.json'
    target.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'cases': len(rows), 'sha256': T.digest(target.read_bytes())}))


if __name__ == '__main__':
    main()
