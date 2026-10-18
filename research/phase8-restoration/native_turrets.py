"""Original ActorData Turret definition loader (constructor 4C7660 + LoadData 4C7820).

Runs the unmodified x86 of AA.exe under Unicorn. Only the TGI node getters
(5B54D0 string, 5B5130 fixed, 5B5380 flag, 5B4D80 point2), the Bone registry
lookup 42C070 (b36530+580, "Bone") and the throwing data-error reporter 410C40
are seams; CString copy/refcount, every comparison, multiply, x87 store and the
duplicate-bone walk over ActorData+1F8/+1FC execute natively.

FCOS note: Unicorn (QEMU) implements FCOS through host binary64 cos(); the
+10 field is therefore emulator-observed. The Rust port computes the value
in exact fixed point and refuses binary32 rounding-boundary ambiguity; a host
x87 FCOS cross-check lives in crates/aa-rules/tests/native_turrets.rs.
"""
from pathlib import Path
import sys, json, struct, hashlib, itertools
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase5-actors'))
from original_creation import Machine as Base, P, V, H
from unicorn.x86_const import *

def bits(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]

NAN = 0x7fc00000
FIELDS = ['yaw_rotation_per_sec_degrees', 'pitch_rotation_per_sec_degrees', 'yaw_rotation_range_degrees',
          'pitch_rotation_range_start_degrees', 'pitch_rotation_range_end_degrees', 'yaw_start_degrees',
          'pitch_start_degrees', 'reset_time']
ERRORS = {
    0xa25120: 'bone_unrecognized',
    0xa250d0: 'duplicate_bone',
    0xa24fd0: 'yaw_rate_range',
    0xa24ec8: 'pitch_rate_range',
    0xa24dd8: 'yaw_range_range',
    0xa24cc0: 'pitch_range_start_range',
    0xa24bb0: 'pitch_range_end_range',
    0xa24aa8: 'pitch_range_end_before_start',
    0xa249e0: 'yaw_start_range',
    0xa24880: 'pitch_start_range',
}
FLD_STUB = 0x301e000
FLOAT_SLOT = 0x225f000
BONES = {'turret': 0x0bad0100, 'turret2': 0x0bad0200, 'turret3': 0x0bad0300, 'Turret': 0x0bad0100}


class Thrown(Exception):
    pass


class Machine(Base):
    def __init__(self, pc):
        super().__init__(pc)
        # fld dword [FLOAT_SLOT]; ret 4 -- returns a fixed field in ST0.
        self.u.mem_write(FLD_STUB, b'\xd9\x05' + struct.pack('<I', FLOAT_SLOT) + b'\xc2\x04\x00')
        manager = self.alloc(0x600)
        self.w(0xb36530, manager)
        self.w(manager + 0x580, self.alloc(0x40))
        self.values = {}
        self.reads = []
        self.error = None
        sys.path.insert(0, str(HERE))
        import null_guard
        null_guard.install(self)

    def nameat(self, p):
        return P.range(p, 200).decode('utf-16-le').split('\0')[0] if P.base <= p < P.base + P.image_size else self.text(p)

    def hook(self, u, pc, n, z):
        if pc == 0x5b54d0:
            name = self.nameat(self.arg())
            self.reads.append(name)
            value = self.values[name]
            p = self.string([ord(c) for c in value])
            r = self.alloc(4)
            self.w(r, p)
            self.ret(r, pop=4)
        elif pc == 0x5b5130:
            name = self.nameat(self.arg())
            self.reads.append(name)
            self.w(FLOAT_SLOT, self.values[name])
            u.reg_write(UC_X86_REG_EIP, FLD_STUB)
        elif pc == 0x5b5380:
            name = self.nameat(self.arg())
            self.reads.append(name)
            self.ret(int(self.values[name]), pop=4)
        elif pc == 0x5b4d80:
            out, name = self.arg(), self.nameat(self.arg(1))
            self.reads.append(name)
            self.w(out, self.values[name][0])
            self.w(out + 4, self.values[name][1])
            self.ret(out, pop=8)
        elif pc == 0x42c070:
            name = self.text(self.arg())
            self.reads.append('bone:' + name)
            # Seam: registry resolution of a Bone IDS (case-insensitive in the
            # Rust port's resolved_definition); 0 for empty or unknown.
            self.ret(BONES.get(name, 0), pop=4)
        elif pc == 0x410c40:
            self.error = ERRORS.get(self.arg(), hex(self.arg()))
            u.emu_stop()
        else:
            super().hook(u, pc, n, z)

    def load(self, values, existing):
        self.values = values
        turret = self.alloc(0x40)
        for o in range(0, 0x34, 4):
            self.w(turret + o, 0xcdcdcdcd)
        self.invoke(0x4c7660, turret, [])
        actor = self.alloc(0x220)
        name = self.string([ord(c) for c in 'test_actor'])
        self.w(actor + 8, name)
        array = self.alloc(4 * max(1, len(existing)))
        for i, bone in enumerate(existing):
            other = self.alloc(0x40)
            self.w(other, BONES[bone])
            self.w(array + 4 * i, other)
        self.w(actor + 0x1f8, array)
        self.w(actor + 0x1fc, len(existing))
        node = self.alloc(0x58)
        self.error = None
        self.w(self.stack, self.stop)
        self.w(self.stack + 4, node)
        self.w(self.stack + 8, actor)
        for reg, val in [(UC_X86_REG_ESP, self.stack), (UC_X86_REG_ECX, turret), (UC_X86_REG_EBX, 0x11223344),
                         (UC_X86_REG_ESI, 0x22334455), (UC_X86_REG_EDI, 0x33445566), (UC_X86_REG_EBP, 0x44556677)]:
            self.u.reg_write(reg, val)
        self.u.emu_start(0x4c7820, self.stop, count=400000)
        words = [self.r(turret + o) for o in range(0, 0x30, 4)]
        bytes_ = list(self.u.mem_read(turret + 0x30, 2))
        if self.error is None:
            assert self.u.reg_read(UC_X86_REG_EIP) == self.stop
            assert self.u.reg_read(UC_X86_REG_ESP) == self.stack + 12
            for reg, val in [(UC_X86_REG_EBX, 0x11223344), (UC_X86_REG_ESI, 0x22334455), (UC_X86_REG_EDI, 0x33445566), (UC_X86_REG_EBP, 0x44556677)]:
                assert self.u.reg_read(reg) == val
        assert not self.null_accesses, self.null_accesses
        return {'error': self.error, 'reads': self.reads, 'words': words, 'bytes': bytes_}


def base_values(**over):
    v = {'turret_ids': 'turret', 'yaw_rotation_per_sec_degrees': bits(360), 'pitch_rotation_per_sec_degrees': bits(360),
         'yaw_rotation_range_degrees': bits(360), 'pitch_rotation_range_start_degrees': 0,
         'pitch_rotation_range_end_degrees': 0, 'yaw_start_degrees': 0, 'pitch_start_degrees': 0,
         'offset': [0, 0], 'reset_time': bits(10), 'can_reset': True, 'can_preaim': True}
    v.update(over)
    return v


def cases():
    rows = []
    # Actual authored source tuples (template defaults plus overrides found in
    # Data/*.tgi Turret nodes) and boundary sweeps of every validated field.
    for yaw_rate in [0, 0x80000000, 1, bits(1e-30), bits(0.1), bits(30), bits(45), bits(60), bits(75), bits(90),
                     bits(120), bits(180), bits(359.99997), bits(360), bits(360.00003), bits(-1e-30), bits(-1), NAN, 0x7f800000, 0xff800000]:
        rows.append(('yaw_rate', base_values(yaw_rotation_per_sec_degrees=yaw_rate), []))
    for pitch_rate in [0, bits(15), bits(360), bits(360.00003), bits(-0.5), NAN]:
        rows.append(('pitch_rate', base_values(pitch_rotation_per_sec_degrees=pitch_rate), []))
    for yaw_range in [0, 0x80000000, 1, bits(1e-30), bits(1), bits(10), bits(30), bits(45), bits(60), bits(90), bits(100), bits(120),
                      bits(135), bits(150), bits(160), bits(170), bits(180), bits(200), bits(210), bits(240), bits(270), bits(300),
                      bits(330), bits(355.5), bits(359), bits(359.99997), bits(360), bits(360.00003), bits(-1), NAN]:
        rows.append(('yaw_range', base_values(yaw_rotation_range_degrees=yaw_range), []))
    # Dense integer and fractional sweep of the FCOS input domain.
    for tenth in range(0, 3601, 7):
        rows.append(('yaw_range_sweep', base_values(yaw_rotation_range_degrees=bits(tenth / 10)), []))
    for start, end, pstart in [(0, 0, 0), (bits(-90), bits(90), 0), (bits(-10), bits(45), bits(5)), (bits(-90), bits(-90), bits(-90)),
                               (bits(90), bits(90), bits(90)), (bits(-90.00001), 0, 0), (0, bits(90.00001), 0),
                               (bits(10), bits(5), bits(7)), (bits(-5), bits(30), bits(-5.0001)), (bits(-5), bits(30), bits(30.001)),
                               (bits(-5), bits(30), bits(30)), (bits(-5), bits(30), bits(-5)), (0x80000000, 0, 0x80000000),
                               (NAN, bits(10), 0), (bits(-10), NAN, 0), (bits(-10), bits(10), NAN), (bits(1e-40), bits(2e-40), bits(1.5e-40))]:
        rows.append(('pitch', base_values(pitch_rotation_range_start_degrees=start, pitch_rotation_range_end_degrees=end,
                                          pitch_start_degrees=pstart), []))
    for yaw_start in [0, 0x80000000, bits(1e-40), bits(45), bits(90), bits(180), bits(270), bits(315), bits(359.99997), bits(360),
                      bits(360.00003), bits(-0.001), NAN]:
        rows.append(('yaw_start', base_values(yaw_start_degrees=yaw_start), []))
    for offset in [[0, 0], [bits(-3.801), bits(3.472)], [bits(1.5), 0x80000000], [NAN, 0xffffffff]]:
        for reset in [0, bits(10), bits(-2.5), NAN]:
            for flags in [(True, True), (False, True), (True, False), (False, False)]:
                rows.append(('tail', base_values(offset=offset, reset_time=reset, can_reset=flags[0], can_preaim=flags[1]), []))
    for bone, existing in [('turret', []), ('Turret', []), ('turret2', ['turret']), ('turret', ['turret2', 'turret3']),
                           ('turret', ['turret']), ('turret3', ['turret', 'turret2', 'turret3']), ('', []), ('missing_bone', []),
                           ('missing_bone', ['turret'])]:
        rows.append(('bone', base_values(turret_ids=bone), existing))
    return rows


def main():
    out = []
    visited = {}
    for precision, cw in [('Single24', 0x7f), ('Double53', 0x27f), ('Extended64', 0x37f)]:
        for group, values, existing in cases():
            m = Machine(cw)
            native = m.load(values, existing)
            visited.update(m.visited)
            out.append({'group': group, 'precision': precision,
                        'input': {k: v for k, v in values.items()}, 'existing_bones': existing, 'native': native})
    spans = []
    for a, n in sorted(visited.items()):
        if not P.base <= a < P.base + P.image_size:
            continue
        if spans and spans[-1][1] >= a:
            spans[-1][1] = max(spans[-1][1], a + n)
        else:
            spans.append([a, a + n])
    pins = []
    for a, b in spans:
        raw = P.range(a, b - a)
        assert raw == V.range(a, b - a), hex(a)
        pins.append({'start': hex(a), 'end': hex(b), 'sha256': H(raw)})
    doc = {
        'schema': 1, 'passed': True,
        'producer_sha256': H(Path(__file__).read_bytes()),
        'original_sha256': H(P.b), 'v14_sha256': H(V.b),
        'functions': {'4c7660': 'Turret definition constructor', '4c7820': 'Turret LoadData(node, ActorData)'},
        'layout': {'0x00': 'Bone definition pointer (turret_ids)', '0x04': 'yaw_start radians', '0x08': 'yaw rate radians/s',
                   '0x0c': 'half yaw range radians', '0x10': 'FCOS(half yaw range, unrounded)', '0x14': 'pitch_start radians',
                   '0x18': 'pitch rate radians/s', '0x1c': 'pitch range start radians', '0x20': 'pitch range end radians',
                   '0x24': 'offset x raw', '0x28': 'offset y raw', '0x2c': 'reset_time', '0x30': 'can_reset', '0x31': 'can_preaim'},
        'seams': ['TGI getters return supplied typed values; a missing field throws natively (not exercised).',
                  'Bone registry 42C070 returns a fixed pointer per IDS; 0 for empty/unknown. Duplicate detection compares these pointers natively.',
                  'Data-error reporter 410C40 throws; the harness records which format string was raised and stops.',
                  'Unicorn FCOS uses host binary64 cos(); +10 is emulator-observed, cross-checked against host x87 in Rust tests.'],
        'cases': out, 'native_windows': pins,
    }
    path = HERE / 'native-turrets.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'errors': sum(1 for c in out if c['native']['error']), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
