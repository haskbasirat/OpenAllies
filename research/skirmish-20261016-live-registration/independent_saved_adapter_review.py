"""Independent immutable native replay by the saved-adapter reviewer.
No production source or author fixture/receipt is modified.
"""
from pathlib import Path
import hashlib,json
import native_registration as registration
import native_lifecycle as lifecycle
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 h=registration.h;original=h.P;reports=[]
 for name,producer in [('native-registration.json',registration),('native-lifecycle.json',lifecycle)]:
  path=HERE/name;before=sha(path);document=json.loads(path.read_text());count=0
  try:
   for row in document['cases']:
    h.P=original if row['image']=='original' else h.V
    assert producer.Machine(row['input'],row['precision']).run()==row['output'],row['input']['name'];count+=1
   if name=='native-registration.json':
    for row in document['team_initialization']:
     h.P=original if row['image']=='original' else h.V;m=h.Machine(row['precision']);owner=m.alloc(12);buffer=m.alloc(32);m.w32(owner,buffer,0,8);m.w32(buffer,*([0xdeadbeef]*8))
     m.seams.update({0x6f7810:lambda m:m.ret(value=m.alloc(m.arg())),0x6f7430:lambda m:m.ret()});m.call(0x430240,owner,[row['count']])
     assert m.r32(m.r32(owner),row['count'])==row['initialized'] and m.r32(owner+8)==row['capacity'];count+=1
   for pin in document['instructions']:
    address=int(pin['va'],16);raw=bytes.fromhex(pin['bytes']);assert original.range(address,len(raw))==h.V.range(address,len(raw))==raw
  finally:h.P=original
  assert sha(path)==before;reports.append(dict(fixture=name,sha256=before,calls=count,pins=len(document['instructions']),preserved=True))
 source=['crates/aa-runtime/src/live_registration.rs','crates/aa-mapworld/src/record_lifecycle.rs']
 report=dict(schema=1,status='native_replay_passed',reviewer='saved_leader_adapter',native_calls=sum(row['calls'] for row in reports),fixtures=reports,
  source_sha256={name:sha(ROOT/name) for name in source},
  source_audit=['Actual held record and same registration State retained; no sorted-registry reconstruction or successful actor-registration default.','Names captured before flags clear; class/team/available-city and assertion-latch stores remain ordered and partial.','Source-space lookup is lazy after native guards; current/self/stale generation references are explicit.','Lifecycle rereads the authoritative out slot at native boundaries; required null dereferences, failed Init warning/destruction and clear-after-return are explicit.','Object world+8, Actor state/grid effects, Init/PostCreate bodies, destruction and definition publication remain required callbacks.'],
  limitations=['No complete battle/startup claim','Concrete callback bodies and arbitrary destruction/replacement during stable-record registration remain outside this coordinator proof','Scoped independent Rust replays reported separately'])
 out=HERE/'independent-saved-adapter-native.json';out.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(native_calls=report['native_calls'],sha256=sha(out))))
if __name__=='__main__':main()
