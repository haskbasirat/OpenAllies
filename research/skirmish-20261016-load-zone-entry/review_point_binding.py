"""Independent static PointShape owner review; no hardware-capture replay."""
from pathlib import Path
import hashlib
import json
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
def pin(p):
    raw=p.read_bytes()
    return {'path':p.relative_to(ROOT).as_posix(),'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
source=ROOT/'crates/aa-runtime/src/load_world_globals.rs'
reference=ROOT/'research/skirmish-20261016-load-world-globals/point-startup-reference.json'
assert pin(source)['sha256']=='f2e5fefce3272a55ebf8fa3450bd33cd8c69d1ebb2c2425dd93409535a77372d'
assert pin(reference)['sha256']=='d50ac9ddb3d22d77c933068ebedb0d77fe700e24ec3511141f618ad3b7d15700'
doc={
 'schema':1,'status':'accepted within bounded static source-owner scope',
 'reviewer':'sdk_native_geometry_review',
 'scope':'Read-only PointShape construction and reached destination-Catalog reuse guard; separate from native capture and source-completion oracle certification',
 'accepted':[
  'Private Shape/provenance/context and sole verified-reference constructor; no Default, Deserialize, Clone or public raw-word constructor',
  'Fixed included receipt digest, schema/address/kind, runtime pins, source/capture identities, all six watched words and two calibrated complete cutoff runs are checked',
  'Destination Catalog pointer owner/source fingerprint and sealed startup FP target are checked; current Settings/numeric bindings are revalidated against the reference',
  'validate_catalog repeats destination source and current numeric checks at reached point reads, retaining the private verified FP context so edited Settings cannot reuse an earlier successful point',
  'SessionPhase E4 and F8 are distinct reads; phase-only hosts fail explicitly if an E4 service is reached',
 ],
 'limits':[
  'Did not independently execute or certify the two Windows hardware-watch captures; that read-only capture review belongs to world_startup_gap',
  'Did not recertify own native geometry/zone proofs through this independent Point binding review',
  'Captured scope ends at source completion/Session construction, not first map, battle, or perpetual native global immutability',
  'No claim that a current Rust thread FP state or live native process is observed by this binder',
  'Independent static acceptance does not replace author tests or root integration tests of actual retained Session/Point owners',
 ],
 'pins':[pin(source),pin(reference)],
}
(HERE/'point-binding-review.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(pin(HERE/'point-binding-review.json')))
