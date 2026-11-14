"""Native oracle for the actor engagement lists: 4D3990 (attacker onto
+118), 4D3A20 / 4D0D70 (+120 insert / 4F1430 removal) and the collectors
4D4EF0 (+98 entries), 4D4E70 (organization members, tail inserts) and
4D4F50 (+88 node lists through the [B364C0] generation check), with the
real 4D0900 and 4F1430 executing.

Random op sequences run over random worlds; after every op the +118, +120
and a caller list are walked head to tail (and the tail word recorded).
Recorded seams: 79EA90 / 7B1ED0 (SAI notice), 57A3C0 (organization notice);
6F7810 / 6F7430 are a bump allocator and a no-op free.

Usage: native_engagement_lists.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

APP = 0xB36528
OPS = ('attacker', 'add120', 'remove120', 'collect98', 'collect_members', 'collect88')


def pick(rng, n, p_null=0.15):
    return None if rng.random() < p_null else rng.randrange(n)


def make_world(rng):
    n = rng.randint(4, 9)
    gens = [rng.getrandbits(16) for _ in range(n)]
    things = []
    for i in range(n):
        t = {'gtid': (gens[i] << 16) | (i + 1), 'kingdom_object': rng.random() < 0.8,
             'org': None, 'c98': None, 'c88': None}
        if rng.random() < 0.5:
            slots = [pick(rng, n, 0.25) for _ in range(rng.randint(0, 4))]
            t['org'] = {'slots': slots, 'count': rng.choice((len(slots), len(slots), max(0, len(slots) - 1), -1))}
        if rng.random() < 0.5:
            entries = [pick(rng, n, 0.25) for _ in range(rng.randint(0, 4))]
            t['c98'] = {'entries': entries, 'count': rng.choice((len(entries), len(entries), 0, -2))}
        if rng.random() < 0.6:
            nodes = []
            for _ in range(rng.randint(0, 3)):
                if rng.random() < 0.2:
                    nodes.append(None)
                    continue
                gtids = []
                for _ in range(rng.randint(0, 3)):
                    j = rng.randrange(n)
                    roll = rng.random()
                    if roll < 0.7:
                        gtids.append((gens[j] << 16) | (j + 1))
                    elif roll < 0.85:
                        gtids.append((((gens[j] + 1) & 0xFFFF) << 16) | (j + 1))
                    else:
                        gtids.append((rng.getrandbits(16) << 16) | (n + 1 + rng.randrange(3)))
                nodes.append(gtids)
            t['c88'] = nodes
        things.append(t)
    registry = {}
    for i in range(n):
        registry[str(i + 1)] = {'generation': gens[i], 'object': i if rng.random() < 0.85 else None}
    for k in range(n + 1, n + 4):
        registry[str(k)] = {'generation': rng.getrandbits(16), 'object': rng.randrange(n) if rng.random() < 0.5 else None}
    return {'things': things, 'registry': registry,
            'notices': rng.random() < 0.6, 'notice_target': rng.random() < 0.7}


class Built:
    def __init__(self, m, world):
        self.m = m
        self.world = world
        self.events = []
        things = world['things']
        self.ptr = [m.alloc(0x200) for _ in things]
        self.index = {p: i for i, p in enumerate(self.ptr)}
        self.kobj = {}
        self.orgs = {}
        for i, t in enumerate(things):
            p = self.ptr[i]
            m.w(p + 0x14, t['gtid'])
            if t['kingdom_object']:
                k = m.alloc(0x10)
                self.kobj[k] = i
                m.w(p + 0xE8, k)
            if t['org'] is not None:
                comp = m.alloc(0x40)
                arr = m.alloc(4 * max(1, len(t['org']['slots'])))
                for k, s in enumerate(t['org']['slots']):
                    m.w(arr + 4 * k, self.ptr[s] if s is not None else 0)
                m.w(comp + 0x28, arr)
                m.w(comp + 0x2C, t['org']['count'] & 0xFFFFFFFF)
                m.w(p + 0x7C, comp)
                self.orgs[comp] = i
            if t['c98'] is not None:
                comp = m.alloc(0x40)
                arr = m.alloc(4 * max(1, len(t['c98']['entries'])))
                for k, s in enumerate(t['c98']['entries']):
                    m.w(arr + 4 * k, self.ptr[s] if s is not None else 0)
                m.w(comp + 0x24, arr)
                m.w(comp + 0x28, t['c98']['count'] & 0xFFFFFFFF)
                m.w(p + 0x98, comp)
            if t['c88'] is not None:
                comp = m.alloc(0x60)
                m.w(comp + 0x40, self.chain([self.holder(g) if g is not None else 0 for g in t['c88']]))
                m.w(p + 0x88, comp)
        table = m.alloc(0x20004 + 0x10000 * 4)
        m.w(h.REGISTRY_ROOT, table)
        for key, r in world['registry'].items():
            idx = int(key)
            m.u.mem_write(table + 4 + idx * 2, (r['generation']).to_bytes(2, 'little'))
            m.w(table + 0x20004 + idx * 4, self.ptr[r['object']] if r['object'] is not None else 0)
        app = 0
        if world['notices'] is not None:
            app = m.alloc(0x40)
            m.wb(app + 0x20, 1 if world['notices'] else 0)
        m.w(APP, app)
        self.notice = m.alloc(0x10) if world['notice_target'] else 0
        m.hooks[0x79EA90] = self.notice_target
        m.hooks[0x7B1ED0] = self.notice_attacked
        m.hooks[0x57A3C0] = self.organization_attacked
        m.hooks[0x6F7810] = lambda mm: (self.assert_eight(mm), mm.ret(0, mm.alloc(8, 8)))
        m.hooks[0x6F7430] = lambda mm: mm.ret(0)

    @staticmethod
    def assert_eight(mm):
        assert mm.arg(0) == 8

    def chain(self, values):
        head = 0
        for v in reversed(values):
            node = self.m.alloc(8, 8)
            self.m.w(node, v)
            self.m.w(node + 4, head)
            head = node
        return head

    def holder(self, gtids):
        obj = self.m.alloc(0x20)
        self.m.w(obj + 0xC, self.chain(gtids))
        return obj

    def notice_target(self, m):
        k = m.arg(0)
        self.events.append(['notice_target', self.kobj[k] if k else None])
        m.ret(4, self.notice)

    def notice_attacked(self, m):
        assert m.reg(UC_X86_REG_ECX) == self.notice
        self.events.append(['notice_attacked', self.index[m.arg(0)]])
        m.ret(4)

    def organization_attacked(self, m):
        org = self.orgs[m.reg(UC_X86_REG_ECX)]
        a = m.arg(0)
        self.events.append(['organization_attacked', org, self.index[a] if a else None])
        m.ret(4)

    def walk(self, head_address):
        head, tail = self.m.r(head_address), self.m.r(head_address + 4)
        values, node, last = [], head, 0
        while node:
            values.append(self.m.r(node))
            last = node
            node = self.m.r(node + 4)
        assert len(values) < 1000
        return {'values': values, 'tail_is_last': tail == last}


def main():
    rng = random.Random(0x4D3990)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)  # fs:[0] (FS base 0) for the SEH prolog of hooked callees
    worlds, cases = [], []
    for w in range(70):
        world = make_world(rng)
        if rng.random() < 0.15:
            world['notices'] = None
        worlds.append(world)
        m.u.mem_write(m.HEAP, bytes(m.cursor - m.HEAP))
        m.cursor = m.HEAP
        m.next_virtual = m.VIRTUALS
        m.hooks = {}
        built = Built(m, world)
        n = len(world['things'])
        actor = rng.randrange(n)
        a = built.ptr[actor]
        caller = m.alloc(8)
        for step in range(14):
            op = rng.choice(OPS)
            thing = pick(rng, n)
            built.events = []
            result = flag = None
            if op == 'attacker':
                m.call(0x4D3990, ecx=a, args=(built.ptr[thing] if thing is not None else 0,), pop=4)
            elif op == 'add120':
                m.call(0x4D3A20, ecx=a, args=(built.ptr[thing] if thing is not None else 0,), pop=4)
            elif op == 'remove120':
                result = m.call(0x4D0D70, ecx=a, args=(built.ptr[thing] if thing is not None else 0,), pop=4) & 0xFF
            else:
                if thing is None:
                    thing = rng.randrange(n)
                if op == 'collect98':
                    m.call(0x4D4EF0, args=(caller, built.ptr[thing]), pop=0)
                elif op == 'collect_members':
                    m.call(0x4D4E70, args=(caller, built.ptr[thing]), pop=0)
                else:
                    flag = rng.choice((0, 1, 0x101, 0x100))
                    m.call(0x4D4F50, args=(caller, built.ptr[thing], flag), pop=0)
            cases.append({'world': w, 'actor': actor, 'step': step, 'op': op, 'thing': thing, 'result': result, 'flag': flag,
                          'events': built.events, 'list118': built.walk(a + 0x118),
                          'list120': built.walk(a + 0x120), 'caller': built.walk(caller)})
    h.report('native-engagement-lists-fixtures.json',
             '4D3990, 4D3A20, 4D0D70 (4F1430), 4D4EF0, 4D4E70, 4D4F50 with the real 4D0900 node insert.',
             cases, m.visited, extra={'worlds': worlds})


if __name__ == '__main__':
    main()
