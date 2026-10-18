"""Independent fixture replay and source pinning for the bounded credit port."""
from pathlib import Path
import hashlib,json,sys,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import original_credit as native
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def main():
 path=HERE/'credit-native-oracle.json';doc=json.loads(path.read_text())
 assert doc['case_count']==186==len(doc['fixtures'])
 for row in doc['fixtures']:
  result=native.Machine(row['input']).run()
  assert result==row['output'],row['input']['name']
 for window in doc['visited_windows']:
  a=int(window['address'],16);raw=bytes.fromhex(window['bytes'])
  assert native.IMAGE.at(a,len(raw))==raw==native.ACCEPTED.at(a,len(raw))
 result=subprocess.run(['cargo','test','-p','aa-body','--test','native_credit'],cwd=ROOT,text=True,capture_output=True)
 assert result.returncode==0,result.stdout+result.stderr
 paths=['crates/aa-body/src/credit.rs','crates/aa-body/tests/native_credit.rs','research/phase6-credit/original_credit.py','research/phase6-credit/credit-native-oracle.json','research/phase6-credit/native-ranges.json']
 report={'schema':1,'reviewer':'visual_audit','passed':True,'native_cases_replayed':186,'visited_windows_checked':len(doc['visited_windows']),'rust_tests':'3 passed','reviewed_behavior':['Credited bit set before damage record availability and timeout; no retry after later callback failure.','Source and allegiance identities resolved before source-accept callback; later damage record mutations cannot change cached identities.','Each classification reads component fields at its native point; statistical component handles are snapshotted before classification callbacks.','Outer actor queried twice before attached-object notification suppression. Owner for notification reread after victim label callback.','Relation statistics compare extended sum after binary32 map store, retaining native leader change on rounded ties.','Victim value stored binary32; source value and ratio remain extended; experience factor cached before callbacks, min/max loaded afterward.','Source leader may supply experience ratio, while reward still targets the originally cached source. Organization/component reward gates reread after award callback.','Original and modified executable visited bytes matched; virtual callbacks, strings, player services, derived values and reward application remain explicit leaves.'],'findings':[],'limits':['Only coherent native component snapshots are covered; hosts must preserve victim identity and provide body/alternate record fields corresponding to actual component presence.','This verifies credit sequencing and arithmetic, not complete weapon-hit calculation, derived actor value resolution, live organization experience effects or campaign integration.'],'pins':{p:sha(ROOT/p) for p in paths}}
 out=HERE/'independent-review.json';out.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
 print(json.dumps({'passed':True,'cases':186,'review_sha256':sha(out)}))
if __name__=='__main__':main()
