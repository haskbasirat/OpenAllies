from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
results=[]
for name in ['native-view-task-queue-final.json','native-queue-waiter-constructor.json']:
 p=HERE/name;d=json.loads(p.read_text());verified=[]
 for instruction in d['instructions']:
  va=int(instruction['va'],16);b=bytes.fromhex(instruction['bytes'])
  assert n.original.range(va,len(b))==n.modified.range(va,len(b))==b
  verified.append(va)
 results.append(dict(fixture=name,sha256=n.sha(p.read_bytes()),rows=len(d['rows']),executions=d['native_executions'],recorded_pins=len(verified),executed_pins=len(verified)-(0x5ae022 in verified),observed_cutoffs=['0x5ae022']if 0x5ae022 in verified else[],same_original_v14_bytes=True))
#No fixture/producer rewrites on immutable validation.
out=dict(original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),results=results)
(HERE/'pin-verification.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out))
