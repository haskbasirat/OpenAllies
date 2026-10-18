"""Extract every original map's decompressed TRIGGERS and MAPSTRINGS chunks (read-only).

Writes chunks/<map>.bin (TRIGGERS), chunks/mapstrings/<map>.txt (MAPSTRINGS)
and chunks/index.json (map path, map SHA256, chunk SHA256s, lengths).  Rust tests replay these bytes without a zlib dependency;
the SHA256 pins tie them to the original RMP archives.
"""
from pathlib import Path
import hashlib, json, re, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research' / 'phase7-triggers'))
import decode_census as dc  # noqa: E402

DATA = ROOT.parent / 'reconstruction/assets/original/Data/Maps'


def main():
    census = json.loads((HERE / 'census.json').read_text())
    out = HERE / 'chunks'
    out.mkdir(exist_ok=True)
    (out / 'mapstrings').mkdir(exist_ok=True)
    index = []
    for rel, rec in sorted(census.items()):
        if rec['status'] not in ('decoded', 'decode error'):
            continue
        data, pins = dc.chunk(DATA / rel, 'TRIGGERS')
        name = re.sub(r'[^A-Za-z0-9_.-]', '_', rel.replace('/', '__')) + '.bin'
        (out / name).write_bytes(data)
        text, text_pins = dc.chunk(DATA / rel, 'MAPSTRINGS')
        assert text_pins['map_sha256'] == pins['map_sha256']
        text_name = name[:-4] + '.txt'
        (out / 'mapstrings' / text_name).write_bytes(text)
        index.append({'map': rel, 'file': name, 'map_sha256': pins['map_sha256'],
                      'chunk_sha256': pins['chunk_sha256'], 'length': len(data),
                      'manager_version': int.from_bytes(data[:4], 'little'),
                      'triggers': rec.get('triggers'), 'campaign': rel.startswith('Campaign/'),
                      'elements': rec.get('elements'),
                      'mapstrings_file': 'mapstrings/' + text_name,
                      'mapstrings_sha256': text_pins['chunk_sha256'],
                      'mapstrings_length': len(text)})
    (out / 'index.json').write_text(json.dumps(index, indent=1) + '\n')
    print(len(index), sum(i['length'] for i in index))


if __name__ == '__main__':
    main()
