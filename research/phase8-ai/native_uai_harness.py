"""Shared harness for the element (UAI) and aircraft (AAI) Mind-state Think
oracles (native_uai_*.py / native_aai_*.py). Writes no fixture itself.

An emulated Mind holds every embedded state of its type at the real offsets
(inventory.json "mind_states"). All states share one vtable whose +4
(can activate), +C (Think), +18 (enter) and +1C (leave) slots are recorded
stubs; every other state, Mind and object virtual slot is an "unexpected"
stub that aborts the producer, so no unseamed virtual call can pass silently.
Mind Updatable (+8) virtual +4 is the recorded scheduler. The allocator
6F7810/6F7430, the diagnostic loggers 404ED0/404F60/404F90 and the RNG sync
logger 77DA70 are seamed and not recorded. GTIDs resolve through an emulated
registry [B364C0]; the world object [B36534] carries the time (+E0) and the
world RNG seed (+D4). The null point [B279C0]/[B279C4] is a case input.
"""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, from_f32, header, instructions, sha  # noqa: F401
from unicorn import UcError  # noqa: F401
from unicorn.x86_const import UC_X86_REG_EIP

ELEMENT_STATES = {'unload': 0x34, 'deploy': 0x48, 'sleep': 0x68, 'move': 0x7c, 'kill': 0xf8, 'bombard': 0x10c,
                  'engage': 0x12c, 'repair': 0x148, 'join': 0x164, 'reattach': 0x178, 'unpack': 0x188}
AIRCRAFT_STATES = {'move': 0x34, 'guard': 0x9c, 'recon': 0xb4, 'escort': 0xd0, 'attack': 0xe8, 'join': 0x12c,
                   'takeoff': 0x15c, 'landing': 0x174, 'queued': 0x188, 'transport': 0x198}
# Metadata+8 byte (discard on suspend) per state, from inventory.json.
DISCARD = {'deploy', 'sleep'}
f64b = lambda v: struct.unpack('<Q', struct.pack('<d', v))[0]


class Fault(Exception):
    """A native null dereference detected at a seam (the callee would fault)."""


class MindMachine(Machine):
    def __init__(self, c, states, discard=()):
        super().__init__(c['precision'])
        self.c = c
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        for a in (0x404ed0, 0x404f60, 0x404f90):
            self.seam(a, lambda m: self.ret(0))
        self.seam(0x77da70, lambda m: self.ret(0, 12))
        self.reg = self.alloc(0x20004 + 0x10000 * 4 + 16); self.w(0xb364c0, self.reg)
        self.world = self.alloc(0x200); self.w(0xb36534, self.world)
        self.w(self.world + 0xe0, c.get('time', 0)); self.w(self.world + 0xd4, c.get('rng', 0))
        self.w(0xb3655c, self.alloc(0x40))
        point = c.get('null_point', [0, 0])
        self.w(0xb279c0, point[0]); self.w(0xb279c4, point[1])
        self.names = {}
        self.mvt = self.table('mind', 0x200)
        self.mind = self.alloc(0x400); self.w(self.mind, self.mvt)
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule)); self.w(self.mind + 8, uvt)
        self.mind_virtual(0x1a0, 'reset_default'); self.mind_virtual(0x1a4, 'state_changed')
        self.svt = self.table('state', 0x40)
        self.w(self.svt + 4, self.stub(self.v_can_activate)); self.w(self.svt + 0xc, self.stub(self.v_think))
        self.w(self.svt + 0x18, self.stub(self.v_state('enter'))); self.w(self.svt + 0x1c, self.stub(self.v_state('leave')))
        self.state_offsets = dict(states)
        self.states = {}
        for key, off in states.items():
            p = self.mind + off
            meta = self.alloc(0x10); self.w(meta, self.alloc(4)); self.wb(meta + 8, int(key in discard))
            self.w(p, self.svt); self.w(p + 4, self.mind); self.w(p + 8, meta)
            self.w(p + 0xc, c.get('status', {}).get(key, 0))
            self.states[key] = p
        self.state_names = {p: k for k, p in self.states.items()}
        head = tail = 0
        for key in reversed(c.get('stack', [])):
            node = self.alloc(8); self.w(node, 0 if key is None else self.states[key]); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(self.mind + 0x10, head); self.w(self.mind + 0x14, tail)
        for i, word in enumerate(c.get('words', [0, 0, 0, 0])):
            self.w(self.mind + 0x24 + 4 * i, word)
        self.actor = self.obj('actor', c.get('actor_gtid', 0x00010001), registered=False)
        self.w(self.mind + 4, self.actor)

    # objects --------------------------------------------------------------
    def table(self, kind, size):
        t = self.alloc(size)
        for slot in range(0, size, 4):
            self.w(t + slot, self.stub(self.unexpected(kind, slot)))
        return t

    def unexpected(self, kind, slot):
        def fn(m):
            raise AssertionError('unexpected %s virtual +%x' % (kind, slot))
        return fn

    def obj(self, name, gtid=0, registered=True, size=0x800):
        """An object with a vtable of unexpected stubs, a definition (+4)
        whose +8 is a name pointer (read by the diagnostic logs) and GTID +14."""
        p = self.alloc(size)
        self.w(p, self.table(name, 0x200))
        d = self.alloc(0x800); self.w(d + 8, self.alloc(16)); self.w(p + 4, d)
        self.w(p + 0x14, gtid)
        if registered and gtid:
            self.register(gtid, p)
        self.names[p] = name
        return p

    def register(self, gtid, p):
        idx, gen = gtid & 0xffff, gtid >> 16
        self.u.mem_write(self.reg + 4 + 2 * idx, struct.pack('<H', gen)); self.w(self.reg + 0x20004 + 4 * idx, p)

    def vslot(self, p, slot, fn):
        """Install a recorded stub at virtual slot `slot` of object p."""
        self.w(self.r(p) + slot, self.stub(fn))

    def name(self, p):
        if p == 0:
            return 0
        return self.names.get(p, self.state_names.get(p, hex(p)))

    # events ----------------------------------------------------------------
    def ev(self, _event, **kw):
        self.events.append({'event': _event, **kw})

    def mind_virtual(self, slot, name, nargs=0, fmt=None):
        def fn(m):
            assert self.ecx() == self.mind, 'Mind virtual +%x on %x' % (slot, self.ecx())
            args = [self.arg(i + 1) for i in range(nargs)]
            self.ev(name, **({'args': [fmt(a) if fmt else a for a in args]} if nargs else {}))
            self.ret(0, 4 * nargs)
        self.w(self.mvt + slot, self.stub(fn))

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.ev('schedule', delay=self.arg(1), data=self.arg(2)); self.ret(0, 8)

    def v_can_activate(self, m):
        k = self.state_names[self.ecx()]; r = self.c.get('can_activate', True)
        self.ev('can_activate', state=k, result=r); self.ret(int(r))

    def v_think(self, m):
        self.ev('embedded_think', state=self.state_names[self.ecx()]); self.ret(0)

    def v_state(self, name):
        def fn(m):
            self.ev(name, state=self.state_names[self.ecx()]); self.ret(0)
        return fn

    def fault(self, where):
        """The seamed callee would dereference a null pointer here."""
        self.null_accesses.append((where, 0, 4)); self.u.emu_stop()

    def x87(self, value, pop):
        """Return `value` (a Python float, exact binary64) through ST0."""
        scratch = self.alloc(16); self.u.mem_write(scratch, struct.pack('<d', value))
        stub = self.alloc(16); self.u.mem_write(stub, b'\xdd\x05' + struct.pack('<I', scratch) + b'\xc2' + struct.pack('<H', pop))
        self.u.reg_write(UC_X86_REG_EIP, stub)

    # snapshot --------------------------------------------------------------
    def snap(self):
        stack, node = [], self.r(self.mind + 0x10)
        while node:
            stack.append(self.name(self.r(node))); node = self.r(node + 4)
        return {'status': {k: self.r(p + 0xc) for k, p in self.states.items()}, 'stack': stack}

    def run_think(self, entry, this):
        """Run a Think on the embedded state `this`; returns (fault, snapshot)."""
        try:
            self.call(entry, ecx=self.states[this])
        except (AssertionError, UcError) as e:
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        return dict(events=self.events, **self.snap())


def produce(producer, doc, fixture, cases, machine, ranges):
    """Run every case, verify the visited bytes against the v14 image and
    write the fixture; prints case/fault counts and per-function coverage."""
    import json
    out, visited = [], {}
    for c in cases:
        try:
            m = machine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(producer, doc)
    cover = {}
    for name, (lo, hi) in ranges.items():
        cover[name] = [len([a for a in visited if lo <= a < hi]), body_instructions(lo, hi)]
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / fixture
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))
    return out, visited


def body_instructions(lo, hi):
    """Count of instructions of the original function body [lo, hi)."""
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    from native_machine import original
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    return len(list(md.disasm(original.range(lo, hi - lo), lo)))
