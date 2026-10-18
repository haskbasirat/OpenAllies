"""Actual moved-owner57E000 captured inputs, native enabled-byte prefix.
Both images execute the actual gate. A changed-byte4D1350 is a required stop,
not a successful child seam or full Actions parity. Current shipped inputs
have no constrained ranks; duplicate phase rows remain explicitly counted.
"""
from pathlib import Path
import sys,json,hashlib
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-performer-init'))
import native_performer_init as native
def main():
 source=HERE/'actual-stored-performer-init.json';doc=json.loads(source.read_bytes());rows=[];pins={};unique=set()
 for row in doc['rows']:
  case=row['input'];pc=row['precision'];a,pa=native.run(case,'original',pc);b,pb=native.run(case,'v14',pc)
  assert a==b and pa==pb
  assert a['enabled']==[int(v)for v in row['enabled_after']]
  assert (a['outcome']=='required')==row['changed']
  if not row['changed']:assert row['result']=={'Ok':True}
  rows.append({'owner':row['owner'],'phase':row['phase'],'precision':pc,'input':case,'output':a,'rust_full_result':row['result']});pins.update(pa)
  unique.add(json.dumps([pc,case],sort_keys=True))
 executed={}
 for va,raw in sorted(pins.items()):
  if va==0x4d1350:continue
  assert native.ORIGINAL.range(va,len(bytes.fromhex(raw)))==native.V14.range(va,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
  executed[f'{va:08x}']=raw
 out={'schema':1,'scope':__doc__,'captured_rows':len(rows),'unique_precision_inputs':len(unique),'separate_native_executions':2*len(rows),'changed_rows':sum(r['changed']for r in doc['rows']),
  'capture_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'successful_seams':[],'rows':rows,'executed_instruction_pins':executed}
 p=HERE/'native-actual-stored-performer-init.json'
 if '--write'in sys.argv:
  with p.open('x')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert json.loads(p.read_bytes())==out
 print(json.dumps({'captured_rows':len(rows),'unique_precision_inputs':len(unique),'native_executions':2*len(rows),'executed_pins':len(executed),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
