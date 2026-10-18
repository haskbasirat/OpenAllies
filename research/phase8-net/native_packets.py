"""Original ClientToServer / ServerToClient packet bodies end to end, with
two isolated emulated nodes (host and client), each running the ORIGINAL
TGC_AdminConn constructor 0x756BC0 (all channels, FileSender/Receiver),
channel bindings ResetPermanentSendersToClient 0x752490 (host) and
ResetPermanentSendersToServer 0x7525B0 (client), writers 0x74E280 / 0x74E0F0,
readers 0x75A350 / 0x74E170 (incl. status block 0x74C6D0/0x74C7B0, file
generation check 0x74C8B0) and the ack dispatcher 0x74B4F0.

The host appends decree packets to its decree history (AdminNode+0x214), the
client appends request packets to its request history (+0xC); datagram bodies
are copied between the machines, some are dropped, and ack/loss results are
delivered through 0x74B4F0 in native order. Recorded: every body's bytes and
end position, reader end positions, the client's decree history and the
host's per-connection request history (AdminConn+0xC) after each step.
Seams: allocator, CString manager, logging, debug name formatting; AdminNode
+0x158 (admin clock) is null so the client-clock update 0x75FB40 is skipped.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_channels import ChannelMachine, finalized, READER_VT, WRITER_VT, PE0, PE1, sha
from unicorn.x86_const import *

NODE = 0x2020000; CONN = 0x2021000; NETCONN = 0x2022000; AVARS = 0x2023000; SESSION_ = 0x2024000; NETNODE = 0x2025000
AVAR_VALUES = {0x120: 20, 0x124: 64, 0x128: 256, 0x12c: 32, 0x130: 32, 0x134: 2, 0x138: 4, 0x13c: 64, 0x140: 32}

# .text/.rdata of the accepted v14 image loaded over the original for the v14 runs
# (as native_file_transfer.py); .data keeps the Phase3/4 machine initialisation.
V14_SECTIONS = (0x401000, 0x9f0000)

class NodeMachine(ChannelMachine):
    def __init__(self, host, image='original'):
        super().__init__()
        self.image = image
        if image == 'v14':
            for vs, r, size, off in PE1.sections:
                if PE1.base + r in V14_SECTIONS: self.u.mem_write(PE1.base + r, PE1.b[off:off + size])
        self.u.mem_write(NODE, b'\0' * 0x1000 * 6)
        # AVar pointer table [0xB39990]: every entry -> int (native defaults for the used ones).
        vals = self.alloc(0x400)
        for off in range(0, 0x400, 4):
            v = vals + off; self.w(v, AVAR_VALUES.get(off, 0)); self.w(AVARS + off, v)
        self.w(0xb39990, AVARS)
        self.w(0xb36558, NODE); self.w(0xb36550, SESSION_); self.w(0xb36524, NETNODE)
        self.w(0xb43b40, self.string(''))  # static empty CString (file channel names)
        for off, cap in [(0xc, 64), (0x1c, 64), (0x214, 64), (0x224, 64)]:
            self.w(NODE + off, 0); self.w(NODE + off + 4, 0); self.w(NODE + off + 8, cap); self.w(NODE + off + 12, self.alloc(4 * cap))
        self.w(NETCONN + 0x34, 0xffffffff); self.w(NETCONN + 0xa0, CONN)
        self.call(0x756bc0, [NETCONN], this=CONN, callee_pop=4)
        self.u.mem_write(CONN + 0x88, b'\x01')
        # AdminConn+8: the connection's session client (only its name +8 is read, for logging).
        client = self.alloc(0x40); self.w(client + 8, self.string('peer')); self.w(CONN + 8, client)
        if host:
            self.w(NODE + 0x2c4, NODE); self.w(NODE + 0x2c8, NODE)
        else:
            self.w(NODE + 0x2c4, CONN); self.w(NODE + 0x2c8, CONN)
    def append(self, off, packet):
        base = NODE + off
        begin = struct.unpack('<i', self.u.mem_read(base, 4))[0]; end = struct.unpack('<i', self.u.mem_read(base + 4, 4))[0]
        cap = self.r(base + 8); assert end - begin < cap
        slot = ((end + 0x80000000) & 0xffffffff) % cap
        self.w(self.r(base + 12) + 4 * slot, self.packet(list(packet), refs=1)); self.w(base + 4, (end + 1) & 0xffffffff)
    def append_conn(self, off, packet):
        base = CONN + off
        begin = struct.unpack('<i', self.u.mem_read(base, 4))[0]; end = struct.unpack('<i', self.u.mem_read(base + 4, 4))[0]
        cap = self.r(base + 8); assert end - begin < cap
        slot = ((end + 0x80000000) & 0xffffffff) % cap
        self.w(self.r(base + 12) + 4 * slot, self.packet(list(packet), refs=1)); self.w(base + 4, (end + 1) & 0xffffffff)
    def history(self, base):
        return self.read_history(base)

def body(m, fn, seq):
    W = m.stream(WRITER_VT)
    m.call(fn, [W, seq], this=CONN, callee_pop=8)
    end = m.position(W); bits = end['byte'] * 8 + end['bit']
    return list(m.u.mem_read(m.r(W + 4), (bits + 7) // 8)), end

def feed(m, fn, data, seq, highest):
    m.w(NETCONN + 0x34, highest)
    R = m.stream(READER_VT, data)
    m.call(fn, [R, seq], this=CONN, callee_pop=8)
    return m.position(R)

def generation(seq):
    """Host file generation (AdminConn+0x418) per step: newer, older and equal cases of 0x74C8B0."""
    if seq == 8: return 0
    if seq >= 12: return 2
    if seq >= 5: return 1
    return 0

def backlog(m):
    """The send gate's backlog test: 0x74BCB0 on the original image; on v14 the call at
    0x74BEC8 goes to the cave 0x9EFF40 (-> 0x9EFEC0 -> 0x74BCB0)."""
    return m.call(0x74bcb0 if m.image == 'original' else 0x9eff40, [], this=CONN, callee_pop=0) & 0xff

def scenario(rng, image, name, start, steps, drop_down, drop_up, late, decree_bits, request_bits, ack_delay, private_at=()):
    """`late` sequences are delivered one step later (after the next datagram), so they arrive not most recent.
    `decree_bits[i]` is one packet size or a list of sizes appended before step i."""
    host = NodeMachine(True, image); client = NodeMachine(False, image)
    # As TGC_AdminConn host join completion 0x753580: ResetBackupSendersToClient then
    # ResetPermanentSendersToClient(start).
    host.call(0x752300, [], this=CONN, callee_pop=0)
    host.call(0x752490, [start], this=CONN, callee_pop=4)
    client.call(0x7525b0, [], this=CONN, callee_pop=0)
    rows = []; pend_h = []; pend_c = []; held = []
    high_c = -1; high_h = -1
    dec = [[finalized([rng.getrandbits(1) for _ in range(n)]) for n in (d if isinstance(d, list) else [d])] for d in decree_bits]
    req = [finalized([rng.getrandbits(1) for _ in range(n)]) for n in request_bits]
    # The host's decree history starts at `start` (bound sender start index).
    host.w(NODE + 0x214, start & 0xffffffff); host.w(NODE + 0x218, start & 0xffffffff)
    for seq in range(steps):
        added_d = []; added_r = None; added_p = None
        if seq < len(dec):
            for d in dec[seq]: host.append(0x214, d); added_d.append(list(d))
        if seq in private_at:
            pkt = finalized([rng.getrandbits(1) for _ in range(rng.randrange(0, 600))])
            host.append_conn(0xac, pkt); added_p = list(pkt)
        if seq < len(req):
            client.append(0xc, req[seq]); added_r = list(req[seq])
        gen = generation(seq); host.w(CONN + 0x418, gen)
        word = (seq * 2654435761) & 0xffffffff; behind = seq % 3 == 0
        client.w(CONN + 0x60, word); client.u.mem_write(NETNODE + 0x64, bytes([int(behind)]))
        backlog_before = backlog(host)
        down, down_end = body(host, 0x74e280, seq)
        up, up_end = body(client, 0x74e0f0, seq)
        deliveries = []
        # A held (late) datagram arrives after the next one, i.e. not most recent.
        queue = ([(seq, down, up)] if seq not in late else []) + held
        held = [(seq, down, up)] if seq in late else []
        for s, d, u in queue:
            row = {'sequence': s}
            if s not in drop_down:
                high_c = max(high_c, s)
                row['down_most_recent'] = s == high_c
                row['down_read_end'] = feed(client, 0x75a350, d, s, high_c)
            if s not in drop_up:
                high_h = max(high_h, s)
                row['up_most_recent'] = s == high_h
                row['up_read_end'] = feed(host, 0x74e170, u, s, high_h)
            deliveries.append(row)
        pend_h.append((seq, seq not in drop_down)); pend_c.append((seq, seq not in drop_up))
        results_h = []; results_c = []
        while len(pend_h) > ack_delay:
            s, ok = pend_h.pop(0); host.call(0x74b4f0, [s, int(ok)], this=CONN, callee_pop=8); results_h.append([s, ok])
        while len(pend_c) > ack_delay:
            s, ok = pend_c.pop(0); client.call(0x74b4f0, [s, int(ok)], this=CONN, callee_pop=8); results_c.append([s, ok])
        rows.append({'sequence': seq, 'added_decrees': added_d, 'added_request': added_r, 'added_private': added_p,
                     'backlog_before': backlog_before, 'backlog_after': backlog(host),
                     'client_private': client.history(CONN + 0xac),
                     'generation': gen, 'status_word': word, 'falling_behind': behind,
                     'down': down, 'down_end': down_end, 'up': up, 'up_end': up_end,
                     'deliveries': deliveries,
                     'host_results': results_h, 'client_results': results_c,
                     'client_decrees': client.history(NODE + 0x214),
                     'client_file_generation': client.signed(CONN + 0x418),
                     'client_backup_index': client.signed(CONN + 0xa0), 'client_backup_decree_next': client.signed(NODE + 0x234),
                     'host_conn_requests': host.history(CONN + 0xc),
                     'host_conn_status': {'word': host.r(CONN + 0x60), 'backlog': host.u.mem_read(CONN + 0xa8, 1)[0],
                                          'falling_behind': host.u.mem_read(CONN + 0xa9, 1)[0]}})
    visited = {**host.visited, **client.visited}
    return {'name': name, 'start': start, 'steps': rows}, visited

def main():
    images = {}; visited = {}
    burst = [[30] * 6 if i % 5 == 0 else [] for i in range(30)]
    for image in ('original', 'v14'):
        rng = random.Random(54); out = []
        for args in [('lossless', 0, 10, set(), set(), set(), [20, 0, 300, 5, 700, 64], [10, 3, 400, 1], 1),
                     ('losses', 4, 24, {2, 3, 9}, {1, 5, 6}, {11, 15}, [100, 900, 250, 255, 256, 1, 60, 500], [200, 30, 800, 9, 9], 2),
                     ('long', 0, 30, {7, 8, 9, 20}, {12}, {3, 17}, [40 * (i % 7) for i in range(25)], [33 * (i % 5) for i in range(20)], 3),
                     ('request_backlog', 0, 52, {4}, set(range(0, 38)), {44}, [8] * 6, [8] * 45, 1),
                     ('decree_burst_private', 2, 30, {1, 2, 3, 11}, {6}, {8}, burst, [5] * 10, 2, (0, 1, 4, 9, 10, 17))]:
            row, v = scenario(rng, image, *args); out.append(row); visited.update(v)
        images[image] = out
    ranges = []
    for a, size in sorted(visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = []
    for a, b in ranges:
        # The v14 runtime patches some of these windows (e.g. 0x74E2B6): record, do not assert.
        raw = PE0.range(a, b - a)
        windows.append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw), 'v14_identical': raw == PE1.range(a, b - a)})
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'avars': {hex(k): v for k, v in AVAR_VALUES.items()},
              'producer_sha256': sha(Path(__file__).read_bytes()), 'images': images, 'executed_windows': windows}
    (HERE / 'native-packets.json').write_text(json.dumps(report) + '\n')
    print({k: (len(v), sum(len(s['steps']) for s in v)) for k, v in images.items()}, 'scenarios/steps;', 'v14 differs at', [w['start'] for w in windows if not w['v14_identical']])

if __name__ == '__main__':
    main()
