"""Phase8 trigger census across every original RMP map.

Reuses the phase7 research decoder layout (native 54D3D0/541410/5468A0/54A640),
extended to the native manager version 4 (trailing string + word, discarded)
and element version 2 (same layout as 3) read by the developer maps.
Output: census.json (per-map element counts and decode status).
"""
from pathlib import Path
import collections, hashlib, json, struct, sys, zlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research' / 'phase7-triggers'))
import decode_census as dc  # noqa: E402

DATA = ROOT.parent / 'reconstruction/assets/original/Data/Maps'


class Reader(dc.Reader):
    def element(self):
        begin = self.at
        version = self.u32()
        assert version in (2, 3), (begin, version)
        name = self.text()
        word = self.u32()
        count = self.u32()
        assert count < 100, (begin, count)
        params = [self.param() for _ in range(count)]
        return {'span': [begin, self.at - begin], 'version': version, 'name': name,
                'opcode': dc.elements[name]['index'], 'word_10': word, 'parameters': params}

    def manager(self):
        version = self.u32()
        assert version in (4, 5), version
        next_id = self.u32()
        triggers = [self.trigger() for _ in range(self.u32())]
        variables = [{'name': self.text(), 'bits': self.u32()} for _ in range(self.u32())]
        if version == 4:
            self.text()
            self.u32()
            strings = []
        else:
            strings = [self.text() for _ in range(self.u32())]
        assert self.at == len(self.b), (self.at, len(self.b))
        return {'version': version, 'next_id': next_id, 'triggers': triggers, 'variables': variables, 'strings': strings}

def chunks(path):
    b = path.read_bytes()
    if b[:8] != b'TGCK\x02\0\0\0':
        return None, b
    count = struct.unpack_from('<I', b, 8)[0]
    names = []
    for i in range(count):
        e = b[len(b) - count * 96 + i * 96:len(b) - count * 96 + (i + 1) * 96]
        names.append(e[:64].decode('utf-16le').split('\0')[0])
    return names, b

def main():
    out = {}
    for path in sorted(DATA.rglob('*')):
        if path.suffix.lower() != '.rmp':
            continue
        rel = path.relative_to(DATA).as_posix()
        names, b = chunks(path)
        rec = {'sha256': hashlib.sha256(b).hexdigest(), 'chunks': names}
        if names is None:
            rec['status'] = 'not TGCK v2'
        elif 'TRIGGERS' not in names:
            rec['status'] = 'no TRIGGERS chunk'
        else:
            data, pins = dc.chunk(path, 'TRIGGERS')
            try:
                m = Reader(data).manager()
                counts = {'events': collections.Counter(), 'conditions': collections.Counter(), 'actions': collections.Counter()}
                items = collections.Counter()
                for t in m['triggers']:
                    for label in counts:
                        for e in t[label]:
                            counts[label][e['name']] += 1
                            for p in e['parameters']:
                                items[(e['name'], p['argument'], p['item'])] += 1
                rec.update(status='decoded', triggers=len(m['triggers']), variables=len(m['variables']), strings=len(m['strings']),
                           enabled=sum(bool(t['enabled']) for t in m['triggers']),
                           modes=dict(collections.Counter(t['mode'] for t in m['triggers'])),
                           elements={k: dict(v) for k, v in counts.items()},
                           items=[[a, b_, c, n] for (a, b_, c), n in sorted(items.items())],
                           chunk=pins)
            except Exception as ex:  # report, never hide
                rec.update(status='decode error', error=repr(ex), chunk=pins)
        out[rel] = rec
    (HERE / 'census.json').write_text(json.dumps(out, indent=1) + '\n')
    for k, v in out.items():
        print(k, v['status'], v.get('triggers'), v.get('error', ''))

if __name__ == '__main__':
    main()
