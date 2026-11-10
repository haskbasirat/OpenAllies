"""Native oracle for the Performer's leave virtual 0x57DCB0 (another
component takes over) and the Mover activation 0x576790.

Original code runs, including 0x529C30. Recorded seams: 0x50B250 (cancel
the Updatable's events, on component +8), 0x50AC00 (activate, on +8), the
owner's Mind (+70) virtual +13C and 0x42A590 (the ActorData +24C trigger
list, arguments recorded).

Usage: native_performer_interrupt.py [AA.exe] (checked against the accepted SHA-256).
"""
from pathlib import Path
import hashlib
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

if len(sys.argv) > 1:
    assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() == h.ORIGINAL_SHA, sys.argv[1]


def main():
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    events = []
    state = {}

    def cancel(mm):
        events.append({'cancel': mm.reg(UC_X86_REG_ECX) - state['component']})
        mm.ret(0)

    def activate(mm):
        events.append({'activate': mm.reg(UC_X86_REG_ECX) - state['component'], 'arg': mm.arg(0)})
        mm.ret(4)

    def interrupted(mm):
        assert mm.reg(UC_X86_REG_ECX) == state['mind']
        events.append({'mind_interrupted': 1})
        mm.ret(0)

    def triggers(mm):
        assert mm.reg(UC_X86_REG_ECX) == state['data'] + 0x24C
        events.append({'triggers': 'owner' if mm.arg(0) == state['owner'] else mm.arg(0), 'arg': mm.arg(1)})
        mm.ret(8)
    m.hooks.update({0x50B250: cancel, 0x50AC00: activate, 0x42A590: triggers})
    mind_vt = m.vtable({0x13C: interrupted})
    cases = []
    for stage in (0, 1, 2, 3, 4, 5, -1):
        for latch in (0, 1, 2):
            events.clear()
            comp = m.alloc(0x80)
            owner = m.alloc(0x100)
            mind = m.alloc(0x20)
            m.w(mind, mind_vt)
            m.w(owner + 0x70, mind)
            m.w(comp + 4, owner)
            m.w(comp + 0x10, stage & 0xFFFFFFFF)
            m.wb(comp + 0x5C, latch)
            state.update(component=comp, mind=mind, owner=owner)
            m.call(0x57DCB0, ecx=comp, args=(0x12345678,), pop=4)
            cases.append({'function': 'performer_left', 'stage': stage, 'latch': latch, 'events': list(events)})
    for flag in (0, 1, 2, 0x100):
        events.clear()
        comp = m.alloc(0x80)
        owner = m.alloc(0x300)
        data = m.alloc(0x300)
        m.w(comp + 4, owner)
        m.w(owner + 4, data)
        state.update(component=comp, owner=owner, data=data)
        m.call(0x576790, ecx=comp, args=(flag,), pop=4)
        cases.append({'function': 'mover_activation', 'flag': flag, 'events': list(events)})
    h.report('native-performer-interrupt-fixtures.json',
             '0x57DCB0 Performer leave and 0x576790 Mover activation (50B250/50AC00/Mind +13C/42A590 recorded).',
             cases, m.visited)


if __name__ == '__main__':
    main()
