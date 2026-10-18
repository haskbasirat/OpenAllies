"""Original Defender component (slot 20): constructor 567CD0 (4DB020 base,
ActorData+21C/+220 local abilities through 4C8650) then saved Load 567020 on
the same object (Updatable 50BCD0, version 8589B0, Ability Load 4C8B40 per
record with the min(i, count-1) target, +24 entry list via 566FE0).
Seams as native_supply.py (stream source, 6F7810 allocation)."""
from pathlib import Path
import sys, json, random, struct
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED, W, B, H

SHARED_POWER = [B(18), B(-3), B(0.5), B(1e-30)]


class Machine(Base):
    def defender(self, count):
        owner = self.alloc(0x200)
        definition = self.alloc(0x800)
        self.write(owner + 4, definition)
        array = self.alloc(4 * max(1, count))
        self.locals = []
        self.shared = []
        for i in range(count):
            local = self.alloc(0x80)
            shared = self.alloc(0x100)
            self.write(local, shared)
            self.write(shared + 0x64, SHARED_POWER[i % len(SHARED_POWER)])
            self.write(array + 4 * i, local)
            self.locals.append(local)
            self.shared.append(shared)
        self.write(definition + 0x21c, array)
        self.write(definition + 0x220, count)
        obj = self.object
        for off in range(0, 0x40, 4):
            self.write(obj + off, 0xcdcdcdcd)
        self.call(0x567cd0, obj, [owner])
        ctor = self.snapshot(obj)
        self.call(0x567020, obj, [self.world])
        return {'consumed': self.r.at, 'after_ctor': ctor, 'state': self.snapshot(obj)}

    def ability(self, p):
        d, local = self.read(p + 4), self.read(p + 8)
        ids = []
        node = self.read(p + 0x24)
        guard = 0
        while node and guard < 64:
            ids.append(self.read(node))
            node = self.read(node + 4)
            guard += 1
        return {'definition': self.shared.index(d) if d in self.shared else d,
                'local': self.locals.index(local) if local in self.locals else local,
                'use_count': self.read(p + 0xc), 'last_use': self.read(p + 0x10), 'word14': self.read(p + 0x14),
                'enabled': self.u.mem_read(p + 0x18, 1)[0], 'recharge_bypass': self.u.mem_read(p + 0x19, 1)[0],
                'fuzzy_range': self.read(p + 0x1c), 'power': self.read(p + 0x20), 'summoned_head_first': ids,
                'word28': self.read(p + 0x28)}

    def snapshot(self, obj):
        abilities = [self.ability(self.read(self.read(obj + 0x10) + 4 * i)) for i in range(self.read(obj + 0x14))]
        entries = []
        node = self.read(obj + 0x24)
        while node:
            entries.append([self.read(node), self.read(node + 4)])
            node = self.read(node + 8)
        tail = self.read(obj + 0x28)
        return {'abilities': abilities, 'capacity': self.read(obj + 0x18), 'word_1c': self.read(obj + 0x1c),
                'word_20': self.read(obj + 0x20), 'entries_head_first': entries,
                'tail': None if tail == 0 else [self.read(tail), self.read(tail + 4)],
                'word_2c': self.read(obj + 0x2c), 'byte_30': self.u.mem_read(obj + 0x30, 1)[0]}


def ability_record(rng, version):
    out = W(version) + W(rng.choice([0, 1, 7]), rng.getrandbits(32), rng.getrandbits(32)) + bytes([rng.choice([0, 1, 255])])
    if version:
        ids = [rng.getrandbits(32) for _ in range(rng.randrange(3))]
        out += W(len(ids), *ids)
    return out


def cases():
    rng = random.Random(0x567cd0)
    rows = []
    for count in [1, 2, 4]:
        for version in range(3):
            for saved in ([0] if version == 0 else [0, 1, count, count + 2]):
                for pairs in ([None] if version < 2 else [[], [[5, 6]], [[1, 2], [3, 4], [7, 8]]]):
                    payload = W(1, rng.choice([0, 9])) + W(version)
                    if version:
                        payload += W(saved) + b''.join(ability_record(rng, rng.choice([0, 1])) for _ in range(saved))
                    if version >= 2:
                        payload += W(len(pairs), *[w for p in pairs for w in p])
                    rows.append({'ability_count': count, 'version': version, 'payload': payload.hex()})
    return rows


def main():
    out = []
    for c in cases():
        m = Machine(bytes.fromhex(c['payload']) + b'FOLLOWING', 0x27f)
        native = m.defender(c['ability_count'])
        assert native['consumed'] == len(c['payload']) // 2, (native['consumed'], c)
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'shared_main_power': SHARED_POWER,
           'cases': out}
    path = HERE / 'native-defender.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
