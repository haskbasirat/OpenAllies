"""Original Supply component (slot 11): constructor 58AA50 then saved Load
58ABD0 on the same object, under Unicorn.

Native: base 4DB020/50AB90, 529C30 ActorData access, 45D550(0) supply_range
getter (ActorData+420), GActor virtual +108 (4D1E80, owner word E8), thunk
4E38A0 -> 4E1F00 owner Kingdom supply-source list (+264 head insert, +268
tail), Load version reader 8589B0, Updatable 50BCD0, is_relay (+424) and
stockpile_max (+428) multiply. Seams (inherited from native_saved_components):
stream byte source and node allocation 6F7810.
"""
from pathlib import Path
import sys, json, struct, random
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase7-restoration'))
from native_saved_components import Machine as Base, P, V, VISITED, W, B
from unicorn.x86_const import *
import hashlib
H = lambda b: hashlib.sha256(b).hexdigest()


class NullAccess(Exception):
    pass


class Machine(Base):
    def __init__(self, payload, pc):
        super().__init__(payload, pc)
        sys.path.insert(0, str(HERE))
        import null_guard
        null_guard.install(self)

    def hook(self, u, pc, size, arg):
        if pc == 0x6f7810:
            self.ret(value=self.alloc(max(16, self.arg())))
            return
        if pc == 0x6f7430:  # pool free: allocations are never reused here
            self.ret()
            return
        if pc == 0x4849f0:  # run the original vector growth (base harness seams it)
            return
        if pc == 0x96cad6:  # CRT _wcsicmp, C-locale path (ASCII A-Z folding only)
            a, b = self.wtext(self.arg()), self.wtext(self.arg(1))
            fold = lambda s: [c + 32 if 0x41 <= c <= 0x5a else c for c in s] + [0]
            fa, fb = fold(a), fold(b)
            result = 0
            for x, y in zip(fa, fb):
                if x != y:
                    result = x - y
                    break
            self.ret(value=result & 0xffffffff)
            return
        super().hook(u, pc, size, arg)

    def wtext(self, p):
        out = []
        while True:
            c = int.from_bytes(self.u.mem_read(p, 2), 'little')
            if c == 0:
                return out
            out.append(c)
            p += 2

    def call(self, entry, this, args):
        self.write(self.stack, self.stop)
        for i, a in enumerate(args):
            self.write(self.stack + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ESP, self.stack)
        self.u.reg_write(UC_X86_REG_ECX, this)
        self.u.emu_start(entry, self.stop, count=200000)
        if self.null_accesses:
            raise NullAccess(self.null_accesses)
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))

    def case(self, c):
        owner = self.alloc(0x200)
        vt = self.alloc(0x200)
        self.write(vt + 0x108, 0x4d1e80)
        self.write(owner, vt)
        definition = self.alloc(0x800)
        self.write(owner + 4, definition)
        self.write(definition + 0x420, c['supply_range'])
        self.u.mem_write(definition + 0x424, bytes([int(c['is_relay'])]))
        self.write(definition + 0x428, c['stockpile_max'])
        kingdom = 0
        existing = []
        if c['owner'] is not None:
            kingdom = self.alloc(0x400)
            head = 0
            # existing list entries (most recent first), optionally containing this actor
            for entry in reversed(c['owner']):
                node = self.alloc(8)
                self.write(node, owner if entry == 'self' else 0x7000 + entry)
                self.write(node + 4, head)
                if head == 0:
                    self.write(kingdom + 0x268, node)
                head = node
            self.write(kingdom + 0x264, head)
        self.write(owner + 0xe8, kingdom)
        obj = self.object
        for off in range(0, 0x40, 4):
            self.write(obj + off, 0xcdcdcdcd)
        self.call(0x58aa50, obj, [owner])
        after_ctor = {hex(o): self.read(obj + o) for o in (0x10, 0x18, 0x1c)}
        after_ctor['0x14'] = self.u.mem_read(obj + 0x14, 1)[0]
        self.call(0x58abd0, obj, [self.world])
        state = {hex(o): self.read(obj + o) for o in (0x10, 0x18, 0x1c)}
        state['0x14'] = self.u.mem_read(obj + 0x14, 1)[0]
        supply_list = []
        if kingdom:
            node = self.read(kingdom + 0x264)
            while node:
                value = self.read(node)
                supply_list.append('self' if value == owner else value - 0x7000)
                node = self.read(node + 4)
            tail = self.read(kingdom + 0x268)
            tail_value = self.read(tail) if tail else None
            supply_list = {'head_first': supply_list,
                           'tail': None if tail_value is None else ('self' if tail_value == owner else tail_value - 0x7000)}
        return {'consumed': self.r.at, 'after_ctor': after_ctor, 'state': state, 'kingdom_list': supply_list if kingdom else None}


def cases():
    rng = random.Random(0x58aa50)
    rows = []
    for pc in ['Single24', 'Double53', 'Extended64']:
        for version in range(3):
            for relay in [False, True]:
                for owner in [None, [], [3], ['self'], [4, 'self', 5]]:
                    for multiplier, stock in [(B(0.5), B(50)), (B(1 / 3), B(77.7)), (0, B(10)), (B(-2), B(3)), (1, 1), (B(1e-30), B(1e-15))]:
                        payload = W(version) + (W(1, rng.choice([0, 7, 0xffffffff])) if version else b'') + \
                            (W(multiplier, rng.getrandbits(32)) if version >= 2 else b'')
                        rows.append({'precision': pc, 'payload': payload.hex(), 'version': version,
                                     'supply_range': B(rng.choice([0, 12.5, 40, 1e-3])), 'is_relay': relay,
                                     'stockpile_max': stock, 'owner': owner})
    return rows


def main():
    out = []
    for c in cases():
        cw = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}[c['precision']]
        m = Machine(bytes.fromhex(c['payload']) + b'FOLLOWING', cw)
        native = m.case(c)
        assert native['consumed'] == len(c['payload']) // 2, (native['consumed'], c)
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-supply.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
