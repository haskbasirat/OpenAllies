"""Unicorn oracle for 6FEBA0 (5CCF00 drag rotation about the camera target).

Runs the ORIGINAL 6FEBA0 with its actual 6FE3B0 rotation (870440/870480/
870590 matrix builders, B40054 process matrix from the real 9E3520 static
constructor), 870720 basis transform and 6FE470 zoom on a laboratory camera
allocation (all other bytes poisoned 0xA5). 6FE050 (NiCamera update, already
proven separately) is an explicit seam. FSIN/FCOS are transported from THIS
CPU's isolated x87 at the declared precision (research/skirmish-20261016-
settlement-completion/hardware_trig.py), never Unicorn's binary64 helpers,
exactly as native_camera_pose.py. Both original and v14 images, PC24/53/64.

  python native_camera_rotate.py [--write]
"""
from pathlib import Path
import sys, json, struct, hashlib, itertools
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/skirmish-20261017-native-camera'))
import native_camera_reset as R
from unicorn.x86_const import UC_X86_REG_EIP

f32 = R.f32
INPUTS = [0x208, 0x20c, 0x210, 0x240, 0x244, 0x248, 0x24c, 0x250, 0x254, 0x2b8, 0x2bc]
OUTPUTS = list(range(0x38, 0x5c, 4)) + [0x5c, 0x60, 0x64, 0x208, 0x234, 0x238, 0x23c, 0x24c, 0x250, 0x254,
                                       0x2b8, 0x2bc]


class Machine(R.Machine):
    def __init__(self, image, pc, hw):
        super().__init__(image, pc, hw)
        self.w(0xb4a998, 1)
        self.call(0x9e3520)
        self.matrix = [self.r(0xb40054 + 4 * i) for i in range(9)]

    def _hook(self, u, pc, size, data):
        if R.original.base <= pc < R.original.base + R.original.image_size:
            op = bytes(u.mem_read(pc, size))
            if op in (b'\xd9\xfe', b'\xd9\xff'):
                name = {b'\xd9\xfe': 'fsin', b'\xd9\xff': 'fcos'}[op]
                v = self.st0_live()
                raw = struct.pack('<QH', v['extended_significand'], v['extended_sign_exponent'])
                out = self.hardware.trig(name, R.PRECISIONS[self.precision], raw)
                assert out['status_word'] & (1 | 4 | 8 | 0x400) == 0
                self.trans.append(dict(pc=hex(pc), op=name, input80=raw.hex(), output80=out['raw80']))
                self.actual[pc] = op.hex()
                p = self.alloc(16)
                u.mem_write(p, bytes.fromhex(out['raw80']))
                code = b'\xdd\xd8' + b'\xdb\x2d' + struct.pack('<I', p) + b'\x68' + struct.pack('<I', pc + size) + b'\xc3'
                stub = self.alloc(len(code))
                u.mem_write(stub, code)
                u.reg_write(UC_X86_REG_EIP, stub)
                return
        super()._hook(u, pc, size, data)


def cases():
    base = {0x208: f32(1.0), 0x20c: f32(0.7), 0x210: f32(2.1), 0x240: f32(36.2), 0x244: f32(-8.4),
            0x248: f32(77.3), 0x24c: f32(64.5), 0x250: f32(32.25), 0x254: f32(12.5),
            0x2b8: f32(0.6981317), 0x2bc: f32(-0.7853982)}
    for yaw, pitch in [(-0.7853982 - 0.0872665, 0.6981317), (-0.7853982 + 0.25, 0.6981317), (2.5, 0.6981317),
                       (-3.0, 0.9), (0.0, 0.6981317), (-0.7853982, 0.6981317), (1.0, 0.3), (-1.7, 1.4)]:
        yield dict(base, yaw=f32(yaw), pitch=f32(pitch))
    yield dict(base, yaw=f32(0.5), pitch=f32(0.6981317))
    flat = dict(base)
    flat[0x248] = flat[0x254]
    yield dict(flat, yaw=f32(0.3), pitch=f32(0.6981317))
    for zoom in (0.2, 3.0, 1.5):
        c = dict(base)
        c[0x208] = f32(zoom)
        yield dict(c, yaw=f32(-0.6), pitch=f32(0.7))
    locked = dict(base, yaw=f32(0.4), pitch=f32(0.6981317))
    locked['locked'] = 1
    yield locked


def run(image, pc, case, hw):
    m = Machine(image, pc, hw)
    cam = m.alloc(0x394)
    m.u.mem_write(cam, b'\xa5' * 0x394)
    for off in INPUTS:
        m.w(cam + off, case[off])
    m.wb(cam + 0x214, case.get('locked', 0))
    # Matrix/rotation state that 6FE3B0 compares and may keep: start from a
    # real native rotation by first running 6FE3B0 with the initial angles.
    m.seam(0x6fe050, lambda x: (x.event('ni_update'), x.ret(0)))
    m.wb(cam + 0x214, 0)
    m.w(cam + 0x2b8, f32(-9.0))
    m.call(0x6fe3b0, cam, [case[0x2bc], case[0x2b8]])
    m.wb(cam + 0x214, case.get('locked', 0))
    before = {hex(o): m.r(cam + o) for o in OUTPUTS}
    m.trace = []
    m.call(0x6feba0, cam, [case['yaw'], case['pitch']])
    after = {hex(o): m.r(cam + o) for o in OUTPUTS}
    return dict(before=before, after=after, trace=m.trace, matrix_b40054=m.matrix), m.actual, m.trans


def main():
    hw = R.Hardware()
    rows, pins, trans = [], {}, []
    try:
        for image, pc in itertools.product(('original', 'v14'), R.PRECISIONS):
            for i, c in enumerate(cases()):
                out, visited, transport = run(image, pc, c, hw)
                pins.update(visited)
                trans.extend(transport)
                rows.append(dict(image=image, precision=pc, case=i,
                                 input={(hex(k) if isinstance(k, int) else k): v for k, v in c.items()}, output=out))
        for a, b in pins.items():
            assert R.original.range(a, len(bytes.fromhex(b))).hex() == R.modified.range(a, len(bytes.fromhex(b))).hex(), hex(a)
        doc = dict(scope=__doc__.strip().splitlines()[0], producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                   rows=rows, instructions={hex(a): b for a, b in sorted(pins.items())}, hardware_transports=len(trans))
        path = HERE / 'native-camera-rotate.json'
        if '--write' in sys.argv:
            path.write_text(json.dumps(doc, indent=1) + '\n')
        else:
            assert json.loads(path.read_text()) == doc
        print(json.dumps(dict(rows=len(rows), pins=len(pins), transports=len(trans),
                              sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
    finally:
        hw.close()


if __name__ == '__main__':
    main()
