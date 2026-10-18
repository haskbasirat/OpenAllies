"""Original x86 terrain layer selection/mask lookup; original files are read-only.

Only allocation and the coordinate-hashed TransitionSet/texture resource leaf
are replaced. Neighborhood choices, CRT qsort, gap closing and mask orientation
lookup execute unchanged.
This is CPU evidence, not native GPU blending or offscreen sampling parity.
"""
from pathlib import Path
import hashlib, json, random, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-rng'))
from original_rng import Machine, original, modified, sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_ESP
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

RANGES = {
    'neighborhood': (0x729440, 0x7296b0),
    'layer_selection': (0x729760, 0x7299d0),
    'priority_compare': (0x729230, 0x729258),
    'mask_rotation': (0x72a950, 0x72aa72),
    'mask_reflection': (0x72ab30, 0x72ac09),
    'mask_registration': (0x72aff0, 0x72b031),
    'mask_allocation': (0x729ea0, 0x729f2d),
    'mask_matrix_lookup': (0x729180, 0x729230),
    'mask_constants': (0xb285f8, 0xb28a20),
}
for start, end in RANGES.values():
    assert original.range(start, end-start) == modified.range(start, end-start)

class TerrainMachine(Machine):
    OUTPUT = 0x2001000
    BLOCK = 0x2002000
    TILES = 0x2003000
    VARIATIONS = 0x2004000
    MANAGER = 0x2005000
    AVARS = 0x2005400
    MASK_TABLE = 0x2006000
    CENTER = 0x2007000
    COORD = 0x2007100
    HEAP = 0x4000000

    def __init__(self):
        super().__init__()
        self.executed = {}
        self.u.mem_map(self.HEAP, 0x10000)
        self.next_heap = self.HEAP
        self.u.hook_add(UC_HOOK_CODE, self.hook)
        self.run(0x72aff0, self.MASK_TABLE, [])
        self.table = []
        for mask in range(256):
            address = self.r(self.MASK_TABLE + mask*4)
            if not address:
                self.table.append(None)
                continue
            base = self.r(address+12) or address
            transform = self.r(address+8)
            matrix_address = self.run(0x729180, 0, [transform])
            matrix = struct.unpack('<16f', self.u.mem_read(matrix_address, 64))
            self.table.append({'mask': mask, 'base_mask': self.r(base),
                               'transform': transform, 'matrix': matrix})
        self.w(0xb36518, self.MANAGER)
        self.w(self.MANAGER+0xb0, 2)
        self.w(self.MANAGER+0x30, self.MASK_TABLE)
        self.w(0xb39940, self.AVARS)
        self.w(self.AVARS+0x198, self.AVARS+0x200)
        self.w(self.BLOCK+0xd0, 3)
        self.w(self.BLOCK+0x88, self.TILES)
        self.w(self.COORD, 2)
        self.w(self.COORD+4, 2)
        for i in range(9):
            self.w(self.OUTPUT+i*4, self.OUTPUT+0x100+i*0x20)
            self.w(self.VARIATIONS+i*0x100+0x20, self.VARIATIONS+i*0x100+0x40)

    def ret(self, value, pop=0):
        stack = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EAX, value)
        self.u.reg_write(UC_X86_REG_EIP, self.r(stack))
        self.u.reg_write(UC_X86_REG_ESP, stack+4+pop)

    def hook(self, u, address, size, _):
        if original.base <= address < original.base+original.image_size and address not in self.executed:
            code = original.range(address, size)
            assert code == modified.range(address, size), hex(address)
            self.executed[address] = code
        if address == 0x6f7810:
            size = self.r(u.reg_read(UC_X86_REG_ESP)+4)
            result = self.next_heap
            self.next_heap += (size+15)&~15
            assert self.next_heap < self.HEAP+0x10000
            self.ret(result)
        elif address == 0x7296b0:
            # Coordinate-hashed TransitionSet choice and final texture resolution
            # are outside this boundary. Native code already chose base/transform.
            self.ret(0, 12)

    def run(self, function, this, args):
        self.u.context_restore(self.initial)
        self.w(self.STACK, self.STOP)
        for i, value in enumerate(args):
            self.w(self.STACK+4+i*4, value)
        self.u.reg_write(UC_X86_REG_ECX, this)
        self.u.reg_write(UC_X86_REG_ESP, self.STACK)
        self.u.emu_start(function, self.STOP, count=200000)
        assert self.u.reg_read(UC_X86_REG_EIP) == self.STOP
        return self.u.reg_read(UC_X86_REG_EAX)

    def layers(self, ids, priorities):
        for i, priority in enumerate(priorities):
            self.w(self.VARIATIONS+i*0x100+0x18, priority)
        for i, ident in enumerate(ids):
            self.w(self.TILES+((i//3+1)*5+i%3+1)*16, self.VARIATIONS+ident*0x100)
        self.w(self.CENTER, self.VARIATIONS+ids[4]*0x100)
        self.run(0x729760, self.OUTPUT, [self.CENTER, self.COORD, self.BLOCK])
        result = []
        for i in range(self.r(self.OUTPUT+0x24)):
            p = self.r(self.OUTPUT+i*4)
            ident = (self.r(p)-self.VARIATIONS)//0x100
            mask = self.r(p+0xc)
            entry = self.table[mask] or self.table[0]
            result.append({'id': ident, 'priority': priorities[ident], 'mask': mask,
                           'base_mask': None if i == 0 else entry['base_mask'],
                           'transform': self.r(p+0x1c)})
        return result

def main():
    machine = TerrainMachine()
    asset_root = ROOT.parent/'reconstruction/assets/original/Data'
    authored = []
    for base in sorted({entry['base_mask'] for entry in machine.table if entry}):
        digits = ''.join(str(i+1) for i in range(9) if i == 4 or base & (1 << (i-(i>4))))
        path = asset_root/'Terrain/Transitions/Global01'/f'Trans-{digits}.tga'
        raw = path.read_bytes()
        authored.append({'base_mask':base, 'filename_suffix':f'{digits}.tga',
                         'source':path.relative_to(ROOT.parent).as_posix(), 'sha256':sha(raw),
                         'width':struct.unpack_from('<H',raw,12)[0], 'height':struct.unpack_from('<H',raw,14)[0]})
    assert len(authored) == 33
    cases = []
    # Exhaustive binary neighborhoods plus three directional-priority examples
    # and a bounded deterministic random corpus with unique material priorities.
    grids = [([(bits>>i)&1 for i in range(9)], [0, 1]) for bits in range(512)]
    grids += [(list(range(9)), [i if j == 4 else j+10 for j in range(9)]) for i in [-1,5,50]]
    rng = random.Random(0x729760)
    for _ in range(512):
        priorities = rng.sample(range(-100,100), 9)
        grids.append(([rng.randrange(9) for _ in range(9)], priorities))
    for ids, priorities in grids:
        cases.append({'neighbors': ids, 'priorities': priorities, 'layers': machine.layers(ids, priorities)})
    report = {'schema': 1, 'original_sha256': sha(original.b), 'v14_sha256': sha(modified.b),
              'producer_sha256': sha(Path(__file__).read_bytes()),
              'method': __doc__, 'ranges': {name: {'start':hex(start), 'end_exclusive':hex(end),
                         'sha256':sha(original.range(start,end-start)), 'unchanged_v14':True}
                         for name,(start,end) in RANGES.items()},
              'executed_instruction_count': len(machine.executed),
              'executed_instructions_sha256': sha(b''.join(struct.pack('<I',a)+b for a,b in sorted(machine.executed.items()))),
              'all_executed_instructions_unchanged_v14': True,
              'authored_resources': authored,
              'table': machine.table, 'cases': cases,
              'limits': ['Unique priorities only; equal-priority CRT-qsort ordering is not ported.',
                         'Fully populated 3x3 neighborhood; native block-border reconstruction not ported.',
                         'Coordinate-hashed TransitionSet selection at 0x7296B0 and its texture resolution are not ported; filenames are suffixes only.',
                         'GPU alpha equation, filtering, target texel alignment and source texture composition not tested.']}
    output = HERE/'native-layer-fixtures.json'
    output.write_text(json.dumps(report, indent=2), encoding='utf8')
    (HERE/'mask-lookup.json').write_text(json.dumps({'schema':1, 'table':machine.table},indent=2), encoding='utf8')
    cs = Cs(CS_ARCH_X86, CS_MODE_32)
    text = []
    for name,(start,end) in RANGES.items():
        if name == 'mask_constants': continue
        text.append(name)
        text.extend(f'{i.address:08x} {i.mnemonic} {i.op_str}' for i in cs.disasm(original.range(start,end-start),start))
    (HERE/'native-layer-assembly.txt').write_text('\n'.join(text),encoding='utf8')
    print(json.dumps({'cases':len(cases), 'masks':sum(x is not None for x in machine.table),
                      'path':str(output), 'sha256':sha(output.read_bytes())}))

if __name__ == '__main__': main()
