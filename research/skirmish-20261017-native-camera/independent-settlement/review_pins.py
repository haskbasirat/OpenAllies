from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;EVIDENCE=HERE.parent;ROOT=EVIDENCE.parents[1]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
freeze=EVIDENCE/'directory-freeze-v2.json';assert H(freeze)=='f6f6a5a3a0ae6c7ccefc64a0e9e7b43c974d325650fda688f35ac1df51e734c9'
d=json.loads(freeze.read_text());pins={}
for group in['production','fixtures','producer_pins','extra_native']:
 for name,expected in d[group].items():
  p=(ROOT/name)if name.startswith('crates/')else(EVIDENCE/name)
  actual=H(p);assert actual==expected,(name,actual,expected);pins[str(p.relative_to(ROOT))]=actual
b=d['exact_source_test_binary'];assert H(EVIDENCE/b['path'])==b['sha256']
drift={}
for name,expected in d['current_shared_snapshot_pins'].items():
 actual=H(ROOT/name)
 if actual!=expected:drift[name]=dict(frozen_snapshot=expected,current=actual)
out=dict(schema=1,freeze_sha256=H(freeze),verified_author_owned_and_fixture_pins=pins,
 frozen_test_binary=dict(path=b['path'],sha256=b['sha256']),shared_successor_drift=drift,
 qualifier='Shared wholefiles are snapshots; drift is reported separately, never attributed to the frozen camera receipt. No build, native process or production edit.')
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'pins.json';p.write_bytes(data)
print(json.dumps(dict(verified_pins=len(pins),shared_successor_drift=list(drift),sha256=hashlib.sha256(data).hexdigest())))
