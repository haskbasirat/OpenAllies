from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
native=json.loads((HERE/'independent-native-replay.json').read_text())
for file,pin in native['protected_file_sha256'].items():assert sha(ROOT/file)==pin,file
log=(HERE/'independent-tests.log').read_text()
assert '2 passed; 0 failed' in log
assert 'actual Team records checked: 2' in log and '4d6bfb257a9ddf31ad4119251e77ead5e6ac9a569f4dacc187643d3d20695281' in log
evidence=['independent_review.py','independent-native-replay.json','independent-native.log','independent-tests.log','independent-instructions.txt']
report={
 'schema':1,'date':'2026-10-16','reviewer':'organization_morale_source','independent_of_author':True,
 'decision':'No blocking findings in saved Team Init(NULL) coordinator scope',
 'blocking_findings':[],
 'source_and_fixture_sha256':native['protected_file_sha256'],
 'author_findings_sha256':sha(HERE/'findings.json'),
 'evidence_sha256':{p:sha(HERE/p) for p in evidence},
 'native_replayed':native['replayed_rows'],'additional_native_rows':len(native['extra_rows']),
 'images':['original','v14'],'precisions':['Single24','Double53','Extended64'],
 'native_fixture_bytes_preserved':native['protected_bytes_unchanged'],
 'rust_tests':{'passed':2,'failed':0,'include_ignored':True,'scope':'aa-mapworld --test native_team_init only'},
 'verified':[
  'Real Team vtableA29780+E8 targets4E68C0; +C targets4E3E90',
  'Team+C returns constantB3A8C4 with RET and consumes no arguments; already stacked Thing+20/tile size reach46EE00 unchanged',
  'NULL creation block skips the nonnull write-back path',
  'Flag16 -> tile size -> snap -> placement(NULL) -> index -> fresh flags -> conditional terrain -> binary32 Z store -> name snapshot/log',
  'Placement uses only lowAL; 256 is false and255 is true',
  'Post-index bit21 selects123 even whenbit12 is set; otherwise clearbit12 selects122; setbit12 skips height',
  'False returns before index/height/name/log and keeps earlier snap effects',
  'Fallible callbacks preserve all earlier snap/index/height effects; log failure cannot roll them back',
  'World methods have no default success; actual spatial/placement/terrain/log services remain mandatory',
  'Natural Firefight TEAMS records team_0/team_1 with flagsA0042800 reach required4EF6A0 error with fields preserved; tutorial02 has no present Team records',
 ],
 'scope_limits':[
  'Snap46EE00, placement4EF6A0, spatial4EE4C0, terrain4F0E50 and log404F60 bodies are explicit native oracle seams',
  'CString reference transport4012C0 is an infrastructure seam; full diagnostic formatting/rendering (including embeddedNUL) is not proven by UTF16 snapshot probes',
  'Failure injection stops native execution in a service; native SEH unwind/destructor behavior is not claimed',
  'Terrain fixtures use finite binary64 values loaded into ST0; API supports exact Binary but actual terrain producer is not bound',
  'Nonnull creation, complete runtime Team Init, PostCreate/4F9510, battle/publication and multiplayer are not implemented or accepted here',
 ],
 'natural_map_result':'Required placement service unavailable; not success and not full map acceptance',
 'production_files_changed_by_review':False,
 'author_fixture_files_changed_by_review':False,
}
(HERE/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'decision':report['decision'],'native_replayed':report['native_replayed'],'extra_rows':report['additional_native_rows'],'rust_tests':report['rust_tests'],'sha256':sha(HERE/'independent-review.json')},indent=2))
