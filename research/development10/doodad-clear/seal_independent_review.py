"""Read-only source/native/binary evidence audit; never regenerates fixtures."""
from pathlib import Path
import hashlib,json,sys
ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
findings=json.loads((HERE/'findings.json').read_text())
for name,digest in findings['sources'].items():assert sha(ROOT/name)==digest,(name,'source pin differs')
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified
doc=json.loads((HERE/'native-clear.json').read_text())
seams={int(a,16) for a in doc['declared_seams']}
executed=0
for pin in doc['observed_pins']:
 address=int(pin['address'],16);raw=bytes.fromhex(pin['bytes'])
 assert original.range(address,len(raw))==raw
 assert modified.range(address,len(raw))==raw
 executed+=address not in seams
assert (executed,len(seams))==(279,6)
files=list(findings['sources'])+[
 'research/development10/doodad-clear/findings.json',
 'research/development10/doodad-clear/independent-native-replay.log',
 'research/development10/doodad-clear/independent-rust-clear.log',
 'research/development10/doodad-clear/independent-rust-trigger.log',
 'target/root-zone2/release/deps/native_doodad_clear-ba39b74164b7cb54.exe',
 'target/root-zone2/release/deps/native_thing_references-74c82c3c5c6b6c2b.exe',
]
out={
 'schema':1,'result':'Accepted within the declared coordinator/trigger-reference scope; no blocking defect found',
 'reviewer':'state_binding_review','source_review':['native class2/9 then bit10 then SAME manager membership are lazy',
   'overlap GTIDs collected before deletion, resolved afresh in collected order; later current flags reread',
   'loaded Trigger list then events/conditions/actions/parameters preserve native first-match order',
   'only argument5/11/10 and items12/31/30 or name15 participate',
   'case-sensitive counted UTF16 name comparison stops at first NUL; current name getter is lazy',
   'definition IDS precede current GTID before diagnostic/destruction'],
 'reviewer_execution':{'native':'fresh producer --check PASS153pairs/306executions/allPC/bothimages',
   'rust':'existing frozen owner-built binaries: doodad2/2, trigger1/1 PASS; no Cargo/build',
   'byte_validation':'all285 fixture pins match both unchanged image bytes;279executed/6declaredseams'},
 'limits':['whole native overlap descriptor/query, Thing-name ownership and destruction are explicit child seams',
   'frozen binaries were reused, not rebuilt independently; exact artifacts are hashed below',
   'unknown decoded argument/type and stale generations are portable required-error guards, not native invalid-memory equivalence',
   'no concrete runtime World binding, terrain/startup acceptance or playable mission claim',
   'logger threshold/formatting, allocator lifetime/Win32/unwind/native destructor equivalence remain outside this proof'],
 'files':{name:sha(ROOT/name) for name in files},
}
(HERE/'independent-state-binding-review.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'review_sha256':sha(HERE/'independent-state-binding-review.json'),'source_pins_verified':len(findings['sources'])}))
