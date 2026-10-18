import sys, time, json
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_layout_data as N

name = sys.argv[1] if len(sys.argv) > 1 else 'basic'
t0 = time.time()
m = N.LayoutOracle()
m.setup('Double53')
print('setup', round(time.time() - t0, 1), flush=True)
if name == 'shipped':
    files = [(p, (N.DATA / p).read_bytes()) for p in N.SHIPPED]
else:
    files = N.synthetic_cases()[name]
for p, d in files:
    t1 = time.time()
    e = m.process(p, d)
    print(p, e['returned'], e['caught'], e['throw'], round(time.time() - t1, 1), flush=True)
out = m.dump()
print(json.dumps(out, indent=1)[:int(sys.argv[2]) if len(sys.argv) > 2 else 6000])
