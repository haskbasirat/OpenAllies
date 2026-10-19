"""Original/v14 Loading -> Game application state entry, unchanged code.

Executes Loading682EE0 -> 40CE10(Loading+C,0,0) -> Loading OnLeave40CAA0 ->
Game OnEnter608C80/40CA80 with the REAL Loading (vtable A6E6A4) and Game
(vtable A56C00) nodes in the Application table at B39690+CC. Interface Show
5C0440, the status string/773CE0, Session76ACA0 and the SetState diagnostic
404F60 are recorded seams (not executed). Also executes the UserReady command
6E80F0 and pins the AppState-node and Game-interface virtual slots read by
40CE10/40FBE0/410E70/40CF00 and KKC_Application::Init (41AA7A). No game file
is changed. This is application-state evidence only, not a playable battle.
"""
from pathlib import Path
import sys, json, hashlib, itertools
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import Machine as Base, original, modified, PRECISIONS
from unicorn.x86_const import UC_X86_REG_EIP

APP = 0xB39690
LOADING_VT, GAME_VT, MAIN_VT, GAME_UI_VT = 0xA6E6A4, 0xA56C00, 0xA7ED98, 0xA5B0B0


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

    def event(self, label, *values):
        self.trace.append([label, *values] if values else label)


def loading_fields(m, p):
    return dict(changed_8=m.rb(p + 8), return_state_c=m.r(p + 0xc), flags_14_17=[m.rb(p + x) for x in range(0x14, 0x18)])


def application(m, current, previous, loading, game):
    m.w(APP + 0x10c, current); m.w(APP + 0x110, previous)
    m.w(APP + 0xcc + 4 * 4, game); m.w(APP + 0xcc + 6 * 4, loading)


def run(m, case):
    m.seams = {}; m.observers = {}; m.trace = []; m.actual = {}
    op = case['op']
    if op == 'tick_enter':
        loading = m.alloc(0x18); m.u.mem_write(loading, b'\xa5' * 0x18); m.call(0x682de0, loading)
        m.wb(loading + 8, 1)  # 682E10's changed byte, as after the real kind0 bind
        for i, v in enumerate(case['flags']): m.wb(loading + 0x14 + i, v)
        m.w(loading + 0xc, case['return_state'])
        game = m.alloc(8); m.w(game, GAME_VT)
        interface = m.alloc(0x200) if case['game_interface'] else 0
        m.w(game + 4, interface)
        application(m, 6, 2, loading, game)
        world = m.alloc(0xb0); session = m.alloc(0x100)
        m.w(0xb36538, world); m.w(0xb36550, session); m.wb(world + 0xad, case['view']); m.wb(session + 0xf4, case['multi'])

        def cstring(x):
            a = x.alloc(32); x.w(a + 12, 3); x.w(x.ecx(), a + 16); x.ret(x.ecx(), 4)
        m.seam(0x401b00, cstring)
        m.seam(0x773ce0, lambda x: (x.event('waiting_status'), x.ret(0, 12)))
        m.seam(0x76aca0, lambda x: (x.event('vworld_complete', x.arg(1) & 0xff), x.ret(0, 4)))
        m.seam(0x404f60, lambda x: (x.event('set_state_diagnostic', x.arg(6), x.arg(7)), x.ret()))

        def show(x):
            x.event('show_5c0440', 'game_interface' if interface and x.ecx() == interface else hex(x.ecx()), x.arg(1))
            x.ret(0, 4)
        m.seam(0x5c0440, show)
        m.call(0x682ee0, loading)
        return dict(loading=loading_fields(m, loading), current_10c=m.r(APP + 0x10c), previous_110=m.r(APP + 0x110), trace=m.trace)
    if op == 'user_ready':
        loading = m.alloc(0x18); m.u.mem_write(loading, b'\xa5' * 0x18); m.call(0x682de0, loading)
        m.wb(loading + 0x16, case['old16'])
        m.w(APP + 0xe4, loading); m.w(APP + 0x10c, case['current'])
        m.call(0x6e80f0, 0, (0x13579bdf,))
        return dict(loading=loading_fields(m, loading), current_10c=m.r(APP + 0x10c))
    raise AssertionError(op)


def cases():
    for flags, view, multi, state, interface in itertools.product(itertools.product([0, 1], repeat=4), [0, 1], [0, 1], [4, 0], [1, 0]):
        yield dict(op='tick_enter', flags=list(flags), view=view, multi=multi, return_state=state, game_interface=interface)
    for current, old in itertools.product([0, 2, 4, 6], [0, 0xa5]):
        yield dict(op='user_ready', current=current, old16=old)


SLOTS = {'tick_10': 0x10, 'enter_14': 0x14, 'leave_18': 0x18, 'event_1c': 0x1c, 'finished_28': 0x28}
INTERFACE_SLOTS = {'update_0c': 0x0c, 'on_connected_48': 0x48, 'init_c8': 0xc8, 'initialized_e8': 0xe8}


def u32(img, va):
    return int.from_bytes(img.range(va, 4), 'little')


def tables():
    out = {}
    for name, vt, slots in [('loading_node', LOADING_VT, SLOTS), ('game_node', GAME_VT, SLOTS), ('main_node', MAIN_VT, SLOTS),
                            ('game_interface', GAME_UI_VT, INTERFACE_SLOTS)]:
        row = {}
        for slot, off in slots.items():
            target = u32(original, vt + off); assert target == u32(modified, vt + off)
            head = original.range(target, 5); assert head == modified.range(target, 5)
            row[slot] = dict(target=hex(target), head=head.hex())
        out[name] = row
    # Application constructor 40CB40: node4 (+DC) is 608C60, which copies B3974C.
    # KKC_Application::Init: ecx=[esi+BC] (B39690+BC = B3974C), call [vt+C8].
    pins = {}
    for va, size in [(0x40cc10, 5), (0x40cc1f, 6), (0x608c60, 14), (0x41aa7a, 14), (0x608c80, 5), (0x5e8900, 5), (0x943800, 3), (0x4b2830, 1)]:
        raw = original.range(va, size); assert raw == modified.range(va, size); pins[hex(va)] = raw.hex()
    out['static_pins'] = pins
    return out


def main():
    rows = []; pins = {}; inputs = list(cases())
    for image in ['original', 'v14']:
        for precision in PRECISIONS:
            m = Machine(image, precision)
            for case in inputs:
                out = run(m, case); pins.update(m.actual); rows.append(dict(image=image, precision=precision, input=case, output=out))
    a = [{k: v for k, v in r.items() if k != 'image'} for r in rows if r['image'] == 'original']
    b = [{k: v for k, v in r.items() if k != 'image'} for r in rows if r['image'] == 'v14']
    assert a == b
    for va, raw in pins.items():
        assert bytes.fromhex(raw) == original.range(va, len(bytes.fromhex(raw))) == modified.range(va, len(bytes.fromhex(raw)))
    doc = dict(schema=1, scope=__doc__, producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               tables=tables(), native_runs=len(rows), rows=[r for r in rows if r['image'] == 'original'],
               instructions=[dict(va=hex(x), bytes=y) for x, y in sorted(pins.items())])
    path = HERE / 'native-game-state.json'
    data = (json.dumps(doc, indent=1) + '\n').encode()
    if '--write' in sys.argv: path.write_bytes(data)
    else: assert path.read_bytes() == data
    print(json.dumps(dict(rows=len(rows), inputs=len(inputs), native_runs=len(rows), pins=len(pins), sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
