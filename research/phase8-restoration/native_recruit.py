"""Original Recruit component (slot 16): constructor 581E40 then saved Load
583C80 on the same object, under Unicorn (seams as native_supply.py). Legacy
version 1/2 queue items are exercised only with their object flag clear."""
from pathlib import Path
import sys, json, struct, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED, W, B, H


class Machine(Base):
    def recruit(self):
        owner = self.alloc(0x200)
        obj = self.object
        for off in range(0, 0x40, 4):
            self.write(obj + off, 0xcdcdcdcd)
        self.call(0x581e40, obj, [owner])
        ctor = list(self.u.mem_read(obj + 0x10, 0x10))
        self.call(0x583c80, obj, [self.world])
        state = list(self.u.mem_read(obj + 0x10, 0x10))
        return {'consumed': self.r.at, 'after_ctor_bytes_10_1f': ctor, 'bytes_10_1f': state}


def payloads():
    rng = random.Random(0x583c80)
    rows = []
    def upd():
        return W(1, rng.choice([0, 5, 0xffffffff]))
    def item(text):
        return struct.pack('<H', len(text)) + text.encode('utf-16-le') + b'\0'
    for version in range(4):
        for flag in ([None] if version == 0 else [0, 1, 7]):
            body = upd() + W(version)
            if version:
                body += bytes([flag])
                if flag:
                    body += W(rng.getrandbits(32), rng.getrandbits(32))
            if version in (1, 2):
                for items in [[], ['a'], ['regiment_x', ''], None]:
                    b2 = body
                    if items is None:
                        b2 += W(0xffffffff)  # negative count: no iterations
                    else:
                        b2 += W(len(items)) + b''.join(item(t) for t in items)
                    b2 += W(rng.getrandbits(32))
                    if version > 1:
                        b2 += bytes([rng.getrandbits(8)])  # 583EEF reads 1 byte
                    rows.append(b2)
            else:
                rows.append(body)
    return rows


def main():
    out = []
    for payload in payloads():
        m = Machine(payload + b'FOLLOWING', 0x27f)
        native = m.recruit()
        assert native['consumed'] == len(payload), (native, payload.hex())
        out.append({'payload': payload.hex(), 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-recruit.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
