"""Generate immutable registration metadata, never current UVar storage."""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
p=HERE/'native-uvar-registration.json';doc=json.loads(p.read_text());rows=doc['cases'][0]['registrations']
assert len(rows)==213 and all(c['registrations']==rows for c in doc['cases'])
lines=['//Generated from actual41F9D0 constructor captures; no mutable values.',
 '//Fixture SHA256 '+hashlib.sha256(p.read_bytes()).hexdigest(),
 'use super::Registration;', 'pub(super) const REGISTRATIONS: [Registration; 213] = [']
for i,r in enumerate(rows):
 assert i==r['index'];name=''.join(map(chr,r['name']));assert name.isascii()
 lines.append('    Registration { index: %d, name: %s, native_kind: %d, flags: %d },'%(i,json.dumps(name),r['kind'],r['flags']))
lines.append('];');value='\n'.join(lines)+'\n';target=ROOT/'crates/aa-ui/src/view_variable_registration.rs'
if '--write'in sys.argv:target.write_text(value)
else:assert target.read_text()==value
print(hashlib.sha256(target.read_bytes()).hexdigest())
