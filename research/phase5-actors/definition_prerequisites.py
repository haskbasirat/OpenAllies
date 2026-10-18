"""Record original definition-loader boundaries and execute actual derived getters.

This is evidence for later integration, not a substitute TGI definition loader.
No original file is written and no original arithmetic instruction is mocked.
"""
from pathlib import Path
import json, struct
from original_creation import Machine, P, V, H, bits
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from unicorn.x86_const import UC_X86_REG_EIP

HERE = Path(__file__).resolve().parent


def utf16(p):
    words = []
    for i in range(256):
        v = struct.unpack('<H', P.range(p + 2*i, 2))[0]
        if not v:
            return bytes(b''.join(struct.pack('<H', x) for x in words)).decode('utf-16-le')
        words.append(v)
    raise ValueError('unterminated PE string')


class GetterMachine(Machine):
    def __init__(self, cw):
        super().__init__(cw)
        self.modifier = 0x2250000
        self.settings = 0x2251000
        self.limit = 0x2252000
        self.float_output = 0x2253000
        self.getter_stop = self.stop
        self.stop += 6
        # Complete getter returns ST0; a diagnostic FSTP rounds to binary32.
        self.u.mem_write(self.getter_stop, b'\xd9\x1d' + struct.pack('<I', self.float_output))
        self.w(0xb39ab0, self.settings)
        self.w(self.settings + 0x1a4, self.limit)

    def invoke(self, pc, this, args):
        # Parent invokes with return address self.stop. Redirect only the return
        # to our explicit diagnostic store; no getter branch is substituted.
        self.getter_entry = pc
        self.getter_active = True
        result = super().invoke(pc, this, args)
        self.getter_active = False
        return result

    def hook(self, u, pc, size, user):
        if getattr(self, 'getter_active', False) and pc == self.getter_entry:
            self.w(self.stack, self.getter_stop)
            self.getter_active = False
        super().hook(u, pc, size, user)


def main():
    ranges = [('original_direction_startup_count', 0x9c8680, 15),
              ('definition_constructor', 0x4624b0, 2451),
              ('actor_definition_load', 0x464bd0, 16628),
              ('speed_getter', 0x45d6f0, 34), ('health_getter', 0x45d730, 34),
              ('defense_getter', 0x45d760, 34), ('speed_modifier', 0x5014c0, 10),
              ('health_modifier', 0x5014f0, 10), ('defense_modifier', 0x501b10, 95),
              ('component_definition_getter', 0x529c30, 7)]
    windows = []
    for name, address, size in ranges:
        raw = P.range(address, size)
        assert raw == V.range(address, size), (name, hex(address))
        windows.append({'name': name, 'address': hex(address), 'size': size, 'sha256': H(raw)})

    # Keep actual instructions around every mask load/store. This inventory is
    # static evidence: it does not claim to execute or recover the whole loader.
    decoder = Cs(CS_ARCH_X86, CS_MODE_32)
    instructions = list(decoder.disasm(P.range(0x464bd0, 16628), 0x464bd0))
    indexes = set()
    for i, ins in enumerate(instructions):
        if '[ebp + 0x174]' in ins.op_str:
            indexes.update(range(max(0, i-5), min(len(instructions), i+5)))
    lines = [f'{instructions[i].address:08x} {instructions[i].bytes.hex():20s} '
             f'{instructions[i].mnemonic} {instructions[i].op_str}' for i in sorted(indexes)]
    (HERE / 'definition-mask-instructions.txt').write_text('\n'.join(lines)+'\n')

    cases = []
    for precision, cw in [('Single24', 0x7f), ('Double53', 0x27f), ('Extended64', 0x37f)]:
        # Getters are allocation-free and leave an empty x87 stack after our
        # diagnostic store. Reuse the isolated image; every consumed input is
        # overwritten below, rather than resetting 65,536 registry slots441 times.
        m = GetterMachine(cw)
        for name, address, offset, modifier_offset in [
                ('speed', 0x45d6f0, 0x228, 0x38),
                ('health', 0x45d730, 0x284, 0x50),
                ('defense', 0x45d760, 0x288, 0x10)]:
            for base in [0.0, -0.0, -2.0, 0.1, 0.99999994, 100.0, 999.25]:
                for modifier in [None, -5.0, 0.0, 0.1, 1.0, 1.00000012, 2.0]:
                    m.w(m.definition + offset, bits(base))
                    m.w(m.modifier + modifier_offset, bits(modifier or 0))
                    m.w(m.limit, bits(0.25))
                    m.invoke(address, m.definition, [0 if modifier is None else m.modifier])
                    assert m.u.reg_read(UC_X86_REG_EIP) == m.stop
                    cases.append({'getter': name, 'precision': precision, 'base_bits': bits(base),
                                  'modifier_bits': None if modifier is None else bits(modifier),
                                  'defense_floor_limit_bits': bits(0.25), 'result_bits': m.r(m.float_output)})

    body_fields = {hex(a): utf16(a) for a in [0xa08980, 0xa08970, 0x9fc720]}
    output = {'schema': 1, 'passed': True, 'original_sha256': H(P.b), 'v14_sha256': H(V.b),
              'producer_sha256': H(Path(__file__).read_bytes()), 'native_windows': windows,
              'body_field_names': body_fields, 'getter_cases': cases,
              'seams': ['Getter instructions execute unchanged. Only synthetic definition/modifier/settings memory is supplied.',
                        'Each returned ST0 is diagnostically stored as binary32. A caller retaining extended precision needs separate evidence.',
                        'Definition-mask inventory is static disassembly, not a dynamic test of the complete loader.',
                        'No authored definition inheritance, node merge, faction modifier construction or component mask is inferred.']}
    p = HERE / 'definition-prerequisites.json'
    p.write_text(json.dumps(output, indent=2)+'\n')
    print(json.dumps({'getter_cases': len(cases), 'body_field_names': body_fields, 'sha256': H(p.read_bytes())}))


if __name__ == '__main__':
    main()
