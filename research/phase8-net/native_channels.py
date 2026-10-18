"""Original message-history channels under Unicorn: TGC_MessageHistorySender
(ctor 0x763520, bind 0x763600) on TGC_BlockSender (serialize 0x760A80, packet
result 0x7608E0) and TGC_MessageHistoryReceiver (ctor 0x7636B0, bind 0x763130,
section read 0x760F20, skip 0x75FEA0), with the native block windows,
blockify 0x762D30, header 0x762BD0, block codecs 0x761FA0/0x7633C0/0x75FFA0/
0x7610A0 and reassembly 0x7632D0/0x7631D0/0x7626F0.

Scenarios move a sender-side packet history through sections of various bit
limits, drop some datagrams, deliver acks/losses in native order, grow the
history mid-run, and replay received sections into a receiver-side history.
Seams: allocator, CString manager and logging (Phase3/4 machines).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard
from unicorn.x86_const import *

READER_VT = 0xa39da4; WRITER_VT = 0xa39da8

def finalized(bits):
    """Finalized bitstream bytes for a list of body bits (3-bit pad header)."""
    total = 3 + len(bits); n = (total + 7) // 8; out = bytearray(n)
    for i, b in enumerate(bits):
        p = 3 + i
        if b: out[p // 8] |= 1 << (p % 8)
    out[0] |= (8 - total % 8) & 7
    return bytes(out)

class ChannelMachine(OrdersMachine):
    def __init__(self):
        super().__init__()
        self.mode = 'channels'
        null_guard.install(self)
    def hook(self, u, pc, size, user):
        if self.mode == 'channels':
            self.visited[pc] = size
            if pc in (0x404f60, 0x404f90):
                self.ret(); return
            if pc == 0x405d60:  # CString::Format of a debug name in 0x760F20 (no wire effect)
                self.ret(); return
            if pc == 0x405a50:  # CString assignment of that debug name
                self.ret(8); return
        super().hook(u, pc, size, user)
    def call(self, pc, args=(), this=None, callee_pop=0):
        # Same contract as the Phase3 machine with a larger step budget: blocks
        # are copied one bit per native call (0x75FF50).
        self.w(self.stack, self.stop)
        for i, x in enumerate(args): self.w(self.stack + 4 + 4 * i, x)
        self.u.reg_write(UC_X86_REG_ESP, self.stack); self.u.reg_write(UC_X86_REG_ECX, self.admin if this is None else this)
        regs = {UC_X86_REG_EBX: 0xabcdef01, UC_X86_REG_ESI: 0xabcdef02, UC_X86_REG_EDI: 0xabcdef03, UC_X86_REG_EBP: 0xabcdef04}
        for r, v in regs.items(): self.u.reg_write(r, v)
        self.u.emu_start(pc, self.stop, count=50_000_000)
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))
        assert self.u.reg_read(UC_X86_REG_ESP) == self.stack + 4 + callee_pop, (hex(pc), self.u.reg_read(UC_X86_REG_ESP))
        for r, v in regs.items(): assert self.u.reg_read(r) == v, (hex(pc), r)
        return self.u.reg_read(UC_X86_REG_EAX)
    def windowed(self, begin, packets, capacity=16):
        base = self.alloc(16); data = self.alloc(4 * capacity)
        self.w(base, begin & 0xffffffff); self.w(base + 4, (begin + len(packets)) & 0xffffffff)
        self.w(base + 8, capacity); self.w(base + 12, data)
        for i, p in enumerate(packets):
            slot = ((begin + i + 0x80000000) & 0xffffffff) % capacity
            self.w(data + 4 * slot, self.packet(list(p), refs=1) if p is not None else 0)
        return base
    def append(self, base, packet):
        begin = struct.unpack('<i', self.u.mem_read(base, 4))[0]; end = struct.unpack('<i', self.u.mem_read(base + 4, 4))[0]
        cap = self.r(base + 8)
        assert end - begin < cap
        slot = ((end + 0x80000000) & 0xffffffff) % cap
        self.w(self.r(base + 12) + 4 * slot, self.packet(list(packet), refs=1)); self.w(base + 4, (end + 1) & 0xffffffff)
    def read_history(self, base):
        s = lambda p: struct.unpack('<i', self.u.mem_read(p, 4))[0]
        begin, end, cap, data = s(base), s(base + 4), self.r(base + 8), self.r(base + 12)
        out = []
        for i in range(begin, end):
            p = self.r(data + 4 * (((i + 0x80000000) & 0xffffffff) % cap))
            out.append(list(self.u.mem_read(p, self.r(p - 4))) if p else None)
        return {'begin': begin, 'end': end, 'packets': out}
    def stream(self, vt, data=None, size=0x1000):
        obj = self.alloc(16)
        buf = self.packet(list(data) + [0] * 8 if data is not None else [0] * size, refs=1)
        self.w(obj, vt); self.w(obj + 4, buf); self.w(obj + 8, 0); self.w(obj + 12, 3)
        return obj
    def sender_state(self, S):
        s = lambda p: struct.unpack('<i', self.u.mem_read(p, 4))[0]
        pb, pe, pc, pd = s(S + 4), s(S + 8), self.r(S + 0xc), self.r(S + 0x10)
        packets = []
        for i in range(pb, pe):
            r = pd + (((i + 0x80000000) & 0xffffffff) % pc) * 16
            packets.append([s(r), s(r + 4), s(r + 8), self.r(r + 12)])
        bb, be, bc, bd = s(S + 0x14), s(S + 0x18), self.r(S + 0x1c), self.r(S + 0x20)
        blocks = []
        for i in range(bb, be):
            r = bd + (((i + 0x80000000) & 0xffffffff) % bc) * 12
            blocks.append([s(r), s(r + 4)])
        hb, he, hc, hd = s(S + 0x30), s(S + 0x34), self.r(S + 0x38), self.r(S + 0x3c)
        hblocks = []
        for i in range(hb, he):
            r = hd + (((i + 0x80000000) & 0xffffffff) % hc) * 16
            hblocks.append([s(r), self.u.mem_read(r + 4, 1)[0], s(r + 8)])
        return {'packets_begin': pb, 'packets': packets, 'blocks_begin': bb, 'blocks': blocks,
                'history_blocks_begin': hb, 'history_blocks': hblocks, 'next': s(S + 0x44)}
    def receiver_state(self, R):
        s = lambda p: struct.unpack('<i', self.u.mem_read(p, 4))[0]
        return {'blocks_begin': s(R + 0xc), 'blocks_end': s(R + 0x10)}

def run_scenario(m, rng, name, packet_bits, start, limits, drop, ack_delay, grow_at):
    # Native statics (selection scratch 0xB40AA4, block buffers 0xB40AB4/0xB40ABC/0xB40AAC) keep heap pointers: never reset the heap.
    initial = [finalized([rng.getrandbits(1) for _ in range(n)]) for n in packet_bits[:3]]
    later = [finalized([rng.getrandbits(1) for _ in range(n)]) for n in packet_bits[3:]]
    H = m.windowed(start, initial, capacity=64)
    H2 = m.windowed(0, [], capacity=64)
    S = m.alloc(0x60); m.call(0x763520, [m.string('send')], this=S, callee_pop=4)
    m.call(0x763600, [H, start], this=S, callee_pop=8)
    R = m.alloc(0x40); m.call(0x7636b0, [m.string('recv')], this=R, callee_pop=4)
    m.call(0x763130, [H2], this=R, callee_pop=4)
    steps = []; pending = []
    for seq, limit in enumerate(limits):
        if seq == grow_at:
            for p in later: m.append(H, p)
        W = m.stream(WRITER_VT)
        m.call(0x760a80, [W, seq, limit], this=S, callee_pop=12)
        end = m.position(W); bits = end['byte'] * 8 + end['bit']
        data = list(m.u.mem_read(m.r(W + 4), (bits + 7) // 8))
        delivered = seq not in drop
        if delivered:
            Rd = m.stream(READER_VT, data)
            m.call(0x760f20, [Rd, seq], this=R, callee_pop=8)
            read_end = m.position(Rd)
        else:
            read_end = None
        pending.append((seq, delivered))
        results = []
        while len(pending) > ack_delay:
            s0, ok = pending.pop(0)
            m.call(0x7608e0, [s0, int(ok)], this=S, callee_pop=8)
            results.append([s0, ok])
        steps.append({'sequence': seq, 'limit': limit, 'grow': seq == grow_at, 'bytes': data, 'end': end,
                      'delivered': delivered, 'read_end': read_end, 'results': results,
                      'sender': m.sender_state(S), 'receiver': m.receiver_state(R), 'history': m.read_history(H2)})
    return {'name': name, 'start': start, 'initial': [list(p) for p in initial], 'later': [list(p) for p in later], 'steps': steps}

def main():
    m = ChannelMachine(); m.mark = m.heap; rng = random.Random(31); rows = []
    rows.append(run_scenario(m, rng, 'small_lossless', [5, 0, 40, 9], 0, [0x400] * 6, set(), 0, 2))
    rows.append(run_scenario(m, rng, 'multi_block_packets', [255, 256, 600, 1000, 30], 7, [0x400] * 14, set(), 1, 3))
    rows.append(run_scenario(m, rng, 'tight_budget', [120, 300, 80, 255], 0, [0x60, 0x80, 0xa0, 0x100, 0x140, 0x200] * 3, set(), 0, 5))
    rows.append(run_scenario(m, rng, 'losses_and_retransmit', [200, 700, 64, 1, 900], 3, [0x300] * 20, {1, 2, 5, 6, 9}, 2, 4))
    rows.append(run_scenario(m, rng, 'deep_ack_delay', [300, 300, 300, 300, 300, 300], 0, [0x200] * 40, {3, 4, 10}, 5, 8))
    rows.append(run_scenario(m, rng, 'start_negative_index', [17, 18], -5, [0x400] * 5, set(), 0, 1))
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
              'producer_sha256': sha(Path(__file__).read_bytes()), 'scenarios': rows, 'executed_windows': windows}
    (HERE / 'native-channels.json').write_text(json.dumps(report) + '\n')
    print(len(rows), 'scenarios', sum(len(r['steps']) for r in rows), 'steps')

if __name__ == '__main__':
    main()
