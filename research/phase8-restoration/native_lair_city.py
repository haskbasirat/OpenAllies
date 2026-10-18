"""Original Lair (slot 25: ctor 574FA0, Load 5751F0) and City (slot 29:
ctor 564160, Load 564CE0 with 4D1AF0 word list and the ActorData+5B4
city-zone check 45D6A0) on one object each, under Unicorn. Seams as
native_supply.py; the cancel path 5A0910 (KKC_MapResource::
ThrowCancelException) is recorded and stops."""
from pathlib import Path
import sys, json, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED, W, B, H


class Machine(Base):
    cancel = None

    def hook(self, u, pc, size, arg):
        if pc == 0x5a0910:
            self.cancel = 'map load canceled: city zone'
            u.emu_stop()
            return
        super().hook(u, pc, size, arg)

    def run_component(self, ctor, load, radius=None):
        owner = self.alloc(0x200)
        definition = self.alloc(0x800)
        self.write(owner + 4, definition)
        name = self.alloc(32)
        self.u.mem_write(name, 'city'.encode('utf-16-le') + b'\0\0')
        self.write(definition + 8, name)
        if radius is not None:
            self.write(definition + 0x5b4, radius)
        obj = self.object
        for off in range(0, 0x40, 4):
            self.write(obj + off, 0xcdcdcdcd)
        self.call(ctor, obj, [owner])
        ctor_words = [self.read(obj + o) for o in range(0x10, 0x24, 4)]
        try:
            self.call(load, obj, [self.world])
        except AssertionError:
            if self.cancel is None:
                raise
        values = []
        if ctor == 0x564160:  # City: 4D0890 append list {value, next} at +10/+14
            node = self.read(obj + 0x10)
            while node:
                values.append(self.read(node))
                node = self.read(node + 4)
            tail = self.read(obj + 0x14)
            tail = self.read(tail) if tail else None
        else:  # Lair: vector +10 ptr, +14 count, +18 capacity
            ptr, count = self.read(obj + 0x10), self.read(obj + 0x14)
            values = [self.read(ptr + 4 * i) for i in range(count)]
            tail = None
        return {'consumed': self.r.at, 'cancel': self.cancel, 'after_ctor_10_20': ctor_words,
                'list': values, 'tail': tail,
                'words_18_20': [self.read(obj + 0x18), self.read(obj + 0x1c), self.read(obj + 0x20)]}


def main():
    rng = random.Random(0x574fa0)
    out = []
    for n in [0, 1, 3, 0xffffffff]:
        words = [] if n == 0xffffffff else [rng.getrandbits(32) for _ in range(n)]
        payload = W(1, 0) + W(0) + W(n, *words) + W(rng.getrandbits(32), rng.getrandbits(32))
        m = Machine(payload + b'FOLLOWING', 0x27f)
        native = m.run_component(0x574fa0, 0x5751f0)
        assert native['consumed'] == len(payload)
        out.append({'component': 'Lair', 'payload': payload.hex(), 'native': native})
    for version in [0, 1]:
        for n in ([None] if version == 0 else [0, 2, 5]):
            for radius in [B(30), 0, B(-1), 0x80000000, 0x7fc00000]:
                words = [] if n is None else [rng.getrandbits(32) for _ in range(n)]
                payload = W(1, 0) + W(version) + (W(n, *words) if n is not None else b'')
                m = Machine(payload + b'FOLLOWING', 0x27f)
                native = m.run_component(0x564160, 0x564ce0, radius)
                assert native['consumed'] == len(payload)
                out.append({'component': 'City', 'radius': radius, 'payload': payload.hex(), 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-lair-city.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
