"""Actual saved4097 source-bound primary list mapped into both PE rank kernels.

Only local-nullness, signed local60/64 bounds and enabled bytes are inputs to
57E000/57DCF0. Handles/addresses are transported to fresh native allocations.
Changed rows stop before4D1350; the Rust adapter's real tail result is separate.
"""
from pathlib import Path
import json,hashlib,sys
import native_performer_init as oracle
HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 source=HERE/'actual-tutorial4097.json';actual=json.loads(source.read_text());rows=[];pins={}
 for row in actual['rows']:
  specs=[]
  for value in row['input']:
   bounds=value['limits']
   specs.append(dict(local=bounds is not None,minimum=bounds[0]if bounds else 0,maximum=bounds[1]if bounds else 0,enabled=value['enabled']))
  case=dict(kind='init',abilities=specs);pc=row['live_precision']
  original,a=oracle.run(case,'original',pc);v14,b=oracle.run(case,'v14',pc);assert original==v14
  assert original['enabled']==[int(x)for x in row['enabled_after']]
  changed=any(x['enabled']!=bool(y)for x,y in zip(specs,original['enabled']))
  assert(original['outcome']=='required')==changed
  rows.append(dict(owner=actual['owner'],precision=pc,input=case,output=original));pins.update(a);pins.update(b)
 executed=[]
 for va,raw in sorted(pins.items()):
  if va==0x4d1350:continue
  data=bytes.fromhex(raw);assert oracle.ORIGINAL.range(va,len(data))==oracle.V14.range(va,len(data))==data
  executed.append(dict(va=hex(va),bytes=raw))
 doc=dict(schema=1,scope=__doc__,producer_sha256=sha(Path(__file__)),actual_source_capture_sha256=sha(source),base_oracle_sha256=sha(HERE/'native_performer_init.py'),rows=rows,separate_machine_runs=2*len(rows),executed_instructions=executed,successful_seams=[])
 path=HERE/'actual-native-prefix.json'
 if '--write'in sys.argv:
  with path.open('x',encoding='utf-8')as f:f.write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),separate_machine_runs=2*len(rows),executed_pins=len(executed),sha256=sha(path))))
if __name__=='__main__':main()
