from pathlib import Path
import json,hashlib
H=Path(__file__).resolve().parent;R=H.parents[1]
files=[
 'crates/aa-runtime/src/property_fields.rs',
 'crates/aa-runtime/src/staged_property_instances.rs',
 'crates/aa-runtime/src/actor_property_focus.rs',
 'crates/aa-runtime/src/load_property_instances.rs',
 'crates/aa-runtime/src/load_property_numeric.rs',
 'crates/aa-runtime/tests/staged_property_instances.rs',
 'crates/aa-runtime/tests/load_property_instances.rs',
 'crates/aa-runtime/tests/property_numeric.rs',
]
out=[]
def pin(p):return dict(path=str(p.relative_to(R)).replace('\\','/'),sha256=hashlib.sha256(p.read_bytes()).hexdigest(),bytes=p.stat().st_size)
for name in files:
 src=R/name;dst=H/'source-v1'/name;dst.parent.mkdir(parents=True,exist_ok=True)
 if dst.exists():assert dst.read_bytes()==src.read_bytes()
 else:dst.write_bytes(src.read_bytes())
 out.append(pin(src));out.append(pin(dst))
src=R/'crates/aa-runtime/src/load_performer_value_refresh.rs'
text=src.read_text();region=text[text.index('pub(crate) fn local_ability('):text.index('\nstruct Runtime<')]
dst=H/'source-v1/local-ability-helper-region.rs.txt'
if dst.exists():assert dst.read_text()==region
else:dst.write_text(region)
out.append(pin(dst))
for name in ['actual-inputs.json','native-stored-properties.json','native_stored_properties.py','README.txt']:
 out.append(pin(H/name))
bins=['staged_property_instances-ff34b07d3cab639b.exe','load_property_instances-070dc38f6ebf6176.exe','property_numeric-5b85490e310f7f13.exe']
for name in bins:out.append(pin(R/'target/s-owner-recruitment/release/deps'/name))
doc=dict(schema=1,scope='bounded stored insertion and borrowed saved/stored fields; no initialized mission, admission or completed positive stored Performer claim',pins=out)
p=H/'freeze.json'
if p.exists():assert json.loads(p.read_text())==doc
else:p.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(pins=len(out),freeze=hashlib.sha256(p.read_bytes()).hexdigest())))
