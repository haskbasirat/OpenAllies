"""Unicorn oracle for the KKC_UI drag-box screen rectangle on the camera.

Runs the ORIGINAL AA.exe code at x87 PC24/PC53/PC64 on laboratory inputs:
  5CCA80  box vertices from the UI+58 tracker and B396D0/B396D4
  5CD850  box state enter: +214 lock, 887F40 polygon, 5CCA80, the three
          installed properties (A44EFC, A4D92C, 87D280) and 5CD7E0 ->
          5CD730/5CCFE0/882300 on Camera+168 (seams: 5E0780 state push,
          6F7810 operator new)
  5CCA30  box state exit: +214 unlock, 874350/882300 detach, release +4
          (seam: the polygon's AB990C destructor), VWorld+13C vt+44 tail
  5CCD60/5CCDA0  facing state camera lock/unlock (seams: 5E0780, the
          arrow's vt+2C)
Every executed instruction of the image is checked byte-equal in the v14
image (shared with native_camera_input.py's Machine).

  python native_camera_screen.py           verify the frozen fixture
  python native_camera_screen.py --write   regenerate it
"""
from pathlib import Path
import sys, json, struct, hashlib, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_camera_input as base
from native_camera_input import Machine, CW

MARK = 0x5a5a0000
VIEWPORTS = [(800, 600), (1024, 768), (1280, 720), (1366, 768), (1600, 900), (1920, 1080), (2560, 1440),
             (3, 7), (1, 1), (16777217, 16777219)]
POINTS = [(0, 0), (799, 599), (-5, 10), (400, 300), (123, 456), (1000, -1), (65535, 65535),
          (-32768, 32767), (2147483647, -2147483648), (7, 7), (1919, 1079), (333, 1)]


def s32(v):
    return v - (1 << 32) if v & 0x80000000 else v


def ui_tracker(m, current, press, viewport):
    ui = m.alloc(0x100)
    m.w(0xb3653c, ui)
    t = m.alloc(0x20)
    m.w(ui + 0x58, t)
    m.w(t + 4, current[0])
    m.w(t + 8, current[1])
    m.w(t + 0xc, press[0])
    m.w(t + 0x10, press[1])
    m.w(0xb396d0, viewport[0])
    m.w(0xb396d4, viewport[1])
    return t


def rows_5cca80(pc):
    rng = random.Random(0x5cca80)
    rows = []
    for vp in VIEWPORTS:
        pairs = [(POINTS[i], POINTS[j]) for i in range(len(POINTS)) for j in range(len(POINTS))]
        for current, press in rng.sample(pairs, 12):
            m = Machine(pc)
            ui_tracker(m, current, press, vp)
            poly = m.alloc(0x30)
            verts = m.alloc(0x30)
            for i in range(12):
                m.w(verts + 4 * i, MARK + i)
            m.w(poly + 0x14, verts)
            m.w(poly + 0x20, 0x4440)
            this = m.alloc(0x10)
            m.w(this + 4, poly)
            m.call(0x5cca80, this, [])
            rows.append(dict(precision=pc, viewport=list(vp), current=list(current), press=list(press),
                             vertices=[m.r(verts + 4 * i) for i in range(12)], word_20=m.r(poly + 0x20)))
    return rows


class Box:
    """Laboratory UI, camera and allocator for the box/facing states."""

    def __init__(self, pc, list_slots, seeds, current, press, viewport):
        m = self.m = Machine(pc)
        self.allocs, self.pushes, self.destroyed, self.vt44, self.arrow = [], 0, [], 0, []

        def new(x):
            n = x.arg(0)
            p = x.alloc(n)
            x.u.mem_write(p, b'\xcd' * n)
            self.allocs.append(n)
            x.ret(0, p)
        m.seam(0x6f7810, new)

        def push(x):
            self.pushes += 1
            x.ret(0)
        m.seam(0x5e0780, push)
        self.freed = []

        def free(x):
            self.freed.append(x.arg(0))
            x.ret(0)
        m.seam(0x6f7430, free)

        def destroy(x):
            self.destroyed.append(x.u.reg_read(base.UC_X86_REG_ECX))
            x.ret(4)
        m.seam(m.r(0xab990c), destroy)
        ui_tracker(m, current, press, viewport)
        for i, a in enumerate([0xb43cf8, 0xb43cfc, 0xb43d00]):
            m.w(a, seeds[i])
        self.camera = self.new_camera(list_slots)
        cell = m.alloc(4)
        m.w(cell, self.camera)
        m.w(0xb40048, cell)
        vworld = m.alloc(0x200)
        m.w(0xb36538, vworld)
        pick = m.alloc(8)
        pvt = m.alloc(0x50)
        m.w(pick, pvt)

        def vt44(x):
            self.vt44 += 1
            x.ret(0)
        m.w(pvt + 0x44, m.stub(vt44))
        m.w(vworld + 0x13c, pick)
        self.this = m.alloc(0x40)
        m.w(self.this + 4, 0)

    def new_camera(self, list_slots):
        m = self.m
        cam = m.alloc(0x400)
        m.wb(cam + 0x214, 0)
        cap = max(5, len(list_slots))
        # NiCamera 874BA6..874BC9: a count-prefixed array (new[] of 4DD140).
        block = m.alloc(4 * cap + 4)
        m.w(block, cap)
        data = block + 4
        self.dummies = []
        for i, used in enumerate(list_slots):
            if used:
                d = m.alloc(0x30)
                m.w(d, 0xab990c)
                m.w(d + 4, 1)
                m.w(data + 4 * i, d)
                self.dummies.append(d)
        m.w(cam + 0x168, 0xab7738)
        m.w(cam + 0x16c, data)
        m.w(cam + 0x170, cap if len(list_slots) < 5 else len(list_slots))
        m.w(cam + 0x174, len(list_slots))
        m.w(cam + 0x178, sum(1 for u in list_slots if u))
        m.w(cam + 0x17c, 5)
        return cam

    def list_state(self, cam, poly):
        m = self.m
        data, size = m.r(cam + 0x16c), m.r(cam + 0x174)
        slots = []
        for i in range(size):
            p = m.r(data + 4 * i)
            slots.append('box' if p == poly and p else ('dummy' if p else None))
        return dict(size=size, count=m.r(cam + 0x178), capacity=m.r(cam + 0x170), slots=slots,
                    lock_214=m.b(cam + 0x214))

    def polygon(self, poly):
        m = self.m
        state = m.r(poly + 0xc)
        slots = []
        for i in range(12):
            p = m.r(state + 8 + 4 * i)
            if p == 0:
                slots.append(None)
                continue
            vt = m.r(p)
            words = {f'{o:#x}': m.r(p + o) for o in range(8, {0xa44efc: 0x2c, 0xa4d92c: 0x24, 0xab91e0: 0x64}[vt], 4)}
            slots.append(dict(vtable=vt, refcount=m.r(p + 4), words=words))
        return dict(vtable=m.r(poly), refcount=m.r(poly + 4), count=m.r(poly + 0x10) & 0xffff,
                    vertices=[m.r(m.r(poly + 0x14) + 4 * i) for i in range(12)], uv=m.r(poly + 0x18),
                    colors=[m.r(m.r(poly + 0x1c) + 4 * i) for i in range(16)], flags_20=m.r(poly + 0x20) & 0xffff,
                    word_24=m.r(poly + 0x24), word_28=m.r(poly + 0x28), state_refcount=m.r(state + 4), slots=slots)


LISTS = [[], [True], [True, False, True], [True] * 5, [False, False], [True] * 7, [True, True, False]]


def rows_box(pc):
    rng = random.Random(0x5cd850)
    rows = []
    for k, slots in enumerate(LISTS):
        vp = VIEWPORTS[k % len(VIEWPORTS)]
        current, press = rng.choice(POINTS), rng.choice(POINTS)
        seeds = [rng.getrandbits(32) & 0x7f7fffff for _ in range(3)]
        b = Box(pc, slots, seeds, current, press, vp)
        m = b.m
        event = m.alloc(0x30)
        raw = bytes(rng.getrandbits(8) for _ in range(0x30))
        m.u.mem_write(event, raw)
        material_before = m.r(0xb323c0)
        m.call(0x5cd850, b.this, [event], pop=4)
        poly = m.r(b.this + 4)
        enter = dict(pushes=b.pushes, allocs=b.allocs[:], event_copied=bytes(m.u.mem_read(b.this + 8, 0x30)).hex(),
                     event=raw.hex(), material_before=material_before, material_after=m.r(0xb323c0),
                     camera=b.list_state(b.camera, poly), polygon=b.polygon(poly))
        # Exit on the SAME camera, or after *B40048 moved to another camera.
        moved = k % 3 == 2
        if moved:
            other = b.new_camera([True])
            m.w(m.r(0xb40048), other)
        b.allocs.clear()
        m.call(0x5cca30, b.this, [])
        exit_ = dict(this_4=m.r(b.this + 4), destroyed=[d == poly for d in b.destroyed], vt44=b.vt44,
                     original_camera=b.list_state(b.camera, poly), refcount=m.r(poly + 4))
        if moved:
            exit_['current_camera'] = b.list_state(m.r(m.r(0xb40048)), poly)
        rows.append(dict(precision=pc, viewport=list(vp), current=list(current), press=list(press), seeds=seeds,
                         list=slots, moved_current_camera=moved, enter=enter, exit=exit_))
    return rows


def rows_facing(pc):
    rows = []
    for initial in [0, 1]:
        b = Box(pc, [], [0, 0, 0], (0, 0), (0, 0), (800, 600))
        m = b.m
        m.wb(b.camera + 0x214, initial)
        state = m.alloc(0x40)
        for o in range(0, 0x40, 4):
            m.w(state + o, 0x77777777)
        # +18 is an embedded object: its first DWORD is the vtable.
        avt = m.alloc(0x40)

        def show(x):
            b.arrow.append([x.arg(0), x.u.reg_read(base.UC_X86_REG_ECX) == state + 0x18])
            x.ret(4)
        m.w(avt + 0x2c, m.stub(show))
        m.w(state + 0x18, avt)
        m.call(0x5ccd60, state, [])
        enter = dict(pushes=b.pushes, arrow=b.arrow[:], lock_214=m.b(b.camera + 0x214), byte_8=m.b(state + 8),
                     words=[m.r(state + o) for o in (0xc, 0x10, 0x14)])
        b.arrow.clear()
        m.call(0x5ccda0, state, [])
        rows.append(dict(precision=pc, initial_214=initial, enter=enter,
                         exit=dict(arrow=b.arrow[:], lock_214=m.b(b.camera + 0x214))))
    return rows


def main():
    doc = dict(scope=__doc__.strip().splitlines()[0], sections={})
    for name, fn in [('update_5cca80', rows_5cca80), ('box_5cd850_5cca30', rows_box), ('facing_5ccd60_5ccda0', rows_facing)]:
        rows = []
        for pc in CW:
            rows += fn(pc)
        doc['sections'][name] = rows
    doc['instructions'] = {f'{a:08x}': base.PINS[a] for a in sorted(base.PINS)}
    doc['harness_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    out = HERE / 'native-camera-screen.json'
    text = json.dumps(doc, indent=1) + '\n'
    if '--write' in sys.argv:
        out.write_text(text)
    else:
        assert json.loads(out.read_text()) == json.loads(text), 'fixture differs from fresh native execution'
    print(json.dumps({k: len(v) for k, v in doc['sections'].items()} | dict(pins=len(base.PINS),
          sha256=hashlib.sha256(out.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
