"""Refresh only the registry expectations from the corrected native audit.

The historical phase4 mechanics bundle is immutable. All its nonregistry
expected values are retained. The inserted native323 and final two IDs are
selected explicitly to catch the missing-initializer/shift regression.
"""
from pathlib import Path
import copy, hashlib, json, sys
ROOT=Path(__file__).resolve().parents[2]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
old=ROOT/'research/phase4-orders/mechanics-samples.json'
oracle=ROOT/'research/phase4-orders/orders-native-oracle.json'
audit=ROOT/'research/skirmish-20261017-order-registry-correction/native-registry-audit.json'
assert sha(old)=='4cd39bb12e499d851f634837f8ad825822c3a28f2457df3e4cba1b1441cedce9'
assert sha(oracle)=='d9ae9925730f88b63e262616c2ee9aaab1fb19f5d57faa735528ac4c855ab137'
assert sha(audit)=='7d0ef97a4ea1a9a565016ecc25fe1c2fe27dff756910c71990268cd038989b76'
source=json.loads(old.read_text());current=json.loads(oracle.read_text());proof=json.loads(audit.read_text())
registry=current['registry'];assert len(registry)==358
for result in proof['results']:
    assert result['registrations']==registry
assert registry[323]['initializer']=='0x9e65f0'
assert registry[356]['name']=='KKC_TellMGCommandOrderRequest'
assert registry[357]['name']=='KKC_TellMGCommandOrderDecree'
out=copy.deepcopy(source)
out['order_registry']={'count':len(registry),'samples':[registry[i]for i in [0,47,66,84,86,298,323,356,357]]}
out['source_oracles']['orders']={'path':oracle.relative_to(ROOT).as_posix(),'sha256':sha(oracle)}
out['source_oracles']['order_registry_audit']={'path':audit.relative_to(ROOT).as_posix(),'sha256':sha(audit)}
out['derivation']={'scope':__doc__,'historical_bundle':{'path':old.relative_to(ROOT).as_posix(),'sha256':sha(old)},'producer_sha256':sha(Path(__file__))}
for key in source:
    if key not in ('order_registry','source_oracles'):assert out[key]==source[key]
path=ROOT/'research/phase4-orders/mechanics-samples-registry358.json'
data=(json.dumps(out,indent=2)+'\n').encode()
if '--check'in sys.argv:assert path.read_bytes()==data
else:
    with path.open('xb')as f:f.write(data)
print(json.dumps({'passed':True,'registry_count':len(registry),'sample_count':len(out['order_registry']['samples']),'sha256':hashlib.sha256(data).hexdigest()}))
