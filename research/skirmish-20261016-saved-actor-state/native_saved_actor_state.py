"""Run real 4D64F0 until return or the first required state-effect entry.

The registered descriptors/masks are projected from the prior native 477FC0
registration oracle. No callback returns success: live enter/leave and exclusive
action rebuilding stop at their original entry, before executing their bodies.
"""
from pathlib import Path
import json
import random
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-economy'))
from harness import Machine, ORIGINAL, ACCEPTED, MODES, sha, check_null_accesses
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_FPCW

REGISTRATION = ROOT / 'research/phase7-actors/native-actor-states.json'
REGISTERED = json.loads(REGISTRATION.read_text())['registration']
LOAD_PREFIX = ROOT / 'qa-output/economy-lifecycle-20261016/managed-element-attach.json'
EFFECTS = {0x4d4a90: '4D4A90', 0x4d16b0: '4D16B0', 0x4d1350: '4D1350'}


class Probe(Machine):
    def __init__(self, image, cw):
        super().__init__(cw)
        if image == 'v14':
            old, new = (ORIGINAL.size + 4095) & ~4095, (ACCEPTED.size + 4095) & ~4095
            if new > old:
                self.uc.mem_map(ORIGINAL.base + old, new - old)
            for rva, size, offset, _ in ACCEPTED.sections:
                if size:
                    self.uc.mem_write(ACCEPTED.base + rva, ACCEPTED.data[offset:offset + size])
        self.actor = self.alloc(0x200)
        self.manager = self.alloc(0x400)
        self.table = self.alloc(42 * 4)
        self.put(0xb36530, self.manager)
        self.put(self.manager + 0x2e4, self.table)
        for record in REGISTERED['records']:
            descriptor = self.alloc(0x40)
            self.put(self.table + record['index'] * 4, descriptor)
            self.put(descriptor + 0xc, record['index'])
            self.put(descriptor + 0x2c, record['group'])
            self.put8(descriptor + 0x30, int(record['exclusive']))
        for index, mask in enumerate(REGISTERED['masks']):
            self.put(0xb3ac40 + index * 8, mask)
            self.put(0xb3ac44 + index * 8, mask >> 32)
        self.boundary = None
        self.instructions = {}

    def _code(self, uc, address, size, data):
        if ORIGINAL.base <= address < ORIGINAL.base + ORIGINAL.size:
            self.instructions[f'0x{address:08x}'] = bytes(uc.mem_read(address, size)).hex()
        if address in EFFECTS:
            self.boundary = EFFECTS[address]
            uc.emu_stop()
            return
        super()._code(uc, address, size, data)

    def run(self, bits, flags, index, enabled, poison_catalog=False):
        self.boundary = None
        self.put(self.actor + 0x100, bits)
        self.put(self.actor + 0x104, bits >> 32)
        self.put(self.actor + 8, flags)
        self.put(0xb36530, 0 if poison_catalog else self.manager)
        self.put(self.STACK, self.STOP)
        self.put(self.STACK + 4, index)
        self.put(self.STACK + 8, int(enabled))
        self.uc.reg_write(UC_X86_REG_ESP, self.STACK)
        self.uc.reg_write(UC_X86_REG_ECX, self.actor)
        self.uc.reg_write(UC_X86_REG_FPCW, self.cw)
        self.uc.emu_start(0x4d64f0, self.STOP, count=10000)
        check_null_accesses(self)
        pc = self.uc.reg_read(UC_X86_REG_EIP)
        assert pc == self.STOP or pc in EFFECTS, hex(pc)
        return {
            'bits': self.read(self.actor + 0x100) | self.read(self.actor + 0x104) << 32,
            'flags': self.read(self.actor + 8),
            'first_required_effect': self.boundary,
            'completed': pc == self.STOP,
        }


def main():
    rng = random.Random(0x4d64f0)
    inputs = []
    for index in range(42):
        for bits in [0, (1 << 42) - 1, 1 << index, rng.getrandbits(42)]:
            for enabled in [False, True]:
                for flags in [0x100, 0x80000100]:
                    inputs.append(dict(bits=bits, flags=flags, index=index, enabled=enabled,
                                       poison_catalog=((bits & (1 << index) != 0) == enabled)))
    # Concrete saved-map input from the existing Rust partial restore, not a
    # claim that a complete native map load or immediate Init was executed.
    load_prefix = json.loads(LOAD_PREFIX.read_text())
    owner = next(row for row in load_prefix['records'] if row['id'] == 4099)
    member = next(row for row in load_prefix['records'] if row['id'] == 4100)
    loaded_bits = owner['state_low'] | owner['state_high'] << 32
    for index, enabled in [(34, bool(member['state_high'] & 4)), (37, False)]:
        assert ((loaded_bits & (1 << index)) != 0) == enabled
        inputs.append(dict(bits=loaded_bits, flags=owner['flags'], index=index,
                           enabled=enabled, poison_catalog=True))
    cases = []
    pins = {}
    for image in ['original', 'v14']:
        for pc, cw in MODES:
            machine = Probe(image, cw)
            for inputs_row in inputs:
                cases.append({'image': image, 'precision': pc, 'input': inputs_row,
                              'output': machine.run(**inputs_row)})
            for address, instructions in machine.instructions.items():
                assert instructions == ORIGINAL.at(int(address, 16), len(bytes.fromhex(instructions))).hex()
                assert instructions == ACCEPTED.at(int(address, 16), len(bytes.fromhex(instructions))).hex()
                pins[address] = instructions
    out = dict(schema=1, scope=__doc__, original_sha256=sha(ORIGINAL.data),
               v14_sha256=sha(ACCEPTED.data), producer_sha256=sha(Path(__file__).read_bytes()),
               harness_sha256=sha((ROOT / 'research/phase8-economy/harness.py').read_bytes()),
               descriptor_fixture_sha256=sha(REGISTRATION.read_bytes()),
               concrete_load_input={'path': str(LOAD_PREFIX.relative_to(ROOT)),
                                    'sha256': sha(LOAD_PREFIX.read_bytes()),
                                    'scope': 'Rust partial tutorial02 owner4099/member4100; '
                                             'state34 request from actual member high&4; '
                                             'state37 false case before later unbound steps'},
               descriptor_source='prior original 477FC0 registration oracle; no SDK implementation',
               callbacks_executed=False, page_zero_guard='only FS:[0] accesses permitted',
               instruction_bytes=pins, cases=cases)
    path = HERE / 'native-saved-actor-state.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(out, indent=2) + '\n')
    else:
        assert json.loads(path.read_text()) == out
    print(json.dumps({'cases': len(cases), 'completed': sum(c['output']['completed'] for c in cases),
                      'required_effect_boundary': sum(not c['output']['completed'] for c in cases),
                      'sha256': sha(path.read_bytes())}))


if __name__ == '__main__':
    main()
