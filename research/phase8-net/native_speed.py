"""Original KKC_SetGameSpeedOrderDecree process 0x75D810 under Unicorn, with
its authority test 0x76AD90 (world +0x275 cutscene, ordering player +4
client, 0x74B900 client == AdminNode+0x2C4) and the ordering-player getter
0x74B3B0 (AdminNode+0x284). KKC_Sim::SetGameSpeed 0x76F370 is a recording
seam (ported and oracle-proven separately in aa-runtime, research/phase8-tick):
the fixture records whether it is reached, with which `this` and argument.

Every combination of: session [0xB36550] null/set, ordering player null/set,
world [0xB36534] null / +0x275 = 0 / 1, player+4 client null/set, client is
/ is not AdminNode+0x2C4; several speed bit patterns.
"""
from pathlib import Path
import itertools, json, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard

NODE = 0x2030000; SESSION_ = 0x2031000; WORLD = 0x2032000; PLAYER = 0x2033000; CLIENT = 0x2034000; OTHER = 0x2035000
ORDER = 0x2036000; SIM = 0x2037000

class SpeedMachine(OrdersMachine):
    def __init__(self):
        super().__init__()
        self.mode = 'speed'; self.calls = []
        null_guard.install(self)
        self.u.mem_write(NODE, b'\0' * 0x8000)
    def hook(self, u, pc, size, user):
        if self.mode == 'speed':
            self.visited[pc] = size
            if pc == 0x76f370:
                from unicorn.x86_const import UC_X86_REG_ECX
                self.calls.append({'this': hex(u.reg_read(UC_X86_REG_ECX)), 'speed_bits': self.arg()})
                self.ret(4); return
        super().hook(u, pc, size, user)

def main():
    m = SpeedMachine(); rows = []
    m.w(0xb36554, SIM)
    speeds = [0x00000000, 0x3f800000, 0xbf800000, 0x40400000, 0x7fc00000]
    for k, (session, player, world, client, is_host) in enumerate(itertools.product([0, 1], [0, 1], [None, 0, 1], [0, 1], [0, 1])):
        speed = speeds[k % len(speeds)]
        m.w(0xb36550, SESSION_ if session else 0)
        m.w(0xb36558, NODE)
        m.w(NODE + 0x284, PLAYER if player else 0)
        m.w(0xb36534, 0 if world is None else WORLD)
        m.u.mem_write(WORLD + 0x275, bytes([world or 0]))
        m.w(PLAYER + 4, CLIENT if client else 0)
        m.w(NODE + 0x2c4, CLIENT if is_host else OTHER)
        m.w(ORDER + 4, speed)
        m.calls = []
        m.call(0x75d810, [], this=ORDER, callee_pop=0)
        rows.append({'session': bool(session), 'ordering_player': bool(player),
                     'world_cutscene': None if world is None else bool(world),
                     'ordering_client': bool(client), 'client_is_host': bool(is_host),
                     'speed_bits': speed,
                     'set_game_speed': [{'sim': c['this'] == hex(SIM), 'speed_bits': c['speed_bits']} for c in m.calls]})
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = []
    for a, b in ranges:
        raw = PE0.range(a, b - a)
        windows.append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw), 'v14_identical': raw == PE1.range(a, b - a)})
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-speed.json').write_text(json.dumps(report, indent=1) + '\n')
    print(len(rows), 'cases;', sum(bool(r['set_game_speed']) for r in rows), 'reach 0x76F370;',
          'v14 differs at', [w['start'] for w in windows if not w['v14_identical']])

if __name__ == '__main__':
    main()
