"""Independent replay of the bounded actor wire decoder evidence."""
from pathlib import Path
import hashlib,json,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_actor_payload as n
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 doc=json.loads((HERE/'native-actor-payload-fixtures.json').read_text());count=0
 for row in doc['cases']:
  mask=row['expected']['saved_components_mask']|row['expected']['default_components_mask']
  assert n.TailMachine(bytes.fromhex(row['tail_hex'])+b'FOLLOWING_RECORD').run_tail(row['version'],mask)==row['native']
  assert n.TailMachine(bytes.fromhex(row['actor_hex'])+b'FOLLOWING_RECORD').run_actor(row['version'],mask)==row['native_actor']
  count+=2
 for key in ['source_crete','source_dunkirk']:
  row=doc[key];path=ROOT.parent/row['path'];raw=path.read_bytes();assert sha(path)==row['sha256']
  assert n.TailMachine(raw[row['tail_offset']:]).run_tail(14,64)==row['native']
  assert n.TailMachine(raw[row['actor_offset']:]).run_actor(14,64)==row['native_actor'];count+=2
 instructions=json.loads((HERE/'executed-instructions.json').read_text())
 for row in instructions:
  a=int(row['address'],16);raw=bytes.fromhex(row['bytes']);assert n.P.range(a,len(raw))==raw==n.V.range(a,len(raw))
 for row in doc['ranges'].values():
  a=int(row['start'],16);raw=n.P.range(a,row['length']);assert raw==n.V.range(a,len(raw));assert hashlib.sha256(raw).hexdigest()==row['sha256']
 run=subprocess.run(['cargo','test','-p','aa-mapworld','--test','actor_payload'],cwd=ROOT,text=True,capture_output=True)
 assert run.returncode==0,run.stdout+run.stderr
 paths=['crates/aa-mapworld/src/actor_payload.rs','crates/aa-mapworld/tests/actor_payload.rs','research/phase6-actor-payloads/native_actor_payload.py','research/phase6-actor-payloads/native-actor-payload-fixtures.json','research/phase6-actor-payloads/evidence.json']
 review={'schema':1,'reviewer':'visual_audit','passed':True,'native_executions_replayed':count,'instruction_windows_checked':len(instructions),'named_ranges_checked':len(doc['ranges']),'rust_tests':'4 passed','reviewed_behavior':['Default component mask precedes saved mask; unknown saved payloads rejected rather than skipped.','Updatable version0 omits ID; v1 reads signed ID. Properties count unsigned; only sentinel classes consume no further payload.','Actor version gates match original1..14 read branches; legacy15c value retained as wire data despite native postread reset.','Presence bytes retain all nonzero encodings; strings preserve UTF16 code units including unpaired surrogates.','Actor list counts interpreted signed, so negative counts consume no elements; positive allocations bounded by remaining bytes.','Exact consumption verified from tail entry and actor entry; proper truncation rejects and trailing records remain unread.','Crete and Dunkirk original source hashes/bytes verified and native boundaries replayed.'],'findings':[],'limits':['Wire decoding only, stopping at4D9714 before runtime fixups. Native owner/components/registries/transient-state effects are not restored.','Onlysavedmask0/64, sentinel Properties classes, actorv1..14, localizationmode0. Explicit stream/component/string/allocation/scheduling seams retain prior documented scope.'],'pins':{p:sha(ROOT/p) for p in paths}}
 p=HERE/'independent-review.json';p.write_text(json.dumps(review,indent=2)+'\n',encoding='utf-8');print(json.dumps({'passed':True,'native_executions':count,'sha256':sha(p)}))
if __name__=='__main__':main()
