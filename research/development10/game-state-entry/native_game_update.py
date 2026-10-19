"""Original/v14 Game interface per-frame update 612010 (vtable A5B0B0 +0C).

Unchanged code runs on a laboratory Game interface whose fields hold the
widgets Init 6182B0 stores there (TimeMessage +190, GroupFloater +144,
FactionAbilitiesFloater +148, DeveloperModeLabel +170, CameraInformationLabel
+174, ControlPanel +138 with ButtonSupplyBorder +A0 / ButtonChat +A8,
DisplayBannerPanel +178 / Background +17C / Banner +180, GameMenuDialog +154,
DisplayPause +184, GameChatPanel +198, UpkeepShortageIcon +128). Real code:
612010, 5EFB00, 5F0910/5F0940/5F01B0/5F0210, 40CEB0, 40D8E0. Seams (recorded,
not executed): vtable +D0 AddChild, 5EFB60 RemoveChild, 5EF100, 60BA50,
5C0E30, 5C06B0, 40EAA0, the camera-text children 856370/401B00/4B9F40/405D60
and label +C8, 427E60, 59ABA0/59AB60 timers, colour get/set 5EE520/5EE600/
5EDFF0/5EDFE0, 5DC780/5C3EB0 selection events, 5C1760, 60B9D0.
Application-state/HUD evidence only; no game file is changed.
"""
from pathlib import Path
import sys, json, hashlib, random, struct
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import Machine as Base, original, modified, PRECISIONS
from unicorn.x86_const import UC_X86_REG_EIP

APP = 0xB39690
UI_GLOBAL, VWORLD, GWORLD, SIM, B3971C = 0xB3653C, 0xB36538, 0xB36534, 0xB36554, 0xB3971C
UVARS, AVARS, CAMERA, TEXT = 0xB39940, 0xB39990, 0xB40048, 0xB36544
f32 = lambda f: struct.unpack('<I', struct.pack('<f', f))[0]
FIELDS = {0x190: 'TimeMessage', 0x144: 'GroupFloater', 0x148: 'FactionAbilitiesFloater', 0x170: 'DeveloperModeLabel',
          0x174: 'CameraInformationLabel', 0x178: 'DisplayBannerPanel', 0x17c: 'DisplayBannerBackground',
          0x180: 'DisplayBanner', 0x154: 'GameMenuDialog', 0x184: 'DisplayPause', 0x198: 'GameChatPanel',
          0x128: 'UpkeepShortageIcon', 0x138: 'ControlPanel'}


class Machine(Base):
    def __init__(self, image, precision):
        super().__init__(precision); self.image = image
        if image == 'v14':
            old = (original.image_size + 0xfff) & ~0xfff; new = (modified.image_size + 0xfff) & ~0xfff
            if new > old: self.u.mem_map(original.base + old, new - old)
            for vs, rva, raw, off in modified.sections: self.u.mem_write(modified.base + rva, modified.b[off:off + raw])
        self.trace = []; self.actual = {}

    def _hook(self, u, pc, size, data):
        super()._hook(u, pc, size, data)
        if pc not in self.seams and u.reg_read(UC_X86_REG_EIP) == pc and original.base <= pc < original.base + original.image_size:
            self.actual[pc] = bytes(u.mem_read(pc, size)).hex()

    def event(self, *values):
        self.trace.append(list(values))


def cstring(m, text=''):
    """A refcounted CString payload whose count never reaches zero here."""
    a = m.alloc(64); m.w(a + 0xc, 1000); return a + 16


def run(m, c):
    m.seams = {}; m.observers = {}; m.trace = []; m.actual = {}
    this = m.alloc(0x200)
    names = {}
    widgets = {}
    for off, name in FIELDS.items():
        if off == 0x154 and not c['game_menu']:
            m.w(this + off, 0); continue
        if off == 0x170 and not c['dev_label']:
            m.w(this + off, 0); continue
        w = m.alloc(0x100); m.w(this + off, w); names[w] = name; widgets[name] = w
    for name, key in [('TimeMessage', 'time_flags'), ('GroupFloater', 'group_flags'), ('FactionAbilitiesFloater', 'ability_flags'),
                      ('DisplayBannerPanel', 'banner_flags'), ('GameMenuDialog', 'menu_flags'), ('DisplayPause', 'pause_flags'),
                      ('GameChatPanel', 'chat_flags'), ('UpkeepShortageIcon', 'upkeep_flags'), ('DeveloperModeLabel', 'dev_flags'),
                      ('CameraInformationLabel', 'camera_flags')]:
        if name in widgets: m.w(widgets[name] + 0x24, c[key])
    m.w(widgets['TimeMessage'] + 8, this if c['time_parent_is_this'] else m.alloc(16))
    for name, key, fkey in [('ButtonSupplyBorder', 'supply_button_flags', 'supply_functor'), ('ButtonChat', 'chat_button_flags', 'chat_functor')]:
        b = m.alloc(0x100); names[b] = name; widgets[name] = b
        m.w(b + 0x24, c[key]); m.w(b + 0xe4, 0)
        functor = m.alloc(8) if c[fkey] else 0
        m.w(b + 0xe0, functor)
        if functor: names[functor] = name + '+E0'
    m.w(widgets['ControlPanel'] + 0xa0, widgets['ButtonSupplyBorder']); m.w(widgets['ControlPanel'] + 0xa8, widgets['ButtonChat'])
    vt = m.alloc(0x200)
    m.w(this, vt)
    m.w(vt + 0xd0, m.stub(lambda x: (x.event('add_child_d0', names.get(x.arg(1), hex(x.arg(1)))), x.ret(0, 4))))
    label_vt = m.alloc(0x200); m.w(widgets['CameraInformationLabel'], label_vt)
    m.w(label_vt + 0xc8, m.stub(lambda x: (x.event('camera_label_set_text_c8'), x.ret(0, 4))))
    for off in (0x19c, 0x1a0, 0x1a4):
        m.w(this + off, c['banner'][{0x19c: 0, 0x1a0: 1, 0x1a4: 2}[off]])
    m.wb(this + 0x1a8, c['banner_prev_paused'])
    # application table for the real 40CEB0/40D8E0
    node7 = m.alloc(16); other = m.alloc(16)
    m.w(APP + 0xcc + 7 * 4, node7); m.w(APP + 0x10c, 7 if c['node7_current'] else 4); m.w(APP + 0xcc + 4 * 4, other)
    uv = m.alloc(0x400); m.w(UVARS, uv)
    for off, key in [(0x8c, 'uvar_time'), (0xb0, 'uvar_camera'), (0x270, 'uvar_supply')]:
        p = m.alloc(4); m.wb(p, c[key]); m.w(uv + off, p)
    av = m.alloc(0x400); m.w(AVARS, av); p = m.alloc(4); m.w(p, c['fade_ms']); m.w(av + 0x108, p)
    sim = m.alloc(0x40); m.w(SIM, sim); m.wb(sim + 0x28, c['paused'])
    world = m.alloc(0x300); m.w(GWORLD, world); m.w(world + 0x3c, 0x11111111); m.w(world + 0x40, 0x22222222)
    m.w(UI_GLOBAL, m.alloc(0x200)); m.w(VWORLD, m.alloc(0x200)); m.w(B3971C, m.alloc(16))
    cam = m.alloc(0x300); camp = m.alloc(4); m.w(camp, cam); m.w(CAMERA, camp)
    for off in (0x208, 0x2b8, 0x2bc, 0x24c, 0x250, 0x254): m.w(cam + off, f32(off / 16.0))
    kingdom = 0
    if c['upkeep_kingdom']:
        kingdom = m.alloc(0x300); m.w(kingdom + 0x1f0, c['shortage_ratio'])

    m.seam(0x5efb60, lambda x: (x.event('remove_child_5efb60', names.get(x.arg(1), hex(x.arg(1))), x.arg(2)), x.ret(0, 8)))
    m.seam(0x5ef100, lambda x: (x.event('visible', names.get(x.ecx(), hex(x.ecx())), x.r(x.ecx() + 0x24) & 1), x.ret()))
    m.seam(0x60ba50, lambda x: (x.event('update_60ba50'), x.ret()))
    m.seam(0x5c0e30, lambda x: (x.event('local_kingdom_5c0e30'), x.ret(0x5150 if c['local_kingdom'] else 0)))
    m.seam(0x5c06b0, lambda x: (x.event('base_update_5c06b0'), x.ret()))
    m.seam(0x40eaa0, lambda x: (x.event('dev_mode_40eaa0'), x.ret(c['dev_mode'])))
    m.seam(0x427e60, lambda x: (x.event('update_427e60'), x.ret()))
    m.seam(0x60b9d0, lambda x: (x.event('update_60b9d0'), x.ret()))
    m.seam(0x5c1760, lambda x: (x.event('local_kingdom_5c1760'), x.ret(kingdom)))
    def timer(x):
        if x.ecx() == 0xb396e0:
            x.event('timer_b396e0'); x.ret(c['now'])
        elif x.ecx() == this + 0x1b0:
            x.event('pause_timer_elapsed'); x.ret(c['pause_elapsed'])
        else:
            raise AssertionError(hex(x.ecx()))
    m.seam(0x59aba0, timer)
    m.seam(0x59ab60, lambda x: (x.event('pause_timer_restart', x.ecx() == this + 0x1b0), x.ret()))
    colors = {}
    def get_color(label):
        def fn(x):
            p = x.alloc(16)
            for i in range(4): x.w(p + 4 * i, f32([0.25, 0.5, 0.75, 0.125][i]))
            x.event('get_color', label); x.ret(p)
        return fn
    def set_color(label):
        def fn(x):
            p = x.arg(1); x.event('set_color', label, [x.r(p + 4 * i) for i in range(4)]); x.ret(0, 4)
        return fn
    m.seam(0x5ee520, get_color('DisplayBannerBackground')); m.seam(0x5ee600, set_color('DisplayBannerBackground'))
    m.seam(0x5edff0, get_color('DisplayBanner')); m.seam(0x5edfe0, set_color('DisplayBanner'))
    m.seam(0x5dc780, lambda x: (x.event('selection_event_5dc780', names.get(x.ecx(), hex(x.ecx())), x.arg(1) & 0xff, x.arg(2) & 0xff), x.ret(0x5dc7, 8)))
    m.seam(0x5c3eb0, lambda x: (x.event('post_event_5c3eb0'), x.ret(0, 4)))
    # camera information text children
    loc = m.alloc(16); locvt = m.alloc(64); m.w(loc, locvt)
    m.w(locvt + 0xc, m.stub(lambda x: (x.ret(cstring(x) - 16))))
    m.seam(0x856370, lambda x: (x.event('text_manager_856370', x.arg(1)), x.ret(loc)))
    def cs(x):
        va = x.arg(1); s = bytes(x.u.mem_read(va, 128)).decode('utf-16le', 'ignore').split('\x00')[0]
        x.event('cstring_401b00', s); x.w(x.ecx(), cstring(x)); x.ret(x.ecx(), 4)
    m.seam(0x401b00, cs)
    def localize(x):
        out = x.arg(1); x.w(out, cstring(x)); x.event('localize_4b9f40'); x.ret(out, 8)
    m.seam(0x4b9f40, localize)
    def fmt(x):
        sp = x.u.reg_read(__import__('unicorn').x86_const.UC_X86_REG_ESP)
        words = [x.r(sp + 12 + 4 * i) for i in range(18)]
        x.event('format_405d60', words); x.ret()
    m.seam(0x405d60, fmt)
    m.call(0x612010, this)
    flags = {n: m.r(w + 0x24) for n, w in sorted(widgets.items(), key=lambda kv: kv[0]) if n != 'ControlPanel'}
    return dict(trace=m.trace, flags=flags, banner=[m.r(this + 0x19c), m.r(this + 0x1a0), m.r(this + 0x1a4), m.rb(this + 0x1a8)])


def cases():
    rng = random.Random(0x612010)
    base = dict(uvar_time=0, time_parent_is_this=0, time_flags=0, local_kingdom=1, group_flags=0x20, ability_flags=0x20,
                dev_label=1, dev_mode=0, dev_flags=1, node7_current=0, uvar_camera=0, camera_flags=1, uvar_supply=0,
                supply_button_flags=0, supply_functor=1, chat_button_flags=0, chat_functor=1, banner_flags=0,
                banner=[0, 0, 0], banner_prev_paused=0, now=1000, fade_ms=f32(1000.0), paused=0, game_menu=0,
                menu_flags=0, pause_flags=0, pause_elapsed=0, chat_flags=0, upkeep_kingdom=1, shortage_ratio=0,
                upkeep_flags=0)
    out = [dict(base)]
    # single-field variations around the in-game start state
    for k, vs in [('uvar_time', [1]), ('time_parent_is_this', [1]), ('local_kingdom', [0]), ('group_flags', [0, 0x21]),
                  ('ability_flags', [0]), ('dev_label', [0]), ('dev_mode', [1]), ('node7_current', [1]), ('uvar_camera', [1]),
                  ('uvar_supply', [1]), ('supply_button_flags', [0x80]), ('chat_button_flags', [0x80]), ('chat_flags', [0x20]),
                  ('paused', [1]), ('game_menu', [1]), ('upkeep_kingdom', [0]),
                  ('shortage_ratio', [f32(0.5), f32(-0.5), 0x80000000, f32(1e-30)])]:
        for v in vs:
            d = dict(base); d[k] = v; out.append(d)
    # DisplayPause blink and the banner fade arithmetic
    for paused, menu, menu_flags, elapsed, pflags in [(1, 0, 0, 999, 0), (1, 0, 0, 1000, 0), (1, 0, 0, 5000, 1), (1, 1, 0x20, 1000, 1),
                                                      (1, 1, 0, 1000, 1), (0, 0, 0, 2000, 1)]:
        d = dict(base); d.update(paused=paused, game_menu=menu, menu_flags=menu_flags, pause_elapsed=elapsed, pause_flags=pflags); out.append(d)
    for i in range(160):
        d = dict(base)
        d['banner_flags'] = 1
        d['banner'] = [rng.choice([0, 100, 5000, 0xfffffff0, rng.randrange(1 << 32)]), rng.choice([0, 900, rng.randrange(1 << 32)]),
                       rng.choice([0, 50, 1200, 0x7fffffff, rng.randrange(1 << 32)])]
        d['now'] = rng.choice([0, 1000, 2500, 0xffffffff, rng.randrange(1 << 32)])
        d['banner_prev_paused'] = rng.choice([0, 0, 1])
        d['paused'] = rng.choice([0, 0, 1])
        d['fade_ms'] = rng.choice([f32(1000.0), f32(0.5), f32(333.3), f32(1e9), f32(3.0)])
        out.append(d)
    for i in range(120):
        d = {k: (rng.choice([0, 1]) if isinstance(v, int) and v in (0, 1) else v) for k, v in base.items()}
        d.update(group_flags=rng.choice([0, 0x20, 0x21]), ability_flags=rng.choice([0, 0x20]), time_flags=rng.choice([0, 0x20]),
                 supply_button_flags=rng.choice([0, 0x80]), chat_button_flags=rng.choice([0, 0x80]), chat_flags=rng.choice([0, 0x20]),
                 menu_flags=rng.choice([0, 0x20]), pause_flags=rng.choice([0, 1]), pause_elapsed=rng.choice([0, 999, 1000, 4000]),
                 shortage_ratio=rng.choice([0, f32(0.25), f32(-1.0)]), banner_flags=0, dev_flags=rng.choice([0, 1]),
                 upkeep_flags=rng.choice([0, 1]), camera_flags=rng.choice([0, 1]))
        out.append(d)
    return out


def main():
    rows = []; pins = {}; inputs = cases()
    for image in ['original', 'v14']:
        for precision in PRECISIONS:
            m = Machine(image, precision)
            for c in inputs:
                out = run(m, c); pins.update(m.actual); rows.append(dict(image=image, precision=precision, input=c, output=out))
    a = [{k: v for k, v in r.items() if k != 'image'} for r in rows if r['image'] == 'original']
    b = [{k: v for k, v in r.items() if k != 'image'} for r in rows if r['image'] == 'v14']
    assert a == b
    for va, raw in pins.items():
        assert bytes.fromhex(raw) == original.range(va, len(bytes.fromhex(raw))) == modified.range(va, len(bytes.fromhex(raw)))
    doc = dict(schema=1, scope=__doc__, producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               native_runs=len(rows), rows=[r for r in rows if r['image'] == 'original'],
               instructions=[dict(va=hex(x), bytes=y) for x, y in sorted(pins.items())])
    path = HERE / 'native-game-update.json'
    data = (json.dumps(doc, indent=1) + '\n').encode()
    if '--write' in sys.argv: path.write_bytes(data)
    else: assert path.read_bytes() == data
    print(json.dumps(dict(rows=len(rows), inputs=len(inputs), pins=len(pins), sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
