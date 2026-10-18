"""Run original terrain compositor instruction slices; never starts the game.

The fixtures cover source UVs and normal-mode modulation, not final lighting,
texture filtering, blend masks, offscreen target rasterization, or water.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

ORIGINAL_SHA = '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
ACCEPTED_SHA = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
RANGES = {
    'variation_scale_loader': (0x4b70f0, 0x4b71fc),
    'quad_uv_initialization': (0x72b2b3, 0x72b2e2),
    'quad_geometry_color': (0x729f30, 0x72a1f8),
    'source_texture_matrix': (0x72a473, 0x72a51e),
    'fmod_dispatch': (0x96db7a, 0x96dba2),
    'fmod_classifier': (0x9791c0, 0x97939a),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Pe:
    def __init__(self, path, expected):
        self.data = path.read_bytes()
        assert sha(self.data) == expected, f'image identity changed: {path}'
        p, = struct.unpack_from('<I', self.data, 0x3c)
        assert self.data[p:p + 4] == b'PE\0\0'
        count, = struct.unpack_from('<H', self.data, p + 6)
        optional, = struct.unpack_from('<H', self.data, p + 20)
        self.base, = struct.unpack_from('<I', self.data, p + 52)
        self.size, = struct.unpack_from('<I', self.data, p + 80)
        self.headers, = struct.unpack_from('<I', self.data, p + 84)
        self.sections = [struct.unpack_from('<4I', self.data, p + 24 + optional + i * 40 + 8) for i in range(count)]

    def at(self, va, size):
        rva = va - self.base
        for _, start, length, offset in self.sections:
            if start <= rva and rva + size <= start + length:
                return self.data[offset + rva - start:offset + rva - start + size]
        raise ValueError(hex(va))

    def load(self, uc):
        uc.mem_map(self.base, (self.size + 4095) & ~4095)
        uc.mem_write(self.base, self.data[:self.headers])
        for _, rva, length, offset in self.sections:
            if length:
                uc.mem_write(self.base + rva, self.data[offset:offset + length])


class Machine:
    def __init__(self, pe):
        self.uc = Uc(UC_ARCH_X86, UC_MODE_32)
        pe.load(self.uc)
        self.uc.mem_map(0x2000000, 0x10000)
        self.uc.mem_map(0x3000000, 0x10000)
        self.uc.reg_write(UC_X86_REG_FPCW, 0x037f)
        self.uc.reg_write(UC_X86_REG_ESP, 0x3008000)

    def put(self, va, *values):
        self.uc.mem_write(va, struct.pack('<' + 'I' * len(values), *values))

    def floats(self, va, *values):
        self.uc.mem_write(va, struct.pack('<' + 'f' * len(values), *values))

    def read(self, va, count):
        return list(struct.unpack('<' + 'f' * count, self.uc.mem_read(va, count * 4)))

    def run(self, start, stop):
        self.uc.emu_start(start, stop, count=50000)
        assert self.uc.reg_read(UC_X86_REG_EIP) == stop


def fixture(pe, origin, size, dimensions):
    m = Machine(pe)
    obj, vertices, uv, colors = 0x2000000, 0x2000100, 0x2000200, 0x2000300
    m.put(obj, 16)
    m.put(obj + 0xc, vertices, uv, colors)
    m.uc.reg_write(UC_X86_REG_ESI, obj)
    m.uc.reg_write(UC_X86_REG_EBP, 0)
    m.uc.reg_write(UC_X86_REG_ECX, 0x3f800000)
    m.run(0x72b2b3, 0x72b2e2)
    base_uv = [m.read(uv + i * 8, 2) for i in range(4)]
    # Normal mode has tile+0xC bit0 clear. Execute original branch and all
    # original color/quad-position stores, stopping before resource selection.
    tile, grid = 0x2000400, 0x2000500
    m.put(grid, 3, 7)
    m.put(0x3008000, 0, tile, grid, 0)
    m.uc.reg_write(UC_X86_REG_ECX, obj)
    m.run(0x729f30, 0x72a160)
    quad = [m.read(vertices + i * 12, 3) for i in range(4)]
    modulation = [m.read(colors + i * 16, 4) for i in range(4)]
    assert modulation == [[1.0] * 4] * 4
    # Actual texture matrix slice and actual CRT fmod implementation. No
    # arithmetic/function mocks. All fields outside this slice are explicit seams.
    layer, variation, matrix, origins, terrain = [0x2001000 + i * 0x200 for i in range(5)]
    m.put(layer, variation)
    m.put(layer + 0x14, matrix)
    m.floats(variation + 0x24, *dimensions)
    m.put(obj + 0x54, origins)
    m.floats(origins + 0x28, *origin)
    m.put(0xb36518, terrain)
    m.put(terrain + 0xb0, size)
    m.uc.reg_write(UC_X86_REG_ESP, 0x3008000)
    m.put(0x3008010, obj)
    # Matrix zeros/identity are established by the caller's original prologue.
    m.floats(0x3008034, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)
    m.uc.reg_write(UC_X86_REG_EBX, layer)
    m.run(0x72a473, 0x72a51e)
    transform = m.read(matrix + 0x20, 16)
    native_uv = [[u * transform[0] + transform[8], v * transform[5] + transform[9]] for u, v in base_uv]
    world_xy = [[origin[0] + x * size, origin[1] + y * size] for x, y in [(0, 0), (1, 0), (1, 1), (0, 1)]]
    direct_uv = [[x / dimensions[0], 1.0 - y / dimensions[1]] for x, y in world_xy]
    # The native block origin removes whole texture repeats using fmod. Under
    # a repeat sampler, the direct whole-map coordinates are equivalent.
    for actual, direct in zip(native_uv, direct_uv):
        for a, b in zip(actual, direct):
            assert abs((a - b) - round(a - b)) < 2e-5, (actual, direct, origin, dimensions)
    return dict(origin=origin, tile_size=size, texture_dimensions=dimensions,
                quad_positions=quad, source_quad_uv=base_uv, matrix=transform,
                native_uv=native_uv, world_xy=world_xy, direct_repeat_equivalent_uv=direct_uv,
                modulation=modulation)


def main():
    original = Pe(GAME / 'AA.exe', ORIGINAL_SHA)
    accepted = Pe(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', ACCEPTED_SHA)
    slices, assembly = {}, []
    cs = Cs(CS_ARCH_X86, CS_MODE_32)
    for name, (start, end) in RANGES.items():
        data = original.at(start, end - start)
        assert data == accepted.at(start, end - start)
        slices[name] = dict(start=hex(start), end_exclusive=hex(end), bytes=data.hex(), sha256=sha(data))
        assembly.append('\n' + name + '\n' + '\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in cs.disasm(data, start)))
    cases = [fixture(original, origin, 2, dims) for dims in [(8.0, 8.0), (16.0, 16.0), (16.0, 8.0), (32.0, 16.0), (3.5, 5.5)] for origin in [(0.0, 0.0), (2.0, 6.0), (16.0, 16.0), (254.0, 126.0), (-2.0, -6.0)]]
    map_sources = []
    metadata_path = ROOT / 'research/engine-baseline/terrain-variation-metadata.json'
    metadata = json.loads(metadata_path.read_text())
    for map_id in ['Campaign/tutorial01', 'Campaign/allies01_el_alamein', 'Campaign/allies06_normandy']:
        path = GAME / 'reconstruction/game/data/maps' / (map_id + '.json')
        source = json.loads(path.read_text())
        identities = []
        for palette_id, row in list(source['palette'].items())[:3]:
            entry = metadata['variations'].get(row['id'])
            if entry is None:
                continue
            tgi = GAME / entry['source']
            assert sha(tgi.read_bytes()) == entry['source_sha256']
            index = source['tiles'].index(int(palette_id))
            origin = [(index % source['width']) * source['cellSize'], (index // source['width']) * source['cellSize']]
            case = fixture(original, origin, source['cellSize'], [row['width'], row['height']])
            case['original_data'] = dict(map_id=map_id, palette_id=int(palette_id), variation=row['id'], tile_index=index,
                                         tgi_path=entry['source'], tgi_sha256=entry['source_sha256'])
            cases.append(case)
            identities.append(case['original_data'])
        map_sources.append(dict(path=str(path.relative_to(GAME)).replace('\\', '/'), sha256=sha(path.read_bytes()), cases=identities))
    report = dict(schema=1, passed=True, original_sha256=ORIGINAL_SHA, accepted_sha256=ACCEPTED_SHA,
                  script_sha256=sha(Path(__file__).read_bytes()), x87_control_word='0x037f',
                  case_count=len(cases), instruction_slices=slices, cases=cases, map_sources=map_sources,
                  scope='Actual native quad UV/color and texture-matrix instructions including CRT fmod; no GPU/native-renderer execution; repeated-coordinate equality tolerance2e-5, not x87 bit parity for Rust.')
    target = Path(__file__).parent
    (target / 'terrain-uv-native-fixtures.json').write_text(json.dumps(report, indent=2) + '\n')
    (target / 'terrain-uv-native-assembly.txt').write_text('\n'.join(assembly) + '\n')
    print(json.dumps({'passed': True, 'cases': len(cases), 'report_sha256': sha((target / 'terrain-uv-native-fixtures.json').read_bytes())}))


if __name__ == '__main__':
    main()
