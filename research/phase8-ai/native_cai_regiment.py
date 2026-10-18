"""Shared Unicorn harness for the regiment (CAI) Mind-state Think oracles
(native_cai_<name>.py). Original AA.exe code only; nothing here writes game
files.

An emulated regiment Mind holds every embedded state at its real offset
(research/phase8-ai/inventory.json "mind_states"), each with a recording
vtable (+4 can activate, +C think, +18 enter, +1C leave; any other slot is an
assertion), its +4 Mind pointer and a +8 metadata block (+0 name pointer, +8
discard-on-suspend byte from the inventory). The Mind has Updatable +8
(virtual +4 = schedule), +4 organization (vtable of recording or unexpected
stubs, +4 definition with a +8 name pointer for the diagnostic logs, +14 GTID,
+7C organization component), the +10/+14 state stack and words +18..+30.
Objects live in an emulated GTID registry [B364C0].

Always seamed: allocator 6F7810 (bump heap), free 6F7430, loggers 404ED0
(recorded as `diagnostic_log` only when called from 529E60, which proves the
base Think's wrong-activity flag), 404F60 and 404F90 (not recorded).
"""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine

STATES = {'sleep': 0x34, 'rampage': 0x54, 'move': 0x68, 'kill': 0xc0, 'bombard': 0xd4, 'siege': 0xf8,
          'chasing': 0x120, 'construct': 0x134, 'repair': 0x160, 'guard': 0x17c, 'regroup': 0x1a8,
          'reform': 0x1b8, 'engage': 0x1cc, 'retreat': 0x1f0, 'rout': 0x20c, 'explore': 0x220,
          'join': 0x24c, 'exhausted': 0x270, 'unpack': 0x280, 'pack': 0x2a4, 'other': 0x700}
DISCARD = {'sleep', 'rampage'}
MIND_SIZE = 0x800


class RegimentMachine(Machine):
    """`c` must hold: precision, states {name: status}, stack [name|None],
    words [w24, w28, w2C, w30]; optional think_effects {thinking state: {state:
    status}} applied after a recorded embedded Think; optional leave_clears
    (a recorded leave sets the leaving state's status to 0). `tracked` lists the state names snapshotted
    in every event (default: those in c['states'])."""

    def __init__(self, c, mind_virtuals=(), org_virtuals=()):
        super().__init__(c['precision'])
        self.c = c
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x404ed0, self.s_log)
        self.seam(0x404f60, lambda m: self.ret(0))
        self.seam(0x404f90, lambda m: self.ret(0))
        reg = self.reg = self.alloc(0x20004 + 0x10000 * 4 + 16)
        self.w(0xb364c0, reg)
        mvt = self.alloc(0x400)
        for slot in range(0, 0x400, 4):
            self.w(mvt + slot, self.stub(self.unexpected('mind', slot)))
        self.w(mvt + 0x1a0, self.stub(self.v_mind('reset_default', 0)))
        self.w(mvt + 0x1a4, self.stub(self.v_mind('state_changed', 0)))
        for slot, name, nargs in mind_virtuals:
            self.w(mvt + slot, self.stub(self.v_mind(name, nargs)))
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule))
        mind = self.mind = self.alloc(MIND_SIZE)
        self.w(mind, mvt); self.w(mind + 8, uvt)
        org = self.org = self.alloc(0x400)
        ovt = self.alloc(0x400)
        for slot in range(0, 0x400, 4):
            self.w(ovt + slot, self.stub(self.unexpected('organization', slot)))
        for slot, fn in org_virtuals:
            self.w(ovt + slot, self.stub(fn))
        self.w(org, ovt)
        defn = self.org_defn = self.alloc(0x400); self.w(defn + 8, self.alloc(4)); self.w(org + 4, defn)
        self.w(org + 0x14, 0x00010001)
        self.comp = self.alloc(0x200); self.w(org + 0x7c, self.comp)
        self.w(mind + 4, org)
        svt = self.alloc(0x40)
        for slot in range(0, 0x40, 4):
            self.w(svt + slot, self.stub(self.unexpected('state', slot)))
        self.w(svt + 4, self.stub(self.v_can_activate)); self.w(svt + 0xc, self.stub(self.v_think))
        self.w(svt + 0x18, self.stub(self.v_state('enter'))); self.w(svt + 0x1c, self.stub(self.v_state('leave')))
        self.states = {}
        for key, off in STATES.items():
            p = mind + off
            meta = self.alloc(0x10); self.w(meta, self.alloc(4)); self.wb(meta + 8, int(key in DISCARD))
            self.w(p, svt); self.w(p + 4, mind); self.w(p + 8, meta)
            self.states[key] = p
        for key, st in c['states'].items():
            self.w(self.states[key] + 0xc, st)
        self.state_names = {p: k for k, p in self.states.items()}
        self.tracked = c.get('tracked', list(c['states']))
        head = tail = 0
        for key in reversed(c['stack']):
            node = self.alloc(8); self.w(node, 0 if key is None else self.states[key]); self.w(node + 4, head)
            head = node
            if not tail:
                tail = node
        self.w(mind + 0x10, head); self.w(mind + 0x14, tail)
        for i, word in enumerate(c['words']):
            self.w(mind + 0x24 + 4 * i, word)
        self.objects, self.names = {}, {}

    # objects ------------------------------------------------------------
    def add_object(self, key, gtid, registered, vtable_slots=(), size=0x400):
        """An object with a +0 vtable (unexpected stubs except vtable_slots
        [(slot, fn)]), +4 definition (+8 name), +14 GTID; registered objects
        are entered in the GTID registry."""
        p = self.alloc(size)
        vt = self.alloc(0x400)
        for slot in range(0, 0x400, 4):
            self.w(vt + slot, self.stub(self.unexpected('object', slot)))
        for slot, fn in vtable_slots:
            self.w(vt + slot, self.stub(fn))
        self.w(p, vt)
        defn = self.alloc(0x40); self.w(defn + 8, self.alloc(4)); self.w(p + 4, defn)
        self.w(p + 0x14, gtid)
        if registered:
            idx, gen = gtid & 0xffff, gtid >> 16
            self.u.mem_write(self.reg + 4 + 2 * idx, struct.pack('<H', gen))
            self.w(self.reg + 0x20004 + 4 * idx, p)
        self.objects[key] = p
        self.names[p] = key
        return p

    def name(self, p):
        if p == 0:
            return 0
        if p in self.names:
            return self.names[p]
        if p in self.state_names:
            return self.state_names[p]
        raise AssertionError('unknown pointer %x' % p)

    # recording ----------------------------------------------------------
    def snap(self):
        stack, node = [], self.r(self.mind + 0x10)
        while node:
            s = self.r(node)
            stack.append(None if s == 0 else self.state_names[s]); node = self.r(node + 4)
        return {'states': {k: self.r(self.states[k] + 0xc) for k in self.tracked}, 'stack': stack}

    def ev(self, name, **kw):
        self.events.append({'event': name, **kw, **self.snap()})

    def unexpected(self, kind, slot):
        def fn(m):
            raise AssertionError('unexpected %s virtual +%x' % (kind, slot))
        return fn

    def s_log(self, m):
        if self.r(self.u.reg_read(__import__('unicorn').x86_const.UC_X86_REG_ESP)) == 0x529eaa:
            self.events.append({'event': 'diagnostic_log'})
        self.ret(0)

    def v_mind(self, name, nargs):
        def fn(m):
            assert self.ecx() == self.mind, name
            args = [self.name_or_word(self.arg(i + 1)) for i in range(nargs)]
            self.ev(name, args=args); self.ret(0, 4 * nargs)
        return fn

    def name_or_word(self, v):
        if v in self.names:
            return self.names[v]
        if v in self.state_names:
            return self.state_names[v]
        if v == self.org:
            return 'organization'
        return v

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.ev('schedule', delay=self.arg(1), data=self.arg(2)); self.ret(0, 8)

    def v_can_activate(self, m):
        s = self.state_names[self.ecx()]; r = self.c['can_activate'][s]
        self.ev('can_activate', state=s, result=r); self.ret(int(r))

    def v_think(self, m):
        s = self.state_names[self.ecx()]
        self.ev('embedded_think', state=s)
        # Optional case-modelled callback effect: the thinking state changes
        # other states' statuses (e.g. the sleep Think activating kill).
        for key, status in self.c.get('think_effects', {}).get(s, {}).items():
            self.w(self.states[key] + 0xc, status)
        self.ret(0)

    def v_state(self, name):
        def fn(m):
            s = self.state_names[self.ecx()]
            self.ev(name, state=s)
            # Optional case-modelled exit: the real OnLeave functions (for
            # example 51D7B0, 4BDDA0) clear the leaving state's status.
            if name == 'leave' and self.c.get('leave_clears'):
                self.w(self.ecx() + 0xc, 0)
            self.ret(0)
        return fn

    # running ------------------------------------------------------------
    def run_think(self, entry, state):
        from unicorn import UcError
        try:
            self.call(entry, ecx=self.states[state])
        except (AssertionError, UcError) as e:
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        return None


def coverage(visited, ranges):
    """Visited instruction counts inside each named [start, end) range against
    the capstone instruction count of that range."""
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    from native_machine import original
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    out = {}
    for name, (start, end) in ranges.items():
        total = [i.address for i in md.disasm(original.range(start, end - start), start)
                 if not (i.mnemonic == 'int3')]
        hit = [a for a in total if a in visited]
        out[name] = {'range': [hex(start), hex(end)], 'instructions': len(total), 'visited': len(hit),
                     'unvisited': [hex(a) for a in total if a not in visited]}
    return out


# Unit orders and player error messages (pack/unpack) ------------------------
class UnitOrderMixin:
    """Stack actions: 4D9E20 (base constructor) runs natively with the
    action registry [B36530]+298 (as do the setters
    4D9BF0, 4D9B10 and the move constructor 4CA760 when a Think calls them);
    4D9B60(unit) is seamed and records the action fields (+0 vtable, +8
    definition, +C flags, +10 word, +18..+28 words, +30 byte); the destructor
    4CA2D0 is seamed and recorded (its +34 reference is null: asserted). Error messages: 401B00 (CString from a wide literal),
    organization virtual +24 (name into a CString), 4B9F40 (display string
    lookup on [B36544]), 401560 (CString copy), organization virtual +108
    (owner) and 40C230 (cdecl message post, which releases its format
    CString) are seamed with refcounted fake CStrings; the Think's own
    releases call the fake string manager's virtual +4 (not recorded)."""

    def install_unit_orders(self, definitions, issue_results, owner):
        self.issue_results = list(issue_results)
        reg = self.alloc(0x400); table = self.alloc(0x100); self.w(reg + 0x298, table); self.w(0xb36530, reg)
        for ordinal, value in definitions.items():
            self.w(table + 4 * int(ordinal), value)
        self.seam(0x4d9b60, self.s_issue_action)
        self.seam(0x4ca2d0, self.s_release_action)
        mgr = self.alloc(0x10); mvt = self.alloc(0x10); self.w(mgr, mvt)
        self.w(mvt + 4, self.stub(lambda m: self.ret(0, 4)))
        self.string_mgr = mgr
        self.strings = {}
        self.owner = owner
        self.seam(0x401b00, self.s_cstring_literal)
        self.seam(0x4b9f40, self.s_display_string)
        self.seam(0x401560, self.s_cstring_copy)
        self.seam(0x40c230, self.s_post_message)
        self.w(0xb36544, 0x0bad5440)

    def org_unit_virtuals(self):
        return [(0x24, self.o_name), (0x108, self.o_owner)]

    def cstring(self, text, refs=1):
        hdr = self.alloc(0x20)
        self.w(hdr, self.string_mgr); self.w(hdr + 0xc, refs)
        self.strings[hdr + 0x10] = text
        return hdr + 0x10

    def s_cstring_literal(self, m):
        raw = bytes(self.u.mem_read(self.arg(1), 128))
        text = raw.decode('utf-16le').split('\x00')[0]
        self.w(self.ecx(), self.cstring(text)); self.ret(self.ecx(), 4)

    def o_name(self, m):
        assert self.ecx() == self.org
        dst = self.arg(1)
        self.w(dst, self.cstring('organization name'))
        self.ev('organization_name'); self.ret(dst, 4)

    def s_display_string(self, m):
        assert self.ecx() == 0x0bad5440 and self.arg(2) == 0xb43b48
        key = self.strings[self.r(self.arg(1))]
        entry = self.alloc(8); self.w(entry, self.cstring('display:' + key, refs=5))
        self.ev('display_string', key=key); self.ret(entry, 8)

    def s_cstring_copy(self, m):
        src = self.r(self.arg(1))
        self.w(src - 4, self.r(src - 4) + 1)
        self.w(self.ecx(), src); self.ret(self.ecx(), 4)

    def o_owner(self, m):
        assert self.ecx() == self.org
        self.ev('organization_owner', result=self.owner); self.ret(self.owner)

    def s_post_message(self, m):
        assert self.arg(1) == 0xb399b0 and self.arg(3) == self.org
        fmt = self.arg(6)
        self.ev('post_message', kind=self.arg(2), owner=self.arg(4), word5=self.arg(5),
                format=self.strings[fmt], argument=self.strings[self.arg(7)])
        self.w(fmt - 4, self.r(fmt - 4) - 1)
        self.ret(0)

    def s_release_action(self, m):
        a = self.ecx()
        assert self.r(a + 0x34) == 0
        self.ev('release_action', definition=self.r(a + 8), flags=self.r(a + 0xc))
        self.ret(0)

    def s_issue_action(self, m):
        a = self.ecx()
        r = self.issue_results.pop(0)
        self.ev('issue_action', unit=self.name(self.arg(1)), vtable=self.r(a), definition=self.r(a + 8),
                flags=self.r(a + 0xc), word10=self.r(a + 0x10), words=[self.r(a + 0x18 + 4 * i) for i in range(5)],
                byte30=self.rb(a + 0x30), result=r)
        self.ret(int(r), 4)
