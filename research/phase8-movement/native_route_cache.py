"""Original SAI route cache, cached-route region search and region path cost.

Executes UNCHANGED original AA.exe instructions through harness.py (every
executed instruction byte-checked against v14, page-0 null traps, unseamed
allocator traps, nonvolatile-register checks) on native-layout memory:

  manager  [0xB3652C]  +0x14 region pointer array, +0x18 count, +0x3C epoch,
                       +0x40 route-cache object {count, rows}, rows[i] = {size, bytes}
  ready    [0xB36528]  +0x20 readiness byte
  busy     0xB3D43D    process-wide solver busy byte
  solver   built by the original constructor 0x53F290 (vtables A3680C/A367B0/A367F0,
           queue capacity and limit 0x2000) and reused across queries like the
           persistent SAI solver [[0xB36534]+0x234] used by 0x7A1B30.

Routines executed:
  0x7ABC30  manager region finalisation tail: destroys any old cache (inline
            0x7AAC90 body + free), allocates the 8-byte cache object (0x79E130),
            builds it with 0x53F0A0 (0x53ED50 rows, rows filled with 0xFF)
  0x5D8560  'ResetPathPlanner' console command: frees rows and rebuilds (0x53F0A0)
  0x7ACAC8..0x7ACAE5  cache destruction tail of manager clear 0x7AC9A0
            (0x7AAC90 + free + [mgr+0x40] = 0)
  0x53F320  region search entry (flag 1 = cached route, flag 0 for comparison):
            0x53EDE0 core, early test 0x53E610, mark 0x53F230/0x53F120,
            finish 0x53EBD0/0x53E920, hop 0x53EAC0, adjacency 0x7B90C0, list
            helpers 0x53E680/0x53E6D0/0x53E750/0x5C2C70, queue 0x53F350/0x53F410
  0x53E5B0  path cost over the output list (via a fresh stub that stores st(0))
  0x5C2CD0  list release, as 0x7A1B30 does after the cost

Seams (explicit): operator new/delete 0x6F7810/0x6F7430 (bump heap, poisoned
fresh and freed memory, double/foreign frees rejected); the per-region
finalisers 0x7BCA60/0x7B9900/0x7BC8B0 called by 0x7ABC30 before the cache
code (recorded, not under test; 0x7B9900 returns a sentinel which 0x7ABC30
stores at region+0x0C). Nothing else is intercepted.

Faulting rows record the native fault (address and pc); they are native
undefined behaviour (out-of-range hop reads or a null cache) and the Rust
port must return an explicit error there.
"""
import copy, hashlib, json, random, struct, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from harness import (Machine, write_document, f2b, NullDereference,  # noqa: E402
                     UnseamedAllocator, STACK, STOP, SCRATCH)
from unicorn import UcError, UC_HOOK_CODE  # noqa: E402
from unicorn.x86_const import (UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_ECX,  # noqa: E402
                               UC_X86_REG_ESI, UC_X86_REG_EBX, UC_X86_REG_EDI,
                               UC_X86_REG_EBP)

MANAGER_GLOBAL, READY_GLOBAL, BUSY = 0xb3652c, 0xb36528, 0xb3d43d
PRECISIONS = ['Single24', 'Double53', 'Extended64']
POISON_NEW, POISON_FREE = 0xcd, 0xdd
COST_OUT = SCRATCH + 0x100


def bits(x):
    return f2b(x)


class RouteMachine(Machine):
    def __init__(self, precision, scenario):
        super().__init__(precision)
        self.sc = scenario
        self.live = {}
        self.heap_log = []
        self.region_calls = []
        self.popped, self.discovered = [], []
        self.seams[0x6f7810] = self.op_new
        self.seams[0x6f7430] = self.op_delete
        for address, name in ((0x7bca60, '7BCA60'), (0x7b9900, '7B9900'), (0x7bc8b0, '7BC8B0')):
            self.seams[address] = (lambda n: (lambda m: m.region_seam(n)))(name)
        self.u.hook_add(UC_HOOK_CODE, self.observe, begin=0x53eef9, end=0x53eef9)
        self.u.hook_add(UC_HOOK_CODE, self.observe, begin=0x53e840, end=0x53e840)
        self.build_world()

    # ---- seams -----------------------------------------------------------
    def op_new(self, m):
        size = m.arg(0)
        p = m.alloc(max(size, 1))
        m.u.mem_write(p, bytes([POISON_NEW]) * max(size, 1))
        self.live[p] = size
        self.heap_log.append(['new', size])
        m.ret(0, p)

    def op_delete(self, m):
        p = m.arg(0)
        if p == 0:
            self.heap_log.append(['delete_null'])
        else:
            assert p in self.live, 'foreign or double free ' + hex(p)
            size = self.live.pop(p)
            m.u.mem_write(p, bytes([POISON_FREE]) * max(size, 1))
            self.heap_log.append(['delete', size])
        m.ret(0)

    def region_seam(self, name):
        region = self.ecx()
        rid = self.r32(region)
        self.region_calls.append('%s:%d' % (name, rid))
        self.ret(0, 0x5e5e0000 + rid if name == '7B9900' else None)

    def observe(self, u, pc, size, _):
        if pc == 0x53eef9:
            self.popped.append(self.r32(self.solver + 0x1c, 4))
        else:
            self.discovered.append('%d>%d' % (self.r32(self.solver + 0x18), self.r32(self.solver + 0x2c)))

    # ---- world -----------------------------------------------------------
    def build_world(self):
        sc = self.sc
        n = len(sc['nodes'])
        self.manager = self.alloc(0x100)
        self.ready = self.alloc(0x40)
        self.w32(MANAGER_GLOBAL, self.manager)
        self.w32(READY_GLOBAL, self.ready)
        self.w8(self.ready + 0x20, 1 if sc['ready'] else 0)
        self.w8(BUSY, 0)
        self.array = self.alloc(4 * max(n, 1))
        self.w32(self.manager + 0x14, self.array, n)
        self.w8(self.manager + 0x3c, sc['epoch'])
        self.w32(self.manager + 0x40, 0)
        self.node_ptr = []
        for i, node in enumerate(sc['nodes']):
            ptr = self.alloc(0x1000, 0x1000)
            self.node_ptr.append(ptr)
            self.w32(self.array + 4 * i, ptr)
        for i, node in enumerate(sc['nodes']):
            # Same original layout as research/phase6-region/graph_oracle.py.
            ptr = self.node_ptr[i]
            self.w32(ptr, i, ptr + 0x100)
            self.w32(ptr + 0x100, ptr + 0x110)
            self.w32(ptr + 0x118, node['category'])
            self.w32(ptr + 0x54, *node['position_bits'])
            self.w32(ptr + 0x5c, ptr + 0x200, 1, 0, 0)
            self.u.mem_write(ptr + 0x6d, bytes([node['visit_epoch'], node['parent_neighbor']]))
            self.w8(ptr + 0x70, len(node['neighbors']))
            self.w32(ptr + 0x74, ptr + 0x300)
            for j, nb in enumerate(node['neighbors']):
                self.w32(ptr + 0x300 + 4 * j, self.node_ptr[nb])
            previous = 0
            for j, (key, value) in enumerate(node['distance_cache'].items()):
                entry = ptr + 0x800 + 16 * j
                self.w32(entry, int(key), value, previous)
                previous = entry
            self.w32(ptr + 0x200, previous)
        # Persistent solver from the original constructor.
        self.solver = self.alloc(0x80)
        self.call(0x53f290, ecx=self.solver)
        assert self.r32(self.solver) == 0xa3680c and self.r32(self.solver + 0x34) == 0xa367b0
        assert self.r32(self.solver + 0x48) == 0xa367f0 and self.r32(self.solver + 0x58) == 0x2000
        self.cost_stub = self.stub_next
        self.stub_next += 32
        rel = 0x53e5b0 - (self.cost_stub + 5)
        code = b'\xe8' + struct.pack('<i', rel) + b'\xdb\x3d' + struct.pack('<I', COST_OUT) + b'\xc3'
        self.u.mem_write(self.cost_stub, code)

    # ---- observation -----------------------------------------------------
    def cache_state(self):
        cache = self.r32(self.manager + 0x40)
        if cache == 0:
            return None
        count = self.r32(cache)
        rows = self.r32(cache + 4)
        shape, data = [], b''
        for i in range(count):
            size, ptr = self.r32(rows + 8 * i, 2)
            shape.append(size)
            data += bytes(self.u.mem_read(ptr, size))
        return {'count': count, 'row_sizes': ' '.join(map(str, shape)), 'rle': rle(data)}

    def node_states(self):
        """Visit epoch byte +0x6D and predecessor byte +0x6E of every region, hex."""
        return b''.join(bytes(self.u.mem_read(p + 0x6d, 2)) for p in self.node_ptr).hex()

    def read_list(self, lst):
        ids, ptr, previous = [], self.r32(lst), 0
        while ptr:
            assert self.r32(ptr + 4) == previous, 'broken back link'
            ids.append(self.r32(ptr))
            assert len(ids) <= 100000
            previous, ptr = ptr, self.r32(ptr + 8)
        assert previous == self.r32(lst + 4), 'tail mismatch'
        return ids

    def run_range(self, begin, until, regs):
        self.u.reg_write(UC_X86_REG_ESP, STACK)
        for r, v in regs.items():
            self.u.reg_write(r, v)
        self.u.emu_start(begin, until, count=2_000_000)
        assert self.u.reg_read(UC_X86_REG_EIP) == until

    def fault(self, e):
        return {'kind': type(e).__name__, 'detail': str(e), 'pc': hex(self.u.reg_read(UC_X86_REG_EIP))}

    # ---- operations ------------------------------------------------------
    def op(self, step):
        kind = step['op']
        if kind == 'create':
            heap0, calls0 = len(self.heap_log), len(self.region_calls)
            self.call(0x7abc30, ecx=self.manager)
            assert self.fpu_top() == 0
            slots = [self.r32(p + 0xc) for p in self.node_ptr]
            assert slots == [0x5e5e0000 + i for i in range(len(self.node_ptr))]
            return {'cache': self.cache_state(), 'heap': self.heap_log[heap0:],
                    'region_calls': ' '.join(self.region_calls[calls0:]),
                    'manager_byte5': self.r8(self.manager + 5)}
        if kind == 'reset':
            heap0 = len(self.heap_log)
            self.call(0x5d8560, ecx=0, args=(0,))
            return {'cache': self.cache_state(), 'heap': self.heap_log[heap0:]}
        if kind == 'destroy':
            heap0 = len(self.heap_log)
            self.w32(STACK, 0)
            self.run_range(0x7acac8, 0x7acae5, {UC_X86_REG_ESI: self.manager, UC_X86_REG_EBX: 0})
            return {'cache': self.cache_state(), 'heap': self.heap_log[heap0:]}
        if kind == 'set':
            if 'busy' in step:
                self.w8(BUSY, step['busy'])
            if 'ready' in step:
                self.w8(self.ready + 0x20, step['ready'])
            if 'queue_limit' in step:
                self.w32(self.solver + 0x58, step['queue_limit'])
            return {}
        assert kind == 'search', kind
        self.popped, self.discovered = [], []
        heap0 = len(self.heap_log)
        lst = self.alloc(8)
        try:
            status = self.call(0x53f320, ecx=self.solver,
                               args=(lst, step['start'], step['goal'], step['flag']), count=60_000_000)
        except (NullDereference, UnseamedAllocator, UcError, AssertionError) as e:
            return {'fault': self.fault(e), 'popped': words(self.popped), 'discovered': ' '.join(self.discovered),
                    'heap': heap_counts(self.heap_log[heap0:])}
        assert self.fpu_top() == 0 and self.final_esp == STACK + 4 + 16
        route = self.read_list(lst)
        out = {'status': status, 'route': route, 'popped': words(self.popped),
               'discovered': ' '.join(self.discovered), 'node_states': self.node_states(),
               'epoch': self.r8(self.manager + 0x3c), 'busy': bool(self.r8(BUSY)),
               'output_linked': self.r32(self.solver + 0x44) != 0,
               'queue_remaining': self.r32(self.solver + 0x50),
               'cache': self.cache_state()}
        # 0x53E5B0 on the output list, then the 0x5C2CD0 release done by 0x7A1B30.
        self.u.mem_write(COST_OUT, b'\0' * 10)
        try:
            self.call(self.cost_stub, ecx=lst)
            assert self.fpu_top() == 0
            raw = bytes(self.u.mem_read(COST_OUT, 10))
            ext = int.from_bytes(raw, 'little')
            out['cost_x87'] = '%020x' % ext
            out['cost_bits'] = ext80_to_f32_bits(ext)
        except (NullDereference, UcError, AssertionError) as e:
            out['cost_fault'] = self.fault(e)
        live_before = len(self.live)
        self.call(0x5c2cd0, ecx=lst)
        assert self.r32(lst) == 0 and self.r32(lst + 4) == 0
        assert live_before - len(self.live) == len(route), 'list release mismatch'
        out['heap'] = heap_counts(self.heap_log[heap0:])
        return out


def rle(data):
    """Row bytes concatenated in row order as 'hh*run' tokens (one JSON line)."""
    runs = []
    for byte in data:
        if runs and runs[-1][0] == byte:
            runs[-1][1] += 1
        else:
            runs.append([byte, 1])
    return ' '.join('%02x*%d' % (b, n) for b, n in runs)


def words(rows):
    """Queue entries {node, depth, priority bits, cost bits} as hex words."""
    return ['%08x %08x %08x %08x' % tuple(r) for r in rows]


def heap_counts(log):
    return {'new': sum(1 for e in log if e[0] == 'new'),
            'delete': sum(1 for e in log if e[0] == 'delete'),
            'delete_null': sum(1 for e in log if e[0] == 'delete_null')}


def ext80_to_f32_bits(ext):
    """The 53E5B0 result is loaded from a binary32 (fld dword), so the 80-bit
    value converts exactly; assert that and return the binary32 pattern."""
    sign = ext >> 79
    exp = (ext >> 64) & 0x7fff
    man = ext & ((1 << 64) - 1)
    if exp == 0 and man == 0:
        return sign << 31
    if exp == 0x7fff:
        frac = (man >> 40) & 0x7fffff
        assert man & ((1 << 40) - 1) == 0
        return (sign << 31) | (0xff << 23) | frac
    assert man >> 63 == 1, 'unnormal'
    e = exp - 16383
    assert man & ((1 << 40) - 1) == 0, 'not a binary32 value'
    if e >= -126:
        assert e <= 127
        return (sign << 31) | ((e + 127) << 23) | ((man >> 40) & 0x7fffff)
    shift = 40 + (-126 - e)
    assert man & ((1 << shift) - 1) == 0, 'not a binary32 subnormal'
    return (sign << 31) | (man >> shift)


# ---- scenarios ---------------------------------------------------------------
def node(pos, category=1):
    return {'position_bits': [bits(pos[0]), bits(pos[1])], 'category': category, 'neighbors': [],
            'distance_cache': {}, 'visit_epoch': 0, 'parent_neighbor': 255}


def graph(positions, edges, categories=None):
    ns = [node(p, (categories or {}).get(i, 1)) for i, p in enumerate(positions)]
    for a, b in edges:
        ns[a]['neighbors'].append(b)
        ns[b]['neighbors'].append(a)
    return ns


def S(start, goal, flag=1):
    return {'op': 'search', 'start': start, 'goal': goal, 'flag': flag}


CREATE, RESET, DESTROY = {'op': 'create'}, {'op': 'reset'}, {'op': 'destroy'}


def scenario(name, nodes, ops, epoch=0, ready=True):
    return {'name': name, 'epoch': epoch, 'ready': ready, 'nodes': nodes, 'ops': ops}


def scenarios():
    rows = []
    line = graph([(0, 0), (1.25, 0.5), (2.75, 0.25), (4.0, 1.0), (5.5, 0.75)],
                 [(0, 1), (1, 2), (2, 3), (3, 4)])
    rows.append(scenario('line_hits', line, [
        CREATE, S(0, 4), S(0, 4), S(4, 0), S(1, 3), S(3, 1, 0), S(2, 2), S(4, 1), S(1, 4, 0),
        RESET, S(1, 3), S(0, 4), DESTROY]))
    # 3x3 grid with near-equal routes: later cached answers come from earlier trees.
    pos = [(x * 1.0 + 0.1 * y, y * 1.0 + 0.05 * x) for y in range(3) for x in range(3)]
    edges = [(y * 3 + x, y * 3 + x + 1) for y in range(3) for x in range(2)]
    edges += [(y * 3 + x, (y + 1) * 3 + x) for y in range(2) for x in range(3)]
    grid = graph(pos, edges)
    rows.append(scenario('grid_stale_trees', grid, [
        CREATE, S(0, 8), S(2, 6), S(6, 2), S(0, 6), S(8, 0), S(2, 6, 0), S(1, 7), S(7, 1), S(5, 3),
        S(3, 5, 0), S(4, 4), S(0, 2), S(2, 0)]))
    # Two categories: mismatched pairs never consult the cache.
    two = graph([(0, 0), (1, 0), (2, 0), (5, 5), (6, 5)], [(0, 1), (1, 2), (3, 4)],
                {3: 2, 4: 2})
    rows.append(scenario('categories', two, [
        CREATE, S(0, 4), S(4, 0), S(3, 4), S(4, 3), S(0, 2), S(2, 0), S(0, 3, 0), S(2, 4)]))
    # Asymmetric adjacency: region 1 does not list region 2, so 0x7B90C0 yields
    # 0xFE for the pair (1,2): the goal is found yet 0x53E920 returns no route.
    asym = graph([(0, 0), (1, 0.5), (2.5, 0)], [])
    asym[0]['neighbors'] = [1]
    asym[1]['neighbors'] = [0]
    asym[2]['neighbors'] = [1]
    rows.append(scenario('asymmetric_no_route', asym, [
        CREATE, S(2, 1), S(2, 1), S(1, 2), S(0, 1), S(1, 0), S(2, 1, 0)]))
    # Natural epoch wrap 254 -> 255 -> 0 with stale epoch-0 node bytes whose
    # parent bytes would look visited unless the wrap resets them to 0xFF.
    stale = copy.deepcopy(grid)
    for nd in stale:
        nd['visit_epoch'], nd['parent_neighbor'] = 0, 0
    rows.append(scenario('epoch_wrap', stale, [
        CREATE, S(0, 8), S(6, 2), S(3, 5), S(0, 8), S(8, 0, 0), S(1, 7), S(4, 4)], epoch=254))
    # Busy, not-ready, start==goal and mismatched categories with and without a cache.
    rows.append(scenario('guards_null_cache', copy.deepcopy(two), [
        S(1, 1), S(0, 4), {'op': 'set', 'busy': 1}, S(0, 2), {'op': 'set', 'ready': 0}, S(0, 2),
        {'op': 'set', 'ready': 1}, S(2, 2, 0), S(0, 2, 0), S(0, 2)]))
    rows.append(scenario('guards_with_cache', copy.deepcopy(two), [
        CREATE, {'op': 'set', 'busy': 1}, S(0, 2), {'op': 'set', 'ready': 0}, S(0, 2),
        {'op': 'set', 'ready': 1}, S(0, 2), {'op': 'set', 'busy': 1}, S(0, 2), S(2, 0)]))
    # Lifecycle: recreate destroys the old cache; destroy leaves no cache.
    rows.append(scenario('lifecycle', copy.deepcopy(line), [
        CREATE, S(0, 4), CREATE, S(4, 0), S(4, 0), RESET, RESET, S(1, 3), DESTROY, S(2, 2), S(0, 1)]))
    rows.append(scenario('single_region', graph([(3, 4)], []), [CREATE, S(0, 0), RESET, S(0, 0, 0)]))
    rows.append(scenario('two_regions', graph([(0, 0), (3, 4)], [(0, 1)]), [
        CREATE, S(1, 0), S(0, 1), S(1, 1), RESET, S(0, 1)]))
    # Bounded queue: a partial result finishes from an unknown (0xFF) cache pair.
    bounded = graph([(0, 0), (1, 1), (1, -1), (2, 0)], [(0, 1), (0, 2), (1, 3), (2, 3)])
    rows.append(scenario('bounded_queue_unknown_hop', bounded, [
        CREATE, {'op': 'set', 'queue_limit': 1}, S(0, 3)]))
    # Depth limit: the failed search still fills the cache; later sub-pairs hit.
    deep = graph([(i * 2.0, 0.0) for i in range(260)], [(i, i + 1) for i in range(259)])
    rows.append(scenario('depth255_fills_cache', deep, [
        CREATE, S(0, 259), S(0, 200), S(250, 3), S(259, 0, 0)]))
    # Cached distances (the per-node 0x7BB420 hash) shape both searches and costs.
    cached = copy.deepcopy(bounded)
    cached[0]['distance_cache'] = {'3': bits(9.75), '1': bits(4.5), '2': bits(0.125)}
    cached[1]['distance_cache'] = {'3': bits(1e-3)}
    cached[2]['distance_cache'] = {'3': bits(-2.5)}
    rows.append(scenario('cached_distances', cached, [
        CREATE, S(0, 3), S(3, 0), S(0, 3, 0), S(1, 2), S(2, 1)]))
    # Non-finite / overflowing path-cost inputs on the final hop only (the
    # search never evaluates parent(goal)->goal; 0x53E5B0 does).
    for label, value in (('snan', 0x7fa00000), ('inf', 0x7f800000), ('overflow', bits(3.0e38))):
        g = graph([(0, 0), (1, 0), (2, 0)], [(0, 1), (1, 2)])
        g[1]['distance_cache'] = {'2': value}
        if label == 'overflow':
            g[0]['distance_cache'] = {'1': bits(3.0e38)}
        sc = scenario('cost_' + label, g, [CREATE, S(0, 2), S(0, 2, 0)])
        sc['snan_input'] = label == 'snan'
        rows.append(sc)
    rng = random.Random(0x7a1b30)
    for trial in range(8):
        n = 12
        positions = [(rng.uniform(-20, 20), rng.uniform(-20, 20)) for _ in range(n)]
        edges = [(i, i + 1) for i in range(n - 1)]
        for i in range(n):
            for j in range(i + 2, n):
                if rng.randrange(5) == 0:
                    edges.append((i, j))
        g = graph(positions, edges)
        if trial % 2:
            for nd in g:
                nd['neighbors'].reverse()
        ops = [CREATE]
        for q in range(14):
            a, b = rng.randrange(n), rng.randrange(n)
            ops.append(S(a, b, 1))
            if q % 3 == 0:
                ops.append(S(a, b, 0))
            if q == 9:
                ops.append(RESET)
        rows.append(scenario('generated_%d' % trial, g, ops, epoch=rng.randrange(256)))
    # Larger shuffled-id graphs: many cached sub-pairs and cross-tree walks
    # (0x53EAC0 overshoot and landing branches inside 0x53F120).
    for trial in range(4):
        n = 30
        ids = list(range(n))
        rng.shuffle(ids)
        positions = [(rng.uniform(-50, 50), rng.uniform(-50, 50)) for _ in range(n)]
        edges = [(ids[i], ids[i + 1]) for i in range(n - 1)]
        for _ in range(n // 2):
            a, b = rng.randrange(n), rng.randrange(n)
            if a != b and (a, b) not in edges and (b, a) not in edges:
                edges.append((a, b))
        g = graph(positions, edges)
        ops = [CREATE]
        for q in range(25):
            a, b = rng.randrange(n), rng.randrange(n)
            ops.append(S(a, b, 1))
            if q % 4 == 0:
                ops.append(S(b, a, 0))
        rows.append(scenario('generated_large_%d' % trial, g, ops, epoch=rng.randrange(256)))
    return rows


def main():
    cases = []
    for sc in scenarios():
        for p in PRECISIONS:
            m = RouteMachine(p, sc)
            results = []
            for step in sc['ops']:
                r = m.op(step)
                results.append(r)
                if 'fault' in r:
                    break
            row = dict(copy.deepcopy(sc))
            row['name'] = sc['name'] + '_' + p
            row['precision'] = p
            row['results'] = results
            row.setdefault('snan_input', False)
            cases.append(row)
    faults = sorted({r['fault']['kind'] + '@' + r['fault']['pc'] for c in cases for r in c['results']
                     if 'fault' in r})
    searches = sum(1 for c in cases for r in c['results'] if 'status' in r or 'fault' in r)
    extra = {'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'seams': ['0x6F7810 operator new (bump heap, poisoned 0xCD)',
                       '0x6F7430 operator delete (poisoned 0xDD, double/foreign free rejected)',
                       '0x7BCA60/0x7B9900/0x7BC8B0 per-region finalisers inside 0x7ABC30 (recorded)'],
             'search_count': searches, 'native_faults': faults}
    write_document('native-route-cache.json', 'SAI route cache 0x7ABC30/0x53F0A0/0x5D8560/0x7AC9A0, '
                   'cached-route region search 0x53F320 flag 1 (and flag 0), path cost 0x53E5B0',
                   cases, extra)


if __name__ == '__main__':
    main()
