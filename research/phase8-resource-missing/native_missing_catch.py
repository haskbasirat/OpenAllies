"""Unicorn oracle: the two caught missing-source paths of the Resource factories.

Original AA.exe code only (bytes checked against the accepted v14 image):
  5A80D0 KKC_NiSeqResource::GetNiSeqResource   try{5A7750(name,1)} catch(TGC_MissingResourceException&)
  59B4B0 Sound factory                           try{59B1F0(name,1)} catch(TGC_MissingResourceException&)
  59B280 KKC_AudioResource::GetDefaultResource   (reached from the 59B4B0 handler)

Static part (from the image): FuncInfo/try maps/handler types, the ThrowInfo of
the 425210 "Unable to find" throw (TGC_MissingResourceException) and of the
4252A0 "Not a" throw (TGC_CorruptResourceException), and the strings.

Executed part: each function runs under Unicorn with explicit seams:
  5A7750 / 59B1F0  -> return a resource pointer, or raise the C++ exception
  405D20 / 405370  -> current DataMissingResourceWarning cell (B3994C table)
  404FC0           -> recorded log call
  410CB0           -> current AVar string (a real refcounted CString layout)
  96BEF5           -> _CxxThrowException(NULL,NULL) = rethrow, recorded
  [B39940]+F4      -> pointer to the UVar61 DataCompatibilityMode byte
The CRT catch dispatch is reproduced for this single frame exactly as MSVC
does it: the catch funclet is called with the frame's EBP, its returned
continuation runs with ESP = [EBP-10] (saved by `mov [ebp-10h],esp`).
The type match is checked statically: the thrown CatchableType list contains
the handler's TypeDescriptor B28008. Nothing here writes game files.
"""
from pathlib import Path
import json, struct, sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
import native_machine as nm
from native_machine import Machine, original, modified, header, instructions
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_ESP, UC_X86_REG_EIP

MISSING_TYPE = 0xB28008  # .?AVTGC_MissingResourceException@@


def u32(pe, a):
    return struct.unpack('<I', pe.range(a, 4))[0]


def cstr(pe, a, n=96):
    return pe.range(a, n).split(b'\0')[0].decode('ascii')


def wstr(pe, a, n=400):
    return pe.range(a, n).decode('utf-16le', 'replace').split('\0')[0]


def type_name(pe, td):
    return cstr(pe, td + 8)


def funcinfo(pe, fi):
    magic, max_state, unwind, ntry, ptry = struct.unpack('<IIIII', pe.range(fi, 20))
    tries = []
    for t in range(ntry):
        low, high, catch_high, ncatch, handlers = struct.unpack('<iiiII', pe.range(ptry + 20 * t, 20))
        catches = []
        for c in range(ncatch):
            adj, td, disp, addr = struct.unpack('<IIiI', pe.range(handlers + 16 * c, 16))
            catches.append({'adjectives': adj, 'type': hex(td), 'type_name': type_name(pe, td) if td else None,
                            'handler': hex(addr)})
        tries.append({'try_low': low, 'try_high': high, 'catch_high': catch_high, 'catches': catches})
    return {'funcinfo': hex(fi), 'magic': hex(magic), 'max_state': max_state, 'try_blocks': tries}


def throwinfo(pe, ti):
    attr, unwind, fwd, cta = struct.unpack('<IIII', pe.range(ti, 16))
    n = u32(pe, cta)
    types = []
    for i in range(n):
        ct = u32(pe, cta + 4 + 4 * i)
        td = u32(pe, ct + 4)
        types.append({'type': hex(td), 'type_name': type_name(pe, td)})
    return {'throwinfo': hex(ti), 'catchable': types}


def ehframe_funcinfo(pe, handler_stub):
    """`mov eax, FuncInfo; jmp __CxxFrameHandler` stub named by the SEH push."""
    b = pe.range(handler_stub, 10)
    assert b[0] == 0xB8 and b[5] == 0xE9, b.hex()
    return struct.unpack_from('<I', b, 1)[0]


def static_facts():
    out = {}
    for pe_name, pe in (('original', original), ('v14', modified)):
        facts = {
            '5A80D0': funcinfo(pe, ehframe_funcinfo(pe, 0x99BAB0)),
            '59B4B0': funcinfo(pe, ehframe_funcinfo(pe, 0x99AAD0)),
            'callers_without_try': {name: funcinfo(pe, ehframe_funcinfo(pe, stub))
                                    for name, stub in (('463B50', 0x989E30), ('49DA40', 0x98FEF3), ('46BB60', 0x98A6A0))},
            'throw_425210_unable_to_find': throwinfo(pe, 0xAD9DA0),
            'throw_4252A0_not_a': throwinfo(pe, 0xAD9E00),
            'strings': {hex(a): wstr(pe, a) for a in (0xA45E20, 0xA45DD8, 0xA44F38, 0x9F92B8, 0xA42E70, 0xA42E20,
                                                     0xA42C98, 0xA42DF0, 0xA42DB4)},
        }
        out[pe_name] = facts
    assert out['original'] == out['v14']
    return out['original']


# ---------------------------------------------------------------- execution
class Run:
    def __init__(self, mode, warning, find, default_names=None):
        m = self.m = Machine('Single24')
        self.events = []
        self.mode, self.warning, self.find = mode, warning, find
        # [B39940] -> UVar value-pointer array; +F4 (index61) -> the Bool byte.
        table = m.alloc(0x200)
        cell = m.alloc(4)
        m.wb(cell, mode)
        m.w(table + 0xF4, cell)
        m.w(0xB39940, table)
        self.warning_cell = m.alloc(4)
        m.wb(self.warning_cell, warning)
        self.warning_slot = m.alloc(4)
        m.w(self.warning_slot, self.warning_cell)
        self.default_names = default_names or {}
        self.allocator = m.alloc(8)
        vtable = m.alloc(8)
        m.w(self.allocator, vtable)
        m.w(vtable + 4, m.stub(self.release))
        self.catch_return = m.stub(self.catch_continue)
        self.rethrown = False
        self.frame = None
        for address, fn in ((0x5A7750, self.factory), (0x59B1F0, self.factory), (0x405D20, self.var_405d20),
                            (0x405370, self.var_405370), (0x404FC0, self.log), (0x410CB0, self.avar),
                            (0x96BEF5, self.rethrow)):
            m.seam(address, fn)

    def name_arg(self, slot):
        data = self.m.r(slot)
        return bytes(self.m.u.mem_read(data, 512)).decode('utf-16le').split('\0')[0]

    def cstring(self, text):
        """CString data pointer with its native header at data-10h."""
        m = self.m
        raw = text.encode('utf-16le') + b'\0\0'
        head = m.alloc(16 + len(raw))
        m.w(head, self.allocator)
        m.w(head + 4, len(text))
        m.w(head + 8, len(text))
        m.w(head + 12, 1)
        m.u.mem_write(head + 16, raw)
        return head + 16

    def factory(self, m):
        pc = m.u.reg_read(UC_X86_REG_EIP)
        name, report = self.name_arg(m.arg(1)), m.arg(2)
        self.events.append({'call': hex(pc), 'name': name, 'report': report})
        result = self.find(name)
        if result is None:
            assert report == 1
            self.throw(m)
        else:
            m.ret(result)

    def throw(self, m):
        """MSVC dispatch for the enclosing frame's single catch funclet."""
        handler = self.frame['handler']
        ebp = m.u.reg_read(UC_X86_REG_EBP)
        self.events.append({'throw': 'TGC_MissingResourceException', 'catch': hex(handler)})
        sp = m.u.reg_read(UC_X86_REG_ESP) - 0x400
        m.w(sp, self.catch_return)
        m.u.reg_write(UC_X86_REG_ESP, sp)
        m.u.reg_write(UC_X86_REG_EIP, handler)
        self.catch_ebp = ebp

    def catch_continue(self, m):
        target = m.u.reg_read(UC_X86_REG_EAX)
        ebp = m.u.reg_read(UC_X86_REG_EBP)
        assert ebp == self.catch_ebp
        self.events.append({'continuation': hex(target)})
        m.u.reg_write(UC_X86_REG_ESP, m.r(ebp - 0x10))
        m.u.reg_write(UC_X86_REG_EIP, target)

    def var_405d20(self, m):
        name = bytes(m.u.mem_read(m.arg(1), 80)).decode('utf-16le').split('\0')[0]
        self.events.append({'call': '0x405d20', 'ecx': hex(m.ecx()), 'var': name})
        m.ret(self.warning, pop=4)

    def var_405370(self, m):
        name = bytes(m.u.mem_read(m.r(m.arg(1)), 80)).decode('utf-16le').split('\0')[0]
        self.events.append({'call': '0x405370', 'ecx': hex(m.ecx()), 'var': name})
        m.ret(self.warning_slot, pop=4)

    def log(self, m):
        args = [m.arg(i) for i in range(1, 8)]
        name = bytes(m.u.mem_read(args[6], 512)).decode('utf-16le').split('\0')[0]
        self.events.append({'call': '0x404fc0', 'channel': hex(args[0]), 'level': args[1],
                            'file': wstr(original, args[2]), 'line': args[3], 'function': wstr(original, args[4]),
                            'format': wstr(original, args[5]), 'name': name})
        m.ret(0)

    def avar(self, m):
        out = m.arg(1)
        var = bytes(m.u.mem_read(m.arg(2), 120)).decode('utf-16le').split('\0')[0]
        value = self.default_names[var]
        self.events.append({'call': '0x410cb0', 'ecx': hex(m.ecx()), 'var': var, 'value': value})
        m.w(out, self.cstring(value))
        m.ret(out, pop=8)

    def release(self, m):
        self.events.append({'call': 'CString-release'})
        m.ret(0, pop=4)

    def rethrow(self, m):
        self.events.append({'call': '0x96bef5', 'args': [m.arg(1), m.arg(2)]})
        self.rethrown = True
        m.u.emu_stop()

    def call(self, entry, args, handler):
        m = self.m
        self.frame = {'handler': handler}
        name = self.cstring(args[0])
        slot = m.alloc(4)
        m.w(slot, name)
        try:
            value = m.call(entry, args=(slot,) + tuple(args[1:]), pop=0)
            outcome = {'returned': hex(value)}
        except AssertionError:
            if not self.rethrown:
                raise
            outcome = {'rethrown': True}
        m.verify_v14()
        return outcome


RESOURCE = 0x21000000
DEFAULT = 0x21100000


def run_sequence(mode, warning, present):
    r = Run(mode, warning, lambda name: RESOURCE if present else None)
    outcome = r.call(0x5A80D0, ('Units/Ambient/Cow/Cowjog.kf',), 0x5A8115)
    return {'case': {'factory': '5A80D0', 'mode': mode, 'warning': warning, 'present': present},
            'outcome': outcome, 'events': r.events, 'instructions': instructions(r.m.visited)}


def run_sound(mode, warning, present, streaming):
    defaults = {'DefaultAudioResource': 'audio/Default/SoundDefault.wav',
                'DefaultAudioStreamingResource': 'audio/Default/StreamDefault.wav'}
    names = {'audio/Default/SoundDefault.wav': DEFAULT, 'audio/Default/StreamDefault.wav': DEFAULT + 0x100}

    def find(name):
        if name in names:
            return names[name]
        return RESOURCE if present else None
    r = Run(mode, warning, find, defaults)
    m = r.m
    m.u.mem_map(RESOURCE, 0x200000)
    outcome = r.call(0x59B4B0, ('Audio/Sounds/RUS/Attack/RUSRocketTruck01.wav', int(streaming)), 0x59B502)
    if 'returned' in outcome:
        p = int(outcome['returned'], 16)
        outcome['byte41'] = m.rb(p + 0x41)
        for q in (RESOURCE, DEFAULT, DEFAULT + 0x100):
            if q != p:
                assert m.rb(q + 0x41) == 0
    return {'case': {'factory': '59B4B0', 'mode': mode, 'warning': warning, 'present': present, 'streaming': streaming},
            'outcome': outcome, 'events': r.events, 'instructions': instructions(r.m.visited)}


def main():
    facts = static_facts()
    for key in ('5A80D0', '59B4B0'):
        [block] = facts[key]['try_blocks']
        [catch] = block['catches']
        assert catch['type'] == hex(MISSING_TYPE) and catch['adjectives'] == 8
    assert any(t['type'] == hex(MISSING_TYPE) for t in facts['throw_425210_unable_to_find']['catchable'])
    assert all(t['type'] != hex(MISSING_TYPE) for t in facts['throw_4252A0_not_a']['catchable'])
    for v in facts['callers_without_try'].values():
        assert v['try_blocks'] == []
    rows = []
    for mode in (0, 1):
        for warning in (0, 1):
            for present in (False, True):
                rows.append(run_sequence(mode, warning, present))
                for streaming in (False, True):
                    rows.append(run_sound(mode, warning, present, streaming))
    doc = header(__file__, 'unicorn original-code execution with explicit seams and single-frame MSVC catch dispatch')
    doc.update({'static': facts, 'rows': rows})
    (HERE / 'fixtures.json').write_text(json.dumps(doc, indent=1))
    for row in rows:
        brief = [e for e in row['events'] if 'call' in e and e['call'] in ('0x404fc0', '0x410cb0', '0x96bef5')]
        print(row['case'], row['outcome'], [(e['call'], e.get('level'), e.get('var')) for e in brief])


if __name__ == '__main__':
    main()
