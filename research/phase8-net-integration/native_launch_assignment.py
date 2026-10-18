"""Execute original 0x779F00 and all value-copy descendants, without copy seams.

Allocator, CString manager and logging seams are inherited from phase8-net.
Page-zero accesses fault except fs:[0]. No floating arithmetic is involved.
The original and v14 .text images must produce identical complete value state.
"""
from pathlib import Path
import copy, hashlib, json, random, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-net'))
from native_lobby import LobbyMachine, base_data, rich_launch, launch, U, executed_windows
from native_channels import PE0, PE1

HEADERS = ('game', 'map_default', 'map_scenario', 'map_random', 'map_24', 'map_metagame')

class Machine(LobbyMachine):
    def vec(self, base, elems, size, build):
        # Native vector growth destroys the old capacity, using the array prefix.
        n = len(elems)
        data = self.alloc(4 + n * size) + 4 if n else 0
        if data: self.w(data - 4, n)
        self.w(base, data); self.w(base + 4, n); self.w(base + 8, n)
        for i, e in enumerate(elems): build(data + i * size, e)

    def header_addresses(self, p):
        return [self.r(self.r(p + 0x14) + 8)] + [self.r(p + o) + 4 for o in (0x18, 0x1c, 0x20, 0x24, 0x28)]

    def build_complete(self, p, s):
        self.build_launch(p, s['wire'])
        self.cs(p + 0x30, s['wire']['text_30'])
        c = s['retained']
        self.cs(p + 0x10, c['text_10'])
        self.w(self.r(p + 0x14) + 0x14, c['game_field_14'])
        self.cs(self.r(p + 0x20) + 0x74, c['random_text_74'])
        self.w(self.r(p + 0x24) + 0x50, c['map24_field_50'])
        meta = self.r(p + 0x28)
        self.b(meta + 0x50, c['metagame_flag_50'])
        self.w(meta + 0x54, c['metagame_field_54']); self.w(meta + 0x58, c['metagame_field_58'])
        for h, name in zip(self.header_addresses(p), HEADERS):
            a = self.r(h + 0x10)
            for i, e in enumerate(c['kingdoms'][name]):
                self.cs(a + i * 0x6c + 0xc, e['text_0c'])
                self.b(a + i * 0x6c + 0x68, e['flag_68'])

    def read_complete(self, p):
        wire = self.read_launch(p)
        wire['text_30'] = self.rs(p + 0x30)
        meta = self.r(p + 0x28)
        kingdoms = {}
        for h, name in zip(self.header_addresses(p), HEADERS):
            kingdoms[name] = self.rvec(h + 0x10, 0x6c, lambda k: {'text_0c': self.rs(k + 0xc), 'flag_68': self.rb(k + 0x68) != 0})
        return {'wire': wire, 'retained': {
            'text_10': self.rs(p + 0x10), 'game_field_14': self.r(self.r(p + 0x14) + 0x14),
            'random_text_74': self.rs(self.r(p + 0x20) + 0x74), 'map24_field_50': self.r(self.r(p + 0x24) + 0x50),
            'metagame_flag_50': self.rb(meta + 0x50) != 0, 'metagame_field_54': self.r(meta + 0x54),
            'metagame_field_58': self.r(meta + 0x58), 'kingdoms': kingdoms}}

def complete(rng, index, rich):
    wire = copy.deepcopy(rich_launch(rng, mode=index % 6, version=index % 4,
                                    scenario='scenario', random_map='rmc') if rich else launch())
    # Vary nested counts, including growth, shrinkage, empty and UTF-16 data.
    for name in HEADERS:
        h = wire[name]['header']
        h['kingdoms'] = (h['kingdoms'] * (1 + index % 3))[:index % 7]
        h['names'] = (h['names'] * (1 + index % 2))[:index % 4]
        wire[name]['conditions']['changes'] = rng.getrandbits(32)
    wire['text_30'] = U('launch\0text-' + str(index) + '\ud800')
    wire['game_type'] = index % 6
    word = rng.getrandbits
    retained = {
        'text_10': U('non-wire\0name-' + str(index)), 'game_field_14': word(32),
        'random_text_74': U('rmc\udfff/' + str(index)), 'map24_field_50': word(32),
        'metagame_flag_50': bool(index & 1), 'metagame_field_54': word(32), 'metagame_field_58': word(32),
        'kingdoms': {name: [{'text_0c': U(f'{index}/{name}/{i}\0tail'), 'flag_68': bool((index + i) & 1)}
                             for i in range(len(wire[name]['header']['kingdoms']))] for name in HEADERS}}
    return {'wire': wire, 'retained': retained}

def main():
    rows = []
    all_visited = {}
    for image in ('original', 'v14'):
        m = Machine(True, image)
        m.mode = 'channels'
        m.setup_data(base_data())
        rng = random.Random(0x779f00)
        image_rows = []
        for i in range(32):
            src = complete(rng, i, i % 4 != 0)
            dst = complete(rng, i + 3, i % 4 != 1)
            sp = m.alloc(0x34); dp = m.alloc(0x34)
            m.build_complete(sp, src); m.build_complete(dp, dst)
            self_assign = i % 8 == 7
            if self_assign: dp = sp; dst = copy.deepcopy(src)
            before = m.read_complete(dp); source = m.read_complete(sp)
            returned = m.call(0x779f00, [sp], this=dp, callee_pop=4)
            after = m.read_complete(dp)
            assert returned == dp
            assert m.read_complete(sp) == source, (image, i, 'source changed')
            assert after == source, (image, i, 'incomplete value copy')
            image_rows.append({'index': i, 'self_assignment': self_assign, 'source': source, 'before': before, 'after': after})
        if rows: assert rows == image_rows, 'original/v14 disagreement'
        else: rows = image_rows
        all_visited.update(m.visited)
        print(image, len(image_rows), 'complete native assignments passed', flush=True)
    result = {'schema': 1, 'function': '0x779f00', 'original_sha256': hashlib.sha256(PE0.b).hexdigest(),
              'v14_sha256': hashlib.sha256(PE1.b).hexdigest(), 'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'seams': ['allocator', 'CString manager', 'logging'], 'cases': rows,
              'executed_windows': executed_windows(all_visited)}
    (HERE / 'native-launch-assignment.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')

if __name__ == '__main__': main()
