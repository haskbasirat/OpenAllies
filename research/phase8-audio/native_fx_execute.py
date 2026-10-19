"""Original FX command execution executing unchanged under Unicorn.

* 4289A0 (node timing): command +38/+3C, VWorld+AD/+8C, node delays +28;
  stub node vtables record vf1C(cmd) / vf18(cmd) calls.
* 42A770 (FXC vf1C filter) on CreateVFX (9FC24C) and PlaySFX (9FC1E4)
  vtables: the real vf20 428360 / vf30 42AF30 / 42AE10 run; seams for the
  thing vtB4/vt100 (stub vtables), 4CF410 flags, 4282A0/42A530/54ECE0.
* 42A920 (PlaySFX vf18) with the real vtable; 7474F0 seamed (arguments
  recorded); 432E40 runs on a real B364C0 handle table.
* 43EC60's FX segment (43ED53..43EDD8): the whole 43EC60 runs with 855D30,
  50C750, 43A400, 4318E0, 4312F0 and 725340 seamed and an empty +208 list;
  429050/45C7A0/6F7430 are seamed and recorded.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, header, instructions, sha, f32, from_f32
from unicorn.x86_const import UC_X86_REG_ECX


class FxMachine(Machine):
    def __init__(self, precision):
        super().__init__(precision)
        self.events = []
        self.vworld = self.alloc(0x300)
        self.w(0xb36538, self.vworld)
        self.world = self.alloc(0x100)
        self.w(0xb36534, self.world)
        self.w(self.world + 0x30, self.alloc(0x40))
        self.table = self.alloc(0x20004 + 4 * 0x10000)
        self.w(0xb364c0, self.table)
        self.seam(0x6f7430, self.s_free)
        self.seam(0x6f7810, lambda m: m.ret(m.alloc(max(m.arg(1), 4))))

    def ev(self, op, **kw):
        self.events.append({'op': op, **kw})

    def s_free(self, m):
        m.ev('free', ptr=m.names.get(m.arg(1), hex(m.arg(1))))
        m.ret(0)

    def thing(self, handle, spec):
        o = self.alloc(0x200)
        vt = self.alloc(0x200)
        self.w(o, vt)
        self.w(vt + 0xb4, self.stub(lambda m, s=spec: m.ret(int(s['b4']))))
        r = self.alloc(0x100)
        owner = self.alloc(0x40)
        self.w(r + 0x98, owner if spec.get('r98', True) else 0)
        self.w(vt + 0x100, self.stub(lambda m, s=spec, r=r: m.ret(r if s.get('r100', True) else 0)))
        self.w(o + 0x14, handle)
        for i, v in enumerate(spec.get('pos', [0, 0, 0])):
            self.w(o + 0x20 + 4 * i, f32(v))
        index, serial = handle & 0xffff, handle >> 16
        self.u.mem_write(self.table + 4 + index * 2, struct.pack('<H', serial))
        self.w(self.table + 0x20004 + index * 4, o)
        self.w(owner + 0x14, o if spec.get('owns', True) else 0)
        return o


def cmd_words(m, c, base_words):
    p = m.alloc(0x80)
    for off, val in base_words.items():
        m.w(p + off, val)
    m.wb(p + 0x58, c.get('byte58', 0))
    return p


# --- 4289A0 ---------------------------------------------------------------
def run_timing(precision, c):
    m = FxMachine(precision)
    m.wb(m.vworld + 0xad, int(c['ad']))
    m.w(m.vworld + 0x8c, f32(c['now']))
    cmd = m.alloc(0x80)
    m.w(cmd + 0x38, f32(c['start']))
    m.w(cmd + 0x3c, f32(c['last']))
    vt = m.alloc(0x40)
    calls = []
    def vf1c(mm):
        node = mm.ecx(); calls.append(('vf1c', mm.node_index[node])); mm.ret(int(c['filter'][mm.node_index[node]]), 4)
    def vf18(mm):
        node = mm.ecx(); calls.append(('vf18', mm.node_index[node])); mm.ret(1, 4)
    m.w(vt + 0x1c, m.stub(vf1c))
    m.w(vt + 0x18, m.stub(vf18))
    m.node_index = {}
    lst = m.alloc(4)
    prev = 0
    nodes = []
    for i, d in enumerate(c['delays']):
        n = m.alloc(0x80); m.w(n, vt); m.w(n + 0x28, f32(d)); m.node_index[n] = i
        cell = m.alloc(8); m.w(cell, n); m.w(cell + 4, 0)
        nodes.append(cell)
    for a, b in zip(nodes, nodes[1:]):
        m.w(a + 4, b)
    m.w(lst, nodes[0] if nodes else 0)
    r = m.call(0x4289a0, ecx=lst, args=[cmd]) & 0xff
    m.verify_v14()
    return m, {'result': r, 'last': m.r(cmd + 0x3c), 'calls': calls}


def timing_cases():
    rnd = random.Random(0x4289a0)
    out = []
    for ad in (False, True):
        out.append({'ad': ad, 'now': 0.0, 'start': 0.0, 'last': -1.0, 'delays': [0.0, 0.5, 1.0], 'filter': [True, True, True]})
    vals = [0.0, 0.1, 0.25, 0.3333333, 0.5, 1.0, 1.5, 2.0, 7.25, 30.0, 123.456]
    for _ in range(160):
        start = rnd.choice([0.0, 0.1, 3.7, 100.25, 4096.5, 65536.125])
        last = rnd.choice([-1.0, start, start + rnd.choice(vals)])
        now = max(last, start) + rnd.choice(vals + [1e-3, 1e-5])
        delays = [rnd.choice(vals + [now - start, last - start]) for _ in range(rnd.randint(0, 5))]
        out.append({'ad': True, 'now': now, 'start': start, 'last': last, 'delays': delays,
                    'filter': [rnd.random() < 0.7 for _ in delays]})
    return out


# --- 42A770 ---------------------------------------------------------------
def run_filter(precision, c):
    m = FxMachine(precision)
    calls = []
    kind = c['kind']
    node = m.alloc(0x80)
    m.w(node, 0x9fc24c if kind == 'vfx' else 0x9fc1e4)
    defn = m.alloc(0x80)
    m.w(defn + 0x24, c['def24'])
    m.w(node + 0x3c, defn if kind == 'vfx' else c['at'])
    m.w(node + 0x40, c['at'] if kind == 'vfx' else 0)
    m.wb(node + 0x64, c['b64'])
    m.w(node + 0x18, c['must']); m.w(node + 0x1c, c['must_not'])
    m.w(node + 0x20, 0x5550 if c['group'] else 0)
    m.w(node + 0x2c, f32(c['threshold']) if c['threshold'] is not None else 0xbf800000)
    m.w(node + 0x30, c['mode']); m.w(node + 0x34, c['state'])
    cmd = m.alloc(0x80)
    m.wb(cmd + 0x58, c['byte58'])
    if c['target'] is not None:
        m.thing(0x10021, c['target']); m.w(cmd + 0x20, 0x10021)
    else:
        m.w(cmd + 0x20, 0x10099)
    if c['source'] is not None:
        m.thing(0x10022, c['source']); m.w(cmd + 0x10, 0x10022)
    else:
        m.w(cmd + 0x10, 0x10098)
    for i, v in enumerate(c['src_pos']):
        m.w(cmd + 0x14 + 4 * i, f32(v))
    for i, v in enumerate(c['dst_pos']):
        m.w(cmd + 0x24 + 4 * i, f32(v))
    m.w(cmd + 0x40, f32(c['before'])); m.w(cmd + 0x44, f32(c['now']))
    m.w(cmd + 0x54, c['ctx_state'])
    def s_flags(mm):
        calls.append(['4cf410']); mm.w(mm.arg(1), c['flags']); mm.ret(mm.arg(1), 4)
    def s_valid(mm):
        calls.append(['4282a0', mm.arg(1), mm.arg(2)]); mm.ret(int(c['valid']), 8)
    tile = m.alloc(0x40)
    grp = m.alloc(0x40)
    m.w(grp + 0x34, 0x5550 if c['tile_group'] else 0x6660)
    def s_tile(mm):
        calls.append(['42a530', mm.arg(1), mm.arg(2)]); mm.ret(tile, 8)
    def s_group(mm):
        calls.append(['54ece0']); mm.ret(grp)
    m.seam(0x4cf410, s_flags); m.seam(0x4282a0, s_valid); m.seam(0x42a530, s_tile); m.seam(0x54ece0, s_group)
    r = m.call(0x42a770, ecx=node, args=[cmd]) & 0xff
    m.verify_v14()
    return m, {'result': r, 'calls': calls}


def filter_cases():
    rnd = random.Random(0x42a770)
    out = []
    thing = lambda: {'b4': rnd.random() < 0.8, 'r100': rnd.random() < 0.8, 'r98': rnd.random() < 0.8, 'owns': rnd.random() < 0.5}
    for _ in range(600):
        out.append({
            'kind': rnd.choice(['vfx', 'sfx']), 'byte58': rnd.choice([0, 0, 1]), 'def24': rnd.choice([0, 1]),
            'b64': rnd.choice([0, 1]), 'at': rnd.choice([0, 1, 2, 3, 4, 5, 6]),
            'must': rnd.choice([0, 0, 1, 6]), 'must_not': rnd.choice([0, 0, 2, 8]), 'flags': rnd.choice([0, 1, 2, 6, 7, 15]),
            'group': rnd.random() < 0.3, 'valid': rnd.random() < 0.8, 'tile_group': rnd.random() < 0.5,
            'threshold': rnd.choice([None, None, 0.0, 0.25, 0.5]), 'mode': rnd.choice([0, 1, 2]),
            'before': rnd.choice([-1.0, 0.0, 0.25, 0.5, 0.75, 1.0]), 'now': rnd.choice([-1.0, 0.0, 0.25, 0.5, 0.75, 1.0]),
            'state': rnd.choice([42, 42, 3, 0x14]), 'ctx_state': rnd.choice([42, 3, 0x14]),
            'target': thing() if rnd.random() < 0.7 else None, 'source': thing() if rnd.random() < 0.7 else None,
            'src_pos': [rnd.uniform(-5, 50), rnd.uniform(-5, 50), 0], 'dst_pos': [rnd.uniform(-5, 50), rnd.uniform(-5, 50), 0],
        })
    return out


# --- 42A920 ---------------------------------------------------------------
def run_sfx(precision, c):
    m = FxMachine(precision)
    m.wb(m.vworld + 0xad, 1)
    m.w(m.vworld + 0x8c, f32(c['now']))
    calls = []
    m.w(m.world + 0x3c, f32(c['map'][0])); m.w(m.world + 0x40, f32(c['map'][1]))
    node = m.alloc(0x80)
    m.w(node, 0x9fc1e4)
    m.w(node + 0x3c, c['at']); m.wb(node + 0x40, c['loop'])
    audio = m.alloc(0x80); m.w(node + 0x44, audio)
    # one stub child in the immediate (+8) list: it must never run here
    cvt = m.alloc(0x40)
    m.w(cvt + 0x1c, m.stub(lambda mm: (calls.append(['child_vf1c']), mm.ret(1, 4))))
    m.w(cvt + 0x18, m.stub(lambda mm: (calls.append(['child_vf18']), mm.ret(1, 4))))
    child = m.alloc(0x80); m.w(child, cvt); m.w(child + 0x28, f32(c['child_delay']))
    cell = m.alloc(8); m.w(cell, child); m.w(cell + 4, 0)
    m.w(node + 8, cell)
    cmd = m.alloc(0x80)
    m.w(cmd + 0x38, f32(c['start'])); m.w(cmd + 0x3c, f32(c['now']))
    names = {}
    for slot, key in [(0x10, 'source'), (0x20, 'target'), (0xc, 'owner')]:
        spec = c[key]
        handle = {0x10: 0x10031, 0x20: 0x10032, 0xc: 0x10033}[slot]
        if spec is not None:
            o = m.thing(handle, spec); names[o] = key
        m.w(cmd + slot, handle)
    for i, v in enumerate(c['src_pos']):
        m.w(cmd + 0x14 + 4 * i, f32(v))
    for i, v in enumerate(c['dst_pos']):
        m.w(cmd + 0x24 + 4 * i, f32(v))
    def s_play(mm):
        a = [mm.arg(i + 1) for i in range(8)]
        assert mm.ecx() == mm.r(0xb36548)
        pos = None if a[5] == 0 else [mm.r(a[5] + 4 * i) for i in range(3)]
        calls.append(['7474f0', a[0] == audio, a[1] & 0xff, names.get(a[2]), a[3], names.get(a[4]), pos, a[6] == node, a[7] == cmd])
        mm.ret(5, 0x20)
    m.w(0xb36548, m.alloc(0x400))
    m.seam(0x7474f0, s_play)
    r = m.call(0x42a920, ecx=node, args=[cmd]) & 0xff
    m.verify_v14()
    return m, {'result': r, 'calls': calls}


def sfx_cases():
    rnd = random.Random(0x42a920)
    out = []
    def thing():
        if rnd.random() < 0.25:
            return None
        return {'b4': rnd.random() < 0.6, 'pos': [rnd.choice([-3.0, 0.0, 4.5, 20.0]), rnd.choice([-3.0, 0.0, 7.0]), 1.5]}
    for _ in range(300):
        out.append({'at': rnd.choice([0, 1, 2, 3, 4, 5, 6, 7]), 'loop': rnd.choice([0, 1]), 'now': 5.0, 'start': 1.0,
                    'child_delay': rnd.choice([0.0, 1.0, 4.0]), 'source': thing(), 'target': thing(),
                    'owner': thing(), 'map': [rnd.choice([256.0, 384.0]), rnd.choice([256.0, 512.0])],
                    'src_pos': [rnd.choice([-1.0, 0.0, 2.0]), rnd.choice([-1.0, 0.0, 3.0]), 0.5],
                    'dst_pos': [rnd.choice([-1.0, 0.0, 9.0]), rnd.choice([-1.0, 0.0, 8.0]), 0.25]})
    return out


# --- 43EC60 FX segment -----------------------------------------------------
def run_loop(precision, c):
    m = FxMachine(precision)
    m.wb(m.vworld + 0xad, 1)
    m.w(m.vworld + 0xb8, 0x77)
    m.w(0xb3cc94, 0x77)
    m.w(m.world + 0xd8, 0)
    m.w(m.vworld + 0x8c, f32(c['now']))
    m.w(m.vworld + 0x208, 0)
    m.names = {}
    calls = []
    for a in (0x855d30, 0x50c750, 0x43a400, 0x4318e0, 0x4312f0, 0x725340):
        m.seam(a, lambda mm, a=a: mm.ret(0))
    m.seam(0x429050, lambda mm: (calls.append(['429050', mm.names[mm.ecx()]]), mm.ret(0)))
    m.seam(0x45c7a0, lambda mm: (calls.append(['45c7a0', mm.names[mm.ecx() - 0x5c]]), mm.ret(0)))
    def s_free(mm):
        calls.append(['free', mm.names.get(mm.arg(1))]); mm.ret(0)
    m.seam(0x6f7430, s_free)
    vt = m.alloc(0x40)
    m.w(vt + 0x1c, m.stub(lambda mm: (calls.append(['vf1c', mm.node_names[mm.ecx()]]), mm.ret(1, 4))))
    m.w(vt + 0x18, m.stub(lambda mm: (calls.append(['vf18', mm.node_names[mm.ecx()]]), mm.ret(1, 4))))
    m.node_names = {}
    cmds = []
    for i, spec in enumerate(c['commands']):
        cmd = m.alloc(0x80)
        m.names[cmd] = i
        m.w(cmd + 0x38, f32(spec['start'])); m.w(cmd + 0x3c, f32(spec['last']))
        lst = m.alloc(4)
        cells = []
        for j, d in enumerate(spec['delays']):
            n = m.alloc(0x40); m.w(n, vt); m.w(n + 0x28, f32(d)); m.node_names[n] = f'{i}.{j}'
            cell = m.alloc(8); m.w(cell, n); cells.append(cell)
        for a, b in zip(cells, cells[1:]):
            m.w(a + 4, b)
        m.w(lst, cells[0] if cells else 0)
        m.w(cmd + 4, lst)
        cmds.append(cmd)
    for a, b in zip(cmds, cmds[1:]):
        m.w(a + 0x68, b)
    m.w(m.vworld + 0xe0, cmds[0] if cmds else 0)
    m.w(m.vworld + 0xe4, cmds[-1] if cmds else 0)
    m.w(m.vworld + 0xe8, len(cmds))
    m.call(0x43ec60, ecx=m.vworld)
    order, p = [], m.r(m.vworld + 0xe0)
    seen = set()
    while p and p not in seen:
        seen.add(p); order.append(m.names[p]); p = m.r(p + 0x68)
    tail = m.r(m.vworld + 0xe4)
    m.verify_v14()
    return m, {'calls': calls, 'order': order, 'tail': m.names.get(tail), 'count': m.r(m.vworld + 0xe8),
               'last': [m.r(cmd + 0x3c) for cmd in cmds]}


def loop_cases():
    rnd = random.Random(0x43ec60)
    out = []
    for _ in range(80):
        n = rnd.randint(0, 6)
        now = rnd.choice([0.0, 1.0, 2.5, 10.0])
        cmds = []
        for i in range(n):
            start = rnd.choice([0.0, 0.5, 1.0])
            cmds.append({'start': start, 'last': rnd.choice([-1.0, start]),
                         'delays': [rnd.choice([0.0, 0.5, 1.0, 2.0, 5.0, 20.0]) for _ in range(rnd.randint(0, 3))]})
        out.append({'now': now, 'commands': cmds})
    return out


def main():
    visited = {}
    out = {'timing': [], 'filter': [], 'sfx': [], 'loop': []}
    for precision in ('Single24', 'Double53', 'Extended64'):
        for group, cases, runner in [('timing', timing_cases(), run_timing), ('filter', filter_cases(), run_filter),
                                     ('sfx', sfx_cases(), run_sfx), ('loop', loop_cases(), run_loop)]:
            for c in cases:
                m, r = runner(precision, c)
                visited.update(m.visited)
                out[group].append({'precision': precision, 'input': c, 'output': r})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), **out)
    p = HERE / 'native-fx-execute.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({k: len(v) for k, v in out.items()} | {'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
