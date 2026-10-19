"""Compare a Rust NIF trace dump (target/nif-trace-dump.tsv) with the native
trace of the same file: from native-nif-samples.json when it is a sample,
otherwise from a fresh oracle run (native_nif_load.load)."""
from pathlib import Path
import sys, json
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
path = sys.argv[1]
rust = [tuple(int(x) for x in l.split('\t')) for l in (HERE.parents[1] / 'target/nif-trace-dump.tsv').read_text().splitlines()]
native = None
for s in json.loads((HERE / 'native-nif-samples.json').read_bytes())['samples']:
    if s['path'] == path:
        native = [tuple(r) for r in s['reads']]
if native is None:
    import native_nif_load as N
    data = (HERE.parents[2] / 'reconstruction/assets/original/Data' / path).read_bytes()
    native = [tuple(r) for r in N.load(data, N.registry(), vtables={})['reads']]
for i, (a, b) in enumerate(zip(rust, native)):
    if a != b:
        print('first difference at read', i, 'rust', a, 'native', b)
        print('context native', native[max(0, i - 5):i + 3])
        print('context rust  ', rust[max(0, i - 5):i + 3])
        break
else:
    print('identical prefix', min(len(rust), len(native)), 'rust', len(rust), 'native', len(native))
