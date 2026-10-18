"""Execute actual 57C800/578E90/57C680 until return or first required live service.

Inputs are captured from actual Rust saved allocations, with test substitutions
explicitly labelled. Mind virtual dispatch, ground relocation, interpolation and
scheduling stop at their required entry; none returns invented success.
"""
from pathlib import Path
import copy
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-economy'))
from harness import Machine, ORIGINAL, ACCEPTED, MODES, sha, check_null_accesses
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_FPCW

INPUT = HERE / 'actual-saved-inputs.json'
STUB64 = 0x300e000
STUB7C = 0x300e010
BOUNDARIES = {STUB64: 'mind_64', STUB7C: 'mind_7c', 0x4eeb40: 'relocate',
              0x57c080: 'interpolate', 0x50b220: 'has_scheduled', 0x50ac00: 'schedule'}


class Probe(Machine):
    def __init__(self, image, cw, row):
        super().__init__(cw)
        if image == 'v14':
            old, new = (ORIGINAL.size + 4095) & ~4095, (ACCEPTED.size + 4095) & ~4095
            if new > old:
                self.uc.mem_map(ORIGINAL.base + old, new - old)
            for rva, size, offset, _ in ACCEPTED.sections:
                if size:
                    self.uc.mem_write(ACCEPTED.base + rva, ACCEPTED.data[offset:offset + size])
        self.instructions = {}
        self.boundary = None
        self.actors = {actor['id']: self.alloc(0x200) for actor in row['actors']}
        vtable = self.alloc(0x100)
        self.put(vtable + 0x64, STUB64)
        self.put(vtable + 0x7c, STUB7C)
        for actor in row['actors']:
            address = self.actors[actor['id']]
            self.put(address + 8, actor['flags'])
            self.put_words(address + 0x20, actor['position'])
            mind = self.alloc(0x80) if actor['mind'] else 0
            self.put(address + 0x70, mind)
            if mind:
                self.put(mind, vtable)
                self.put(mind + 4, address)
        self.org = self.alloc(0xd8)
        self.put(self.org + 4, self.pointer(row['owner']))
        self.put(self.org + 0x40, self.pointer(row['selected']))
        self.put_words(self.org + 0xbc, row['goal'])
        world = self.alloc(0x100)
        self.put(0xb36534, world)
        self.put(world + 0xd8, row['world_mode'])

    def pointer(self, identity):
        if identity is None or identity == 0:
            return 0
        # A nonselected, deliberately absent GTID must not be dereferenced.
        # Return unmapped storage instead of a convenient zero-filled Actor.
        return self.actors.get(identity, 0x1badb000)

    def _code(self, uc, address, size, data):
        if ORIGINAL.base <= address < ORIGINAL.base + ORIGINAL.size:
            self.instructions[f'0x{address:08x}'] = bytes(uc.mem_read(address, size)).hex()
        if address in BOUNDARIES:
            self.boundary = BOUNDARIES[address]
            uc.emu_stop()
            return
        super()._code(uc, address, size, data)

    def run(self, member):
        self.put(self.STACK, self.STOP)
        self.put(self.STACK + 4, self.pointer(member))
        self.uc.reg_write(UC_X86_REG_ESP, self.STACK)
        self.uc.reg_write(UC_X86_REG_ECX, self.org)
        self.uc.reg_write(UC_X86_REG_FPCW, self.cw)
        self.uc.emu_start(0x57c800, self.STOP, count=10000)
        check_null_accesses(self)
        pc = self.uc.reg_read(UC_X86_REG_EIP)
        assert pc == self.STOP or pc in BOUNDARIES, hex(pc)
        return {'outcome': self.boundary or 'ok', 'goal': self.words(self.org + 0xbc, 2)}


def main():
    inputs = json.loads(INPUT.read_text())
    rows, instructions, probes = [], {}, []
    for image in ['original', 'v14']:
        for item in inputs:
            mode = next(cw for pc, cw in MODES if pc == item['precision'])
            machine = Probe(image, mode, item['input'])
            output = machine.run(item['input']['member'])
            for address, encoded in machine.instructions.items():
                raw = bytes.fromhex(encoded)
                assert ORIGINAL.at(int(address, 16), len(raw)) == raw
                assert ACCEPTED.at(int(address, 16), len(raw)) == raw
                instructions[address] = encoded
            rows.append({'image': image, **item, 'output': output})
        # Isolated native caller probes are explicitly outside the actual
        # saved-allocation Rust comparison. A real native Mind allocation and
        # vtable slot are provided, but its required predicate is stopped,
        # never answered. No cross-owner component is installed in Rust.
        seed = next(item for item in inputs if item['input']['name'].endswith('_actual'))
        for pc, cw in MODES:
            for owner_flag30 in [False, True]:
                data = copy.deepcopy(seed['input'])
                data['name'] = f'isolated_native_present_mind_flag30_{owner_flag30}'
                data['scope'] = 'isolated native input substitution; not an observed saved Mind or Rust allocation'
                data['selected'] = data['member']
                for actor in data['actors']:
                    if actor['id'] == data['owner']:
                        actor['mind'] = True
                        actor['flags'] = actor['flags'] | (1 << 30) if owner_flag30 else actor['flags'] & ~(1 << 30)
                    if actor['id'] == data['member']:
                        actor['flags'] &= ~(1 << 30)
                machine = Probe(image, cw, data)
                output = machine.run(data['member'])
                assert output['outcome'] == 'mind_64'
                for address, encoded in machine.instructions.items():
                    raw = bytes.fromhex(encoded)
                    assert ORIGINAL.at(int(address, 16), len(raw)) == raw
                    assert ACCEPTED.at(int(address, 16), len(raw)) == raw
                    instructions[address] = encoded
                probes.append({'image': image, 'precision': pc, 'input': data, 'output': output})
    out = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(ORIGINAL.data),
           'v14_sha256': sha(ACCEPTED.data), 'producer_sha256': sha(Path(__file__).read_bytes()),
           'harness_sha256': sha((ROOT / 'research/phase8-economy/harness.py').read_bytes()),
           'input_sha256': sha(INPUT.read_bytes()),
           'required_services_execute_or_return_success': False,
           'page_zero_guard': 'only FS:[0] accesses permitted',
           'missing_nonselected_actor': 'unmapped native pointer; must remain unread',
           'instruction_bytes': instructions, 'cases': rows, 'isolated_native_probes': probes}
    path = HERE / 'native-saved-position.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(out, indent=2) + '\n')
    else:
        assert json.loads(path.read_text()) == out
    counts = {}
    for row in rows:
        key = row['output']['outcome']
        counts[key] = counts.get(key, 0) + 1
    print(json.dumps({'cases': len(rows), 'isolated_native_probes': len(probes),
                      'outcomes': counts, 'sha256': sha(path.read_bytes())}))


if __name__ == '__main__':
    main()
