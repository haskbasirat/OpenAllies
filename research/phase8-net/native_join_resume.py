"""End-to-end original join resume under Unicorn: does a joining client's
synchronizer A stay aligned with the host's?

Host (one machine): the ORIGINAL request dispatcher 0x74D710 runs a request
packet [54 SetGameSpeed, 306 PlayerJoin, 54 SetGameSpeed, ...] through the
real 357-entry registry; every ProcessRequest handler writes its decree into
the decree stream AdminNode+0x1F8 and logs to the ORIGINAL synchronizers
(0x77CCB0 objects; loggers 0x77DA70 / 0x77DB10 via 0x40F8D0 / 0x40F950 run
natively). The PlayerJoin process 0x75DA80 is the capture point of the host
join completion 0x753580 (which runs from it): AdminNode+0x200/+0x204 (the
admin-state message's resume pair, 0x74C130), +0x208 and synchronizer A
(0x77CBE0 = end - 1, 0x573580 = hash) are recorded there, together with the
decree stream object's own byte/bit fields.

Client (a second machine): synchronizer A reset with the captured (index,
hash) by the ORIGINAL 0x77D900 (as 0x74DF50 does), decree history +0x214
holding the host's decree packet, +0x20C = its index, +0x248/+0x24C = the
captured pair, +0x250 = captured +0x208; then the ORIGINAL client replay
0x74FD10. Control: the same client resuming at the packet start (-1/-1).

Seams: allocator / CString / logging / thread id (simulation thread), order
processes (0x75D810 speed, 0x75DA80 join, 0x75D710 SetPlayer: recorded,
they are game/lobby effects), address interning 0x783870 (phase8-orders
TraceMachine environment).
"""
from pathlib import Path
import json, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-orders'))
from native_order_flow import FlowMachine, SYNC_A, SYNC_S
from native_order_traces import PE0, PE1, sha, WRITER_VTABLE
from unicorn.x86_const import *

THREAD = 0x7601000; THREAD_ID = 0x1234
LOGGERS = {0x77da70: 'int', 0x77d930: 'float', 0x77d9d0: 'bool', 0x77db10: 'text'}
PROCESS = {0x75d810: 'speed', 0x75da80: 'player_join', 0x75d710: 'set_player'}
HOST_PLAYER_ID = 7

def s32(v): return struct.unpack('<i', struct.pack('<I', v & 0xffffffff))[0]

class ResumeMachine(FlowMachine):
    def __init__(self):
        super().__init__()
        self.flow_mode = False; self.extent = (0, 0); self.step_limit = 1 << 60; self.steps = 0
        self.reads, self.writes, self.events = [], [], []; self.pending = None; self.recent = []
        self.out_stream = 0; self.rng = None
        self.u.mem_map(THREAD, 0x1000)
        self.w(0x9f01e4, THREAD); self.w(0xb39828, THREAD_ID); self.w(0xb36528, 0)
        self.log = []; self.captures = []; self.processes = []
        for s in (SYNC_A, SYNC_S):
            self.u.mem_write(s, b'\0' * 0x40)
            self.ncall(0x77ccb0, [0x10000], s, 4)
    def ncall(self, pc, args, this, pop):
        self.w(self.stack, self.stop)
        for i, x in enumerate(args): self.w(self.stack + 4 + 4 * i, x)
        self.u.reg_write(UC_X86_REG_ESP, self.stack); self.u.reg_write(UC_X86_REG_ECX, this)
        self.u.emu_start(pc, self.stop, count=0)
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))
        assert self.u.reg_read(UC_X86_REG_ESP) == self.stack + 4 + pop, hex(pc)
        assert not self.null_accesses, self.null_accesses
        return self.u.reg_read(UC_X86_REG_EAX)
    def sync(self, s):
        return {'end': s32(self.r(s + 0x14)), 'hash': s32(self.r(s + 8)), 'enabled': self.u.mem_read(s, 1)[0]}
    def hook(self, u, pc, size, user):
        if pc == THREAD:
            self.ret(0, THREAD_ID); return
        if pc in LOGGERS:
            this = u.reg_read(UC_X86_REG_ECX)
            name = 'A' if this == SYNC_A else 'S' if this == SYNC_S else hex(this)
            value = self.arg(1) if LOGGERS[pc] != 'text' else self.text(self.arg(1))
            self.log.append({'sync': name, 'kind': LOGGERS[pc], 'label': self.text(self.arg(0)), 'value': value,
                             'hash_before': self.sync(this)['hash'] if this in (SYNC_A, SYNC_S) else None})
            self.visited[pc] = size
            return  # the ORIGINAL logger runs
        if pc in (0x40f8d0, 0x40f950):
            self.visited[pc] = size
            return  # original wrappers -> 0x77DB10
        if pc in PROCESS:
            kind = PROCESS[pc]
            e = {'process': kind, 'this': u.reg_read(UC_X86_REG_ECX)}
            if kind == 'player_join':
                a = self.admin
                e.update({'admin_200': self.r(a + 0x200), 'admin_204': self.r(a + 0x204), 'admin_208': self.r(a + 0x208),
                          'stream_byte': self.r(a + 0x1f8 + 8), 'stream_bit': self.r(a + 0x1f8 + 0xc),
                          'sync_a_index': s32(self.r(SYNC_A + 0x14)) - 1, 'sync_a_hash': s32(self.r(SYNC_A + 8)),
                          'log_count': len(self.log)})
                self.captures.append(e)
            self.processes.append(e)
            self.ret(0); return
        super().hook(u, pc, size, user)

def stream(m, size=0x4000):
    obj = m.alloc(16); buf = m.packet([0] * size, refs=1)
    m.w(obj, WRITER_VTABLE); m.w(obj + 4, buf); m.w(obj + 8, 0); m.w(obj + 12, 3)
    return obj

def put(m, s, value, width):
    src = m.alloc(8); m.w(src, value & 0xffffffff)
    m.ncall(0x40d420, [s, src, width], 0, 0)
    m.w(m.stack, m.stop)  # cdecl: caller pops

def put_text(m, s, text):
    b = text.encode('utf-16-le'); n = len(b) // 2
    put(m, s, n, 16)
    if n:
        src = m.alloc(len(b) + 4); m.u.mem_write(src, b)
        m.ncall(0x40d420, [s, src, 16 * n], 0, 0)

def finalized(m, s):
    byte, bit = m.r(s + 8), m.r(s + 12); bits = byte * 8 + bit
    data = bytearray(m.u.mem_read(m.r(s + 4), (bits + 7) // 8))
    data[0] |= (8 - bits % 8) & 7
    return list(data), {'byte': byte, 'bit': bit}

def request_packet(m, joins_at, speeds):
    """[54 speed]* with a 306 PlayerJoin at position `joins_at`."""
    s = stream(m)
    for k, speed in enumerate(speeds):
        if k == joins_at:
            put(m, s, 306, 9)
            for t in ['Joiner', '', '1.0.0.2', 'AA2004']: put_text(m, s, t)
            put(m, s, 0x1234abcd, 32)
            put(m, s, 0xc0a80105, 32); put(m, s, 6112, 32)
            put(m, s, 7, 32); put_text(m, s, 'joinerkey'); put(m, s, 1, 32); put(m, s, 2, 32); put(m, s, 3, 32)
        put(m, s, 54, 9); put(m, s, speed, 32)
    return finalized(m, s)

def host_run(case):
    m = ResumeMachine()
    m.ncall(0x77d900, [case['host_sync_index'] & 0xffffffff, case['host_sync_hash'] & 0xffffffff], SYNC_A, 8)
    m.ncall(0x77d900, [0xffffffff, 0xffffffff], SYNC_S, 8)
    m.w(m.player + 0x20, HOST_PLAYER_ID)
    m.w(m.admin + 0x284, m.player); m.w(m.admin + 0x208, case['admin_208'])
    out = m.admin + 0x1f8; buf = m.packet([0] * 0x4000, refs=1)
    m.w(out, WRITER_VTABLE); m.w(out + 4, buf); m.w(out + 8, 0); m.w(out + 12, 3)
    m.out_stream = out
    data, end = request_packet(m, case['join_at'], case['speeds'])
    m.log = []
    p = m.packet(data, refs=2)
    m.ncall(0x74d710, [p, 0xffffffff, 0xffffffff, 0], m.admin, 16)
    decree, decree_end = finalized(m, out)
    return m, {'request_packet': data, 'request_end': end, 'decree_packet': decree, 'decree_end': decree_end,
               'log': m.log, 'processes': m.processes, 'captures': m.captures, 'sync_a': m.sync(SYNC_A)}

def client_run(host, case, resume):
    m = ResumeMachine()
    cap = host['captures'][0]
    m.ncall(0x77d900, [cap['sync_a_index'] & 0xffffffff, cap['sync_a_hash'] & 0xffffffff], SYNC_A, 8)
    m.ncall(0x77d900, [0xffffffff, 0xffffffff], SYNC_S, 8)
    window = m.admin + 0x214; cap_n = 4; arr = m.alloc(4 * cap_n + 4); m.w(arr, cap_n); arr += 4
    n = case['packet_index']
    for o, v in [(0, n), (4, n + 1), (8, cap_n), (12, arr)]: m.w(window + o, v & 0xffffffff)
    pkt = m.packet(host['decree_packet'], refs=1); m.w(arr + ((n + 0x80000000) % cap_n) * 4, pkt)
    m.w(m.admin + 0x218, n + 1); m.w(m.admin + 0x20c, n)
    if resume:
        m.w(m.admin + 0x248, cap['admin_200']); m.w(m.admin + 0x24c, cap['admin_204']); m.w(m.admin + 0x250, cap['admin_208'])
    else:
        m.w(m.admin + 0x248, 0xffffffff); m.w(m.admin + 0x24c, 0xffffffff); m.w(m.admin + 0x250, 0)
    m.log = []
    m.ncall(0x74fd10, [], m.admin, 0)
    return {'resume': resume, 'log': m.log, 'processes': m.processes, 'sync_a': m.sync(SYNC_A),
            'next': s32(m.r(m.admin + 0x20c))}

CASES = [
    {'name': 'join_in_middle', 'join_at': 1, 'speeds': [1, 2, 3], 'host_sync_index': -1, 'host_sync_hash': -1, 'admin_208': 0, 'packet_index': 0},
    {'name': 'join_first', 'join_at': 0, 'speeds': [4, 5], 'host_sync_index': 40, 'host_sync_hash': 1234, 'admin_208': 0, 'packet_index': 7},
    {'name': 'join_last', 'join_at': 2, 'speeds': [6, 7, 8], 'host_sync_index': 99, 'host_sync_hash': 777, 'admin_208': 0, 'packet_index': 12},
    {'name': 'long_packet', 'join_at': 3, 'speeds': [1, 0, 2, 3, 1, 2], 'host_sync_index': 5, 'host_sync_hash': 65535, 'admin_208': 0, 'packet_index': 3},
]

def main():
    rows = []; visited = {}
    for case in CASES:
        hm, host = host_run(case)
        visited.update(hm.visited)
        resumed = client_run(host, case, True)
        start = client_run(host, case, False)
        cap = host['captures'][0]
        # The host's A entries after the capture vs the client's (without the client's
        # final "m_client_first_unprocessed_decree", which pairs with the host seal 0x752EA0).
        host_after = [e for e in host['log'][cap['log_count']:] if e['sync'] == 'A']
        client_a = [e for e in resumed['log'] if e['sync'] == 'A']
        aligned = [(h['value'], h['hash_before']) for h in host_after] == [(c['value'], c['hash_before']) for c in client_a[:-1]] \
            and client_a[-1]['hash_before'] == host['sync_a']['hash']
        start_a = [e for e in start['log'] if e['sync'] == 'A']
        rows.append({'case': case, 'host': host, 'client_resumed': resumed, 'client_from_start': start,
                     'resume_pair_is_stream_position': (cap['admin_200'], cap['admin_204']) == (cap['stream_byte'], cap['stream_bit']),
                     'aligned_with_resume': aligned,
                     'aligned_from_start': start_a[-1]['hash_before'] == host['sync_a']['hash'] if start_a else False})
        print(case['name'], 'pair==stream', rows[-1]['resume_pair_is_stream_position'], 'aligned(resume)', aligned,
              'aligned(start)', rows[-1]['aligned_from_start'], 'cap', (cap['admin_200'], cap['admin_204']), flush=True)
    ranges = []
    for a, size in sorted(visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = [{'start': hex(a), 'end': hex(b), 'sha256': sha(PE0.range(a, b - a)),
                'v14_identical': PE0.range(a, b - a) == PE1.range(a, b - a)} for a, b in ranges]
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-join-resume.json').write_text(json.dumps(report) + '\n')
    print('v14 differs at', [w['start'] for w in windows if not w['v14_identical']])

if __name__ == '__main__':
    main()
