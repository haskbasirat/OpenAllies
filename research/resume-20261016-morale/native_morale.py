"""Bounded original ActorData +2C0 producer and 45D580(NULL) oracle.

Original TGI parser 5BD080 -> ElementComponent lookup 466822 -> real
GetFixed resupply_rate/morale_value -> store 4668A2. Stop immediately
after the store, or before the following component when absent. This is
not the whole ActorData loader. Constructor's zero store 46280C runs
on an A5-filled fixture with the constructor-established EBX=0.
"""
from pathlib import Path
import sys, json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-layouts'))
from layout_machine import Machine, P, V, H, native_windows
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

CW = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}
BASE = '[template Thing]\n{\n string IDS\n string category = ACTOR\n [optional ElementComponent]\n {\n fixed morale_value\n fixed resupply_rate = 0\n }\n}\n'

class MoraleMachine(Machine):
    def hook(self, u, pc, size, user):
        if pc == 0x405d60:
            # Diagnostic CString vararg formatter only. The wrong-type
            # rejection and C++ throw execute natively; formatted presentation
            # text is not part of the gameplay/source result being measured.
            self.recorded.append({'event': 'exception_text_format', 'format': self.raw_wstr(self.arg(1))})
            self.w(self.arg(), self.cstring('<diagnostic formatter seam>'))
            self.ret()
            return
        if pc == 0x466b17:
            u.emu_stop()
            return
        super().hook(u, pc, size, user)

    def run_actor(self, node):
        actor = self.alloc(0x600)
        self.u.mem_write(actor, b'\xa5' * 0x600)
        self.u.reg_write(UC_X86_REG_ESI, actor)
        self.u.reg_write(UC_X86_REG_EBX, 0)
        self.u.emu_start(0x46280c, 0x462812, count=1)
        assert self.r(actor + 0x2c0) == 0
        self.w(actor + 0x174, 0)
        frame = self.alloc(0x10000)
        esp = frame + 0xf000
        self.w(esp + 0xb0, node)
        self.u.reg_write(UC_X86_REG_ESP, esp)
        self.u.reg_write(UC_X86_REG_EBP, actor)
        self.thrown = None
        self.messages = []
        self.last_throw = None
        self.u.emu_start(0x466822, 0x4668a8, count=2_000_000)
        self.check_null()
        end = self.u.reg_read(UC_X86_REG_EIP)
        result = {'stored_bits': self.r(actor + 0x2c0),
                  'mask_174': self.r(actor + 0x174), 'stopped_at': hex(end),
                  'thrown': self.thrown, 'messages': list(self.messages),
                  'seams': [e for e in self.recorded if e['event'] == 'exception_text_format']}
        if self.thrown is None:
            assert end in (0x4668a8, 0x466b17), hex(end)
            # Original getter executes; a fresh fixture instruction stores ST0
            # to binary32 solely to observe its return (not a game seam).
            assert 'eax' in self.thiscall(0x45d580, actor, [0])
            out = self.alloc(4)
            stub = self.alloc(16)
            self.u.mem_write(stub, b'\xd9\x1d' + out.to_bytes(4, 'little'))
            self.u.emu_start(stub, stub + 6, count=1)
            result['getter_bits'] = self.r(out)
        return result

def cases():
    for value in ('0', '-0', '1', '-5', '12.25', '100', '37%', '0.1', '16777217', '1.0000000596046448'):
        yield 'literal_' + value, BASE, '[Thing]\n{ IDS = subject\n [ElementComponent]\n { morale_value = ' + value + '\n }\n}\n'
    yield 'component_absent', BASE, '[Thing]\n{ IDS = subject\n}\n'
    yield 'unassigned', BASE, '[Thing]\n{ IDS = subject\n [ElementComponent]\n {}\n}\n'
    yield 'missing', BASE.replace(' fixed morale_value\n', ''), '[Thing]\n{ IDS = subject\n [ElementComponent]\n {}\n}\n'
    yield 'wrong_float', BASE.replace('fixed morale_value', 'float morale_value'), '[Thing]\n{ IDS = subject\n [ElementComponent]\n { morale_value = 2.5\n }\n}\n'
    yield 'inherited', BASE + '[template Inherited template=Thing]\n{ [ElementComponent]\n { morale_value = 33.75\n }\n}\n', '[Thing template=Inherited]\n{ IDS = subject\n}\n'
    yield 'override', BASE + '[template Inherited template=Thing]\n{ [ElementComponent]\n { morale_value = 33.75\n }\n}\n', '[Thing template=Inherited]\n{ IDS = subject\n [ElementComponent]\n { morale_value = 42\n }\n}\n'

def main():
    rows, visited = [], {}
    for pc in CW:
        for name, template, source in cases():
            m = MoraleMachine()
            m.u.reg_write(UC_X86_REG_FPCW, CW[pc])
            m.parse_file('template.tgi', template.encode())
            source = source.replace('[Thing]', '[Thing template=Thing]')
            parsed = m.reparse(source.encode(), 'subject.tgi')
            if parsed['error'] is not None:
                result = {'stage': 'parser', 'thrown': parsed['error'], 'messages': list(m.messages)}
            else:
                children = m.children(parsed['root'])
                assert len(children) == 1
                result = {'stage': 'producer', **m.run_actor(children[0])}
            rows.append({'name': name, 'precision': pc, 'template': template, 'source': source, 'result': result})
            visited.update(m.visited)
            print(pc, name, result, flush=True)
    code = []
    dis = Cs(CS_ARCH_X86, CS_MODE_32)
    for a, z in [(0x46280c, 0x462812), (0x466822, 0x4668a8), (0x45d580, 0x45d5b0)]:
        b = P.range(a, z - a)
        assert b == V.range(a, z - a)
        code.extend({'address': hex(i.address), 'bytes': i.bytes.hex(), 'mnemonic': i.mnemonic, 'operands': i.op_str} for i in dis.disasm(b, a))
    out = {'schema': 1, 'scope': __doc__, 'original_sha256': H(P.b), 'v14_sha256': H(V.b),
           'cases': rows, 'instructions': code, 'native_windows': native_windows(visited)}
    path = HERE / 'native-morale.json'
    path.write_text(json.dumps(out, indent=2) + '\n')
    print('wrote', path, H(path.read_bytes()))

if __name__ == '__main__':
    main()
