"""Original TGC_NetConn datagram header writer/reader under Unicorn.

Executes the ORIGINAL 0x764DA0 (session ids) -> 0x764CA0 (sequence, ack,
ack bits) -> 0x764AF0 (sent-time record) writer and the 0x764BB0 -> 0x764600
-> 0x764550 reader, including the native windowed-array helpers 0x7644B0,
0x763FD0, 0x763D00, 0x750990, 0x74ECD0 and the x87 RTT arithmetic. Seams:
allocator/logging (Phase3 machine), admin payload writer 0x7521F0 (records
the sequence argument, writes nothing), AdminConn ack results 0x74B4F0 and
admin dispatch 0x75CD20 (recorded), statistics 0x77C820. FPU control word is
set explicitly per case (PC24 / PC53 / PC64).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]; GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/phase3-orders'))
from original_orders import Machine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard
from unicorn.x86_const import *

NODE = 0x2010000; CONN = 0x2011000; ADMINCONN = 0x2012000
PC = {'Single': 0x007f, 'Double': 0x027f, 'Extended': 0x037f}

class NetMachine(Machine):
    def __init__(self):
        super().__init__()
        self.visited = {}
        null_guard.install(self)
        self.w(0xb36524, NODE)
        self.log = []; self.acks = []; self.dispatched = []; self.payload_seq = []
    def f(self, p, v): self.u.mem_write(p, struct.pack('<f', v))
    def rf(self, p): return struct.unpack('<f', self.u.mem_read(p, 4))[0]
    def rfb(self, p): return self.r(p)
    def hook(self, u, pc, size, user):
        self.visited[pc] = size
        if pc == 0x404f60:
            self.log.append(self.arg(2)); self.ret(); return
        if pc == 0x7521f0:
            self.payload_seq.append(self.arg(1)); self.ret(8); return
        if pc == 0x74b4f0:
            self.acks.append([self.signed_arg(0), self.arg(1) & 0xff]); self.ret(8); return
        if pc == 0x75cd20:
            self.dispatched.append(self.signed_arg(1)); self.ret(8); return
        if pc == 0x77c820:
            self.ret(4); return
        super().hook(u, pc, size, user)
    def signed_arg(self, n): return struct.unpack('<i', struct.pack('<I', self.arg(n)))[0]
    def window(self, base, begin, end, capacity, elem, values):
        data = self.alloc(capacity * elem + 16)
        self.w(base, begin); self.w(base + 4, end); self.w(base + 8, capacity); self.w(base + 12, data)
        for i, v in values.items():
            slot = ((i + 0x80000000) & 0xffffffff) % capacity
            if elem == 4: self.f(data + slot * 4, v)
            else: self.u.mem_write(data + slot, bytes([v]))
    def conn(self, st):
        self.u.mem_write(CONN, b'\0' * 0xa4)
        self.w(CONN, st['peer_session'] & 0xffffffff)
        sent = st['sent']; rec = st['received']
        self.window(CONN + 0xc, sent['begin'], sent['begin'] + len(sent['times']), 256, 4,
                    {sent['begin'] + i: t for i, t in enumerate(sent['times'])})
        self.window(CONN + 0x1c, rec['begin'], rec['begin'] + len(rec['flags']), 256, 1,
                    {rec['begin'] + i: int(t) for i, t in enumerate(rec['flags'])})
        self.f(CONN + 0x2c, st['last_advance']); self.w(CONN + 0x30, st['last_sent'] & 0xffffffff)
        self.w(CONN + 0x34, st['highest'] & 0xffffffff); self.f(CONN + 0x38, st['rtt'])
        self.w(CONN + 8, st['sent_this_pass']); self.f(CONN + 0x90, st['last_send'])
        self.w(CONN + 0xa0, ADMINCONN)
    def read_window(self, base, elem):
        begin = struct.unpack('<i', self.u.mem_read(base, 4))[0]; end = struct.unpack('<i', self.u.mem_read(base + 4, 4))[0]
        cap = self.r(base + 8); data = self.r(base + 12); out = []
        for i in range(begin, end):
            slot = ((i + 0x80000000) & 0xffffffff) % cap
            out.append(self.rf(data + slot * 4) if elem == 4 else bool(self.u.mem_read(data + slot, 1)[0]))
        return {'begin': begin, 'times' if elem == 4 else 'flags': out}
    def state(self):
        s = lambda p: struct.unpack('<i', self.u.mem_read(p, 4))[0]
        return {'peer_session': s(CONN), 'sent': self.read_window(CONN + 0xc, 4), 'received': self.read_window(CONN + 0x1c, 1),
                'last_advance': self.rf(CONN + 0x2c), 'last_sent': s(CONN + 0x30), 'highest': s(CONN + 0x34),
                'rtt': self.rf(CONN + 0x38), 'sent_this_pass': self.r(CONN + 8), 'last_send': self.rf(CONN + 0x90)}
    def fpu(self, precision):
        self.u.reg_write(UC_X86_REG_FPCW, PC[precision])
    def node(self, own, now):
        self.w(NODE + 0xb0, own & 0xffffffff); self.f(NODE + 0x58, now)

def fresh():
    return {'peer_session': -1, 'sent': {'begin': 0, 'times': []}, 'received': {'begin': 0, 'flags': []},
            'last_advance': 0.0, 'last_sent': -1, 'highest': -1, 'rtt': 0.0, 'sent_this_pass': 0, 'last_send': -1.0}

def write_case(m, st, own, now, precision):
    m.conn(st); m.node(own, now); m.payload_seq = []
    p = m.packet([0] * 256); m.w(m.reader, 0xa39da8); m.w(m.reader + 4, p); m.w(m.reader + 8, 0); m.w(m.reader + 12, 3)
    m.fpu(precision)
    m.call(0x764da0, [m.reader], this=CONN, callee_pop=4)
    end = m.position(m.reader); n = (end['byte'] * 8 + end['bit'] + 7) // 8
    return {'kind': 'write', 'precision': precision, 'own_session': own, 'now': now, 'before': st,
            'bytes': list(m.u.mem_read(m.r(m.reader + 4), n)), 'end': end, 'payload_sequence': m.payload_seq,
            'after': m.state()}

def read_case(m, st, own, now, precision, data):
    m.conn(st); m.node(own, now); m.log = []; m.acks = []; m.dispatched = []
    p = m.packet(list(data) + [0] * 8); m.w(m.reader, 0xa39da4); m.w(m.reader + 4, p); m.w(m.reader + 8, 0); m.w(m.reader + 12, 3)
    m.fpu(precision)
    m.call(0x764bb0, [m.reader], this=CONN, callee_pop=4)
    return {'kind': 'read', 'precision': precision, 'own_session': own, 'now': now, 'before': st, 'bytes': list(data),
            'end': m.position(m.reader), 'log_lines': m.log, 'acks': m.acks, 'dispatched': m.dispatched, 'after': m.state()}

def main():
    m = NetMachine(); rng = random.Random(8); rows = []
    # Writer: fresh, steady, ack < 32, ack window partially known, gaps.
    states = []
    s = fresh(); states.append(s)
    for highest in [0, 5, 31, 32, 33, 40, 300]:
        rb = max(0, highest - 40)
        flags = [rng.random() < 0.7 for _ in range(highest - rb + 1)]
        states.append({**fresh(), 'peer_session': 0x11223344, 'highest': highest, 'last_sent': rng.randrange(-1, 250),
                       'received': {'begin': rb, 'flags': flags}, 'sent': {'begin': 0, 'times': [0.5 * i for i in range(5)]},
                       'rtt': 0.125, 'last_send': 1.0})
    for st in states:
        for precision in ['Single', 'Double']:
            rows.append(write_case(m, st, 0x55667788, 12.5, precision))
    # Reader: packets produced by the native writer of a peer state, fed to receiver states.
    def packet_from(peer_state, peer_own):
        r = write_case(m, peer_state, peer_own, 3.0, 'Single')
        return r['bytes']
    receivers = []
    receivers.append(fresh())
    receivers.append({**fresh(), 'peer_session': 7, 'highest': 10, 'received': {'begin': 0, 'flags': [True] * 11},
                      'sent': {'begin': 0, 'times': [1.0 + 0.0625 * i for i in range(12)]}, 'last_sent': 11, 'rtt': 0.25})
    receivers.append({**fresh(), 'peer_session': 7, 'highest': 100, 'received': {'begin': 60, 'flags': [True] * 41},
                      'sent': {'begin': 90, 'times': [2.0 + 0.01 * i for i in range(30)]}, 'last_sent': 119, 'rtt': 0.3})
    for own in [0x7, 0x0badf00d]:
        for rec in receivers:
            for peer_highest, peer_last in [(-1, -1), (-1, 3), (5, 8), (11, 8), (30, 12), (96, 99), (95, 100), (100, 120), (119, 150), (200, 10)]:
                peer = {**fresh(), 'peer_session': own if peer_highest % 3 else -1, 'highest': peer_highest, 'last_sent': peer_last,
                        'received': {'begin': max(0, peer_highest - 40), 'flags': [rng.random() < 0.6 for _ in range(peer_highest - max(0, peer_highest - 40) + 1)] if peer_highest >= 0 else []}}
                data = packet_from(peer, 0x31337)
                for precision, now in [('Single', 2.25), ('Double', 2.25), ('Single', 9.0)]:
                    try:
                        rows.append(read_case(m, rec, own, now, precision, data))
                    except Exception as e:
                        print('FAIL', own, rec['highest'], peer_highest, peer_last, precision, now, data, repr(e)); raise
    # Malformed headers: no-session bit, invalid sender (-1), invalid target.
    rows.append(read_case(m, receivers[1], 7, 2.0, 'Single', [0x00] * 12))
    rows.append(read_case(m, receivers[1], 7, 2.0, 'Single', packet_from({**fresh(), 'peer_session': 7}, -1)))
    rows.append(read_case(m, receivers[1], 7, 2.0, 'Single', packet_from({**fresh(), 'peer_session': 0x1234}, 0x99)))
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = []
    for a, b in ranges:
        raw = PE0.range(a, b - a); assert raw == PE1.range(a, b - a), hex(a)
        windows.append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw)})
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-transport.json').write_text(json.dumps(report, indent=1) + '\n')
    print(len(rows), 'cases', sum(r['kind'] == 'read' for r in rows), 'reads')

if __name__ == '__main__':
    main()
