"""Original TGC_AdminConn::GetWantedPacketType 0x74BDE0 under Unicorn, with
the session-member test 0x74B600 and the backup rank delay 0x74BD80 executed
natively; the backlog test is a recording seam (0x74BCB0 on the original
image, the v14 cave 0x9EFF40 called from 0x74BEC8 on v14; both are proven in
native-packets.json). Every branch combination of AdminNode +0x2CC/+0x2C4/
+0x2C8/+0x2CD/+0x2C1/+0x70/+0x158/+0x2D8, AdminConn +0x88/+0x89/+0x70/+8 and
NetConn +8/+0x90 against NetNode+0x58, at x87 PC24 and PC53 (FPU control
word 0x007F / 0x027F), with times chosen on and around the 0.02/0.1/0.25 s
thresholds, including values where the two precisions disagree.
"""
from pathlib import Path
import itertools, json, random, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard
from unicorn.x86_const import *

NODE = 0x2040000; CONN = 0x2041000; NETCONN = 0x2042000; NETNODE = 0x2043000; OTHER = 0x2044000; PLAYER = 0x2045000
V14_SECTIONS = (0x401000, 0x9f0000)
PC = {'Single': 0x007f, 'Double': 0x027f}

def fb(x): return struct.unpack('<I', struct.pack('<f', x))[0]

class GateMachine(OrdersMachine):
    def __init__(self, image):
        super().__init__()
        self.mode = 'gate'; self.image = image; self.backlog = 0; self.backlog_calls = 0
        if image == 'v14':
            for vs, r, size, off in PE1.sections:
                if PE1.base + r in V14_SECTIONS: self.u.mem_write(PE1.base + r, PE1.b[off:off + size])
        null_guard.install(self)
        self.u.mem_write(NODE, b'\0' * 0x6000)
        self.w(0xb36558, NODE); self.w(0xb36524, NETNODE); self.w(CONN + 0x414, NETCONN)
    def hook(self, u, pc, size, user):
        if self.mode == 'gate':
            if pc == (0x74bcb0 if self.image == 'original' else 0x9eff40):
                self.backlog_calls += 1; self.ret(0, self.backlog); return
            if pc == 0x404f60:
                self.log.append(self.r(u.reg_read(UC_X86_REG_ESP) + 12)); self.ret(); return
            self.visited[pc] = size  # seams are not executed code
        super().hook(u, pc, size, user)

TIMES = [  # (now, last_send) pairs around the thresholds; PC24/PC53 disagreements included
    (10.0, 9.0), (10.0, 9.75), (10.0, 9.9), (10.0, 9.98), (10.0, 10.0), (0.3, 0.05), (0.12, 0.02),
    (0.0325, 0.0125), (5.0, -1.0), (0.25, 0.0), (0.1, 0.0), (0.02, 0.0), (1e8, 0.1), (100.1, 100.0)]

def main():
    out = {}; windows = {}
    for image in ('original', 'v14'):
        rng = random.Random(77)
        m = GateMachine(image); m.log = []; rows = []
        flags = itertools.product([0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1])
        combos = []
        for connecting, host_sel, server_sel, conn_host, conn_server, leaving, leave_now, disc, connected in flags:
            combos.append(dict(connecting=connecting, we_are_host=host_sel, we_are_server=server_sel, conn_is_host=conn_host,
                               conn_is_server=conn_server, leaving=leaving, leave_now=leave_now, disconnecting=disc, connected=connected))
        for k, c in enumerate(combos):
            for j in range(4):
                c2 = dict(c)
                c2['backlog'] = rng.randrange(2); c2['admin_clock'] = rng.randrange(2); c2['player'] = rng.randrange(2)
                c2['sent_this_pass'] = rng.choice([0, 7, 8, 9, 12])
                c2['node_rank'] = rng.choice([0, 1, 3]); c2['conn_rank'] = rng.choice([0, 1, 2, 3])
                now, last = TIMES[(k * 4 + j) % len(TIMES)]
                c2['now'] = fb(now); c2['last_send'] = fb(last)
                c2['backup_since'] = fb(rng.choice([now - 0.4, now - 1.0, now - 0.5, now, now - 2.0]))
                c2['precision'] = 'Single' if (k + j) % 2 else 'Double'
                rows.append(run(m, c2))
        out[image] = rows
        pe, other = (PE0, PE1) if image == 'original' else (PE1, PE0)
        ranges = []
        for a, size in sorted(m.visited.items()):
            if not pe.base <= a < pe.base + pe.image_size: continue
            if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
            else: ranges.append([a, a + size])
        windows[image] = [{'start': hex(a), 'end': hex(b), 'sha256': sha(pe.range(a, b - a)),
                           'identical_in_other_image': pe.range(a, b - a) == other.range(a, b - a)} for a, b in ranges]
        print(image, 'differs from the other image at', [w['start'] for w in windows[image] if not w['identical_in_other_image']])
        print(image, len(rows), 'cases', {t: sum(r['packet_type'] == t for r in rows) for t in range(12)})
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'images': out,
              'executed_windows': windows}
    (HERE / 'native-send-gate.json').write_text(json.dumps(report) + '\n')

def run(m, c):
    m.w(NODE + 0x2c4, NODE if c['we_are_host'] else (CONN if c['conn_is_host'] else OTHER))
    m.w(NODE + 0x2c8, NODE if c['we_are_server'] else (CONN if c['conn_is_server'] else OTHER))
    m.u.mem_write(NODE + 0x2cc, bytes([c['connecting'], c['leaving']]))
    m.u.mem_write(NODE + 0x2c1, bytes([c['leave_now']]))
    m.w(NODE + 0x70, c['node_rank']); m.w(NODE + 0x158, PLAYER if c['admin_clock'] else 0); m.w(NODE + 0x2d8, c['backup_since'])
    m.u.mem_write(CONN + 0x88, bytes([c['connected'], c['disconnecting']]))
    m.w(CONN + 0x70, c['conn_rank']); m.w(CONN + 8, PLAYER if c['player'] else 0)
    m.w(NETCONN + 8, c['sent_this_pass']); m.w(NETCONN + 0x90, c['last_send']); m.w(NETNODE + 0x58, c['now'])
    m.u.reg_write(UC_X86_REG_FPCW, PC[c['precision']])
    m.backlog = c['backlog']; m.backlog_calls = 0; m.log = []
    t = m.call(0x74bde0, [], this=CONN, callee_pop=0)
    return dict(c, packet_type=t, backlog_called=m.backlog_calls, log=m.log)

if __name__ == '__main__':
    main()
