"""Write native-nif-corpus.tsv (the Rust test fixture) from native-nif-corpus.json."""
from pathlib import Path
import json, hashlib
HERE = Path(__file__).resolve().parent
doc = json.loads((HERE / 'native-nif-corpus.json').read_bytes())
cols = ['path', 'sha256', 'size', 'ok', 'version', 'user_version', 'objects', 'classes_sha256', 'reads', 'consumed',
        'trace_sha256', 'link_ids', 'link_ids_sha256', 'link_counts', 'link_counts_sha256', 'pops', 'pops_sha256',
        'roots', 'destroyed']
lines = ['\t'.join(cols)]
for r in doc['rows']:
    assert 'abort' not in r, r['path']
    row = dict(r)
    row['roots'] = ','.join('n' if x is None else str(x) for x in r['roots'])
    row['destroyed'] = ','.join(str(x) for x in r['destroyed'])
    lines.append('\t'.join(str(row[c]) for c in cols))
raw = ('\n'.join(lines) + '\n').encode()
(HERE / 'native-nif-corpus.tsv').write_bytes(raw)
print(len(lines) - 1, hashlib.sha256(raw).hexdigest())
