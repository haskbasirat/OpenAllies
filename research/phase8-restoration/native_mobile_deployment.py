"""Original MobileDeployment component (slot 19): constructor 575940 then
saved Load 575890 on one object (seams as native_supply.py). Version 2's
temporary object (4CD440/4CD3F0/4CCF10) is outside these cases."""
from pathlib import Path
import sys, json, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED, W, B, H


class Machine(Base):
    def deployment(self):
        owner = self.alloc(0x200)
        obj = self.object
        for off in range(0, 0x40, 4):
            self.write(obj + off, 0xcdcdcdcd)
        self.call(0x575940, obj, [owner])
        ctor = [self.read(obj + 0x10), self.read(obj + 0x14)]
        self.call(0x575890, obj, [self.world])
        return {'consumed': self.r.at, 'after_ctor_10_14': ctor, 'words_10_14': [self.read(obj + 0x10), self.read(obj + 0x14)]}


def main():
    rng = random.Random(0x575940)
    out = []
    for version in (0, 1, 3):
        for _ in range(6):
            payload = W(1, rng.choice([0, 3, 0xffffffff])) + W(version) + W(rng.getrandbits(32)) + (W(rng.getrandbits(32)) if version else b'')
            m = Machine(payload + b'FOLLOWING', 0x27f)
            native = m.deployment()
            assert native['consumed'] == len(payload)
            out.append({'payload': payload.hex(), 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-mobile-deployment.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
