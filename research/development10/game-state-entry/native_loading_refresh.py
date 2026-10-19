"""Original/v14 LoadingScreen OnConnected 6E8730 -> refresh 6E83A0 -> status
6E8300, unchanged code on a laboratory LoadingScreen (vtable A7EDD0 fields of
Init 6E88E0: +D8 MapNameLabel, +DC MapDescriptionLabel, +E0 ObjectivesList,
+E4/+E8 LoadingTextureLeft/Right, +EC/+F0 default art, +F4/+F8 dynamic
campaign art, +FC StatusLabel, +100 ReadyButton).

Real code: 6E8730, 6E83A0, 6E8300, 40CEB0, 76AC90, 5EFB00, 673720, 5C0E10,
76AC60, 76C700, 50DC00. Seams (recorded, not executed): 5C08E0, 411790 and
the campaign mission art +0C calls, 673D40, the label +C8 SetText calls,
ObjectivesList+88 +C8, 5EE580, 5EF100, 856370, 76E4C0, 96CB83 (wcscmp),
745580. Application-state evidence only; no game file is changed.
"""
from pathlib import Path
import sys, json, hashlib, itertools
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import Machine as Base, original, modified, PRECISIONS
from unicorn.x86_const import UC_X86_REG_EIP

APP, SESSION, NODE, UI, AUDIO, CAMPAIGN = 0xB39690, 0xB36550, 0xB36558, 0xB3653C, 0xB36548, 0xB39D04


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


def wstring(m, text):
    p = m.alloc(2 * len(text) + 4); m.u.mem_write(p, text.encode('utf-16le') + b'\0\0'); return p


def run(m, c):
    m.seams = {}; m.observers = {}; m.trace = []; m.actual = {}
    names = {}
    ls = m.alloc(0x120)
    for off, name in [(0xd8, 'MapNameLabel'), (0xdc, 'MapDescriptionLabel'), (0xe0, 'ObjectivesList'), (0xe4, 'LoadingTextureLeft'),
                      (0xe8, 'LoadingTextureRight'), (0xfc, 'StatusLabel'), (0x100, 'ReadyButton')]:
        w = m.alloc(0x100); m.w(ls + off, w); names[w] = name; m.w(w + 0x24, c['flags'])
        vt = m.alloc(0x100); m.w(w, vt)
        m.w(vt + 0xc8, m.stub(lambda x, n=name: (x.event('set_text_c8', n, names.get(x.arg(1), hex(x.arg(1)))), x.ret(0, 4))))
    slot = m.alloc(0x40); svt = m.alloc(0x100); m.w(slot, svt); m.w(m.r(ls + 0xe0) + 0x88, slot)
    m.w(svt + 0xc8, m.stub(lambda x: (x.event('objectives_clear_items'), x.ret())))
    for off, name in [(0xec, 'default_left'), (0xf0, 'default_right'), (0xf4, 'dynamic_left'), (0xf8, 'dynamic_right')]:
        v = m.alloc(16) if c['art'][name] else 0
        m.w(ls + off, v)
        if v: names[v] = name
    # application: GameAppState node 12 (+C kind)
    gas = m.alloc(0x40); m.w(gas + 0xc, c['kind']); m.w(APP + 0xcc + 12 * 4, gas)
    # session: +F8 phase, +58 launch mode, +68 GameSettings (+24 name, +2C description), creators +6C/+70/+74/+7C
    s = m.alloc(0x200); m.w(SESSION, s); m.w(s + 0xf8, c['phase']); m.w(s + 0x58, c['launch_mode'])
    gs = m.alloc(0x40); m.w(s + 0x68, gs); names[gs + 0x24] = 'map_name'; names[gs + 0x2c] = 'map_description'
    creators = {}
    for off in (0x6c, 0x70, 0x74, 0x7c):
        cr = m.alloc(0x40); m.w(s + off, cr - 4); creators[off] = cr
        entries = m.alloc(0x6c * 3)
        m.w(cr + 0x10, entries); m.w(cr + 0x14, 3)
        for i, kname in enumerate(['Side1', 'Side2', c['creator_side'] if off == c['creator_off'] else 'Other']):
            e = entries + 0x6c * i; m.w(e, wstring(m, kname))
            nation = 0
            if i == 2 and off == c['creator_off'] and c['nation']:
                nation = m.alloc(0x80)
                for k, o in [('nation_left', 0x5c), ('nation_right', 0x60)]:
                    v = m.alloc(16) if c[k] else 0; m.w(nation + o, v)
                    if v: names[v] = k
            m.w(e + 0x18, nation)
    node = m.alloc(0x20); m.w(NODE, node)
    player = m.alloc(0x40) if c['local_player'] else 0
    m.w(node + 8, player)
    if player:
        m.w(player + 0x24, wstring(m, c['player_side']))
    m.w(UI, m.alloc(0x200) if c['ui'] else 0)
    m.w(AUDIO, m.alloc(0x200))
    # campaign mission entry for kind 0
    mission = 0
    if c['kind'] == 0:
        mission = m.alloc(0x40)
        for k, o in [('mission_left', 0x14), ('mission_right', 0x18)]:
            if c[k]:
                t = m.alloc(16); tvt = m.alloc(0x40); m.w(t, tvt); names[t] = k
                m.w(tvt + 0xc, m.stub(lambda x, n=k: (x.event('preload_mission_art', n, x.arg(1), x.arg(2) == m.r(0xb282f0)), x.ret(0, 8))))
                m.w(mission + o, t)
            else:
                m.w(mission + o, 0)
        t = m.alloc(16); tvt = m.alloc(0x40); m.w(t, tvt); m.w(mission + 0x10, t)
        m.w(tvt + 0xc, m.stub(lambda x: (x.event('preload_mission_art', 'mission_10', x.arg(1), x.arg(2) == m.r(0xb282f0)), x.ret(0, 8))))
    camp = m.alloc(0x80); m.w(CAMPAIGN, camp)
    def campaign(x):
        x.event('campaign_entry_411790', x.arg(2))
        lst = x.alloc(0x40); arr = x.alloc(0x40); hdr = x.alloc(0x40); x.w(hdr + 0x24, arr); x.w(arr + 4 * 3, mission)
        x.w(lst, hdr); x.w(lst + 0x14, 3); x.ret(lst, 8)
    m.seam(0x411790, campaign)
    m.seam(0x5c08e0, lambda x: (x.event('base_on_connected_5c08e0'), x.ret()))
    m.seam(0x673d40, lambda x: (x.event('objectives_populate_673d40'), x.ret()))
    m.seam(0x5ef100, lambda x: (x.event('visible', names.get(x.ecx(), hex(x.ecx())), x.r(x.ecx() + 0x24) & 1), x.ret()))
    m.seam(0x5ee580, lambda x: (x.event('set_texture_5ee580', names.get(x.ecx(), hex(x.ecx())), names.get(x.arg(1), hex(x.arg(1))) if x.arg(1) else None, x.arg(2)), x.ret(0, 8)))
    def wcscmp(x):
        a = bytes(x.u.mem_read(x.arg(1), 128)).decode('utf-16le', 'ignore').split('\x00')[0]
        b = bytes(x.u.mem_read(x.arg(2), 128)).decode('utf-16le', 'ignore').split('\x00')[0]
        x.ret(0 if a == b else (1 if a > b else 0xffffffff))
    m.seam(0x96cb83, wcscmp)
    loc = m.alloc(16); locvt = m.alloc(64); m.w(loc, locvt)
    def empty(x):
        a = x.alloc(64); x.w(a + 0xc, 1000); x.ret(a)
    m.w(locvt + 0xc, m.stub(empty))
    m.seam(0x856370, lambda x: (x.ret(loc)))
    status = m.alloc(64); m.w(status + 0xc, 1000); names[status + 16] = 'session_status'
    def session_status(x):
        x.event('session_status_76e4c0'); x.w(x.arg(1), status + 16); x.ret(0, 4)
    m.seam(0x76e4c0, session_status)
    m.seam(0x745580, lambda x: (x.event('audio_hold_745580', x.ecx() == m.r(AUDIO)), x.ret()))
    # the label +C8 receives a pointer to the CString holder; identify payloads
    holder_names = dict(names)
    def set_text(n):
        def fn(x):
            p = x.arg(1)
            v = holder_names.get(p) or holder_names.get(x.r(p)) or hex(p)
            x.event('set_text_c8', n, v); x.ret(0, 4)
        return fn
    for off, n in [(0xd8, 'MapNameLabel'), (0xdc, 'MapDescriptionLabel'), (0xfc, 'StatusLabel')]:
        w = m.r(ls + off); m.w(m.r(w) + 0xc8, m.stub(set_text(n)))
    m.call(0x6e8730, ls)
    return dict(trace=m.trace, flags={n: m.r(m.r(ls + o) + 0x24) for o, n in [(0xd8, 'MapNameLabel'), (0xdc, 'MapDescriptionLabel'),
                (0xe0, 'ObjectivesList'), (0xe4, 'LoadingTextureLeft'), (0xe8, 'LoadingTextureRight'), (0x100, 'ReadyButton')]})


MODE_CREATOR = {0: 0x74, 1: 0x6c, 2: 0x70, 3: 0x6c, 4: 0x6c, 5: 0x7c}


def cases():
    out = []
    full_art = dict(default_left=1, default_right=1, dynamic_left=1, dynamic_right=1)
    base = dict(flags=0, phase=2, launch_mode=1, kind=2, local_player=1, ui=1, player_side='Side3', creator_side='Side3',
                creator_off=0x6c, nation=1, nation_left=1, nation_right=1, art=full_art, mission_left=0, mission_right=0)
    for phase, mode in itertools.product([0, 2, 5], [0, 1, 2, 3, 4, 5]):
        d = dict(base); d.update(phase=phase, launch_mode=mode, creator_off=MODE_CREATOR[mode]); out.append(d)
    for kind in [0, 1, 2, 4]:
        for local, ui, nation, nl, nr, match in itertools.product([0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1]):
            if not nation and (nl or nr):
                continue
            d = dict(base); d.update(kind=kind, local_player=local, ui=ui, nation=nation, nation_left=nl, nation_right=nr,
                                     player_side='Side3' if match else 'Nobody')
            out.append(d)
    for ml, mr, dl, dr in itertools.product([0, 1], [0, 1], [0, 1], [0, 1]):
        d = dict(base); d.update(kind=0, mission_left=ml, mission_right=mr, art=dict(default_left=1, default_right=1, dynamic_left=dl, dynamic_right=dr))
        out.append(d)
        d = dict(d); d.update(kind=4); out.append(d)
    for flags in [1, 0x21]:
        d = dict(base); d['flags'] = flags; out.append(d)
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
               native_runs=len(rows), rows=[r for r in rows if r['image'] == 'original' and r['precision'] == 'Double53'],
               precision_independent=all(len({json.dumps(r['output'], sort_keys=True) for r in a if r['input'] == c}) == 1
                                         for c in inputs),
               instructions=[dict(va=hex(x), bytes=y) for x, y in sorted(pins.items())])
    path = HERE / 'native-loading-refresh.json'
    data = (json.dumps(doc, indent=1) + '\n').encode()
    if '--write' in sys.argv: path.write_bytes(data)
    else: assert path.read_bytes() == data
    print(json.dumps(dict(rows=len(rows), inputs=len(inputs), pins=len(pins), sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
