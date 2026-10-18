"""Bounded Mover/Element embedded Updatable availability. Constructor fixture
supplies actual embedded vtable identities; actual table slot and leaf execute
from each untouched PE at all3PC. No whole constructor/scheduler/Init proof.
"""
from pathlib import Path
import json,hashlib,sys,struct
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
ORIGINAL=nm.original
CONSTRUCTOR=ROOT/'research/phase7-actors/component-constructors-native.json'
CONSTRUCTOR_SHA='68a4806f7d0375f0ad3fa7a0e65e5597626573e95237e8fb08c4a452089eb69e'
def main():
 assert hashlib.sha256(CONSTRUCTOR.read_bytes()).hexdigest()==CONSTRUCTOR_SHA
 ctor=json.loads(CONSTRUCTOR.read_bytes());rows=[];pins={};tables={}
 for kind,table in [('Mover',0xa3ce5c),('Element',0xa3c6c8)]:
  source=[r for r in ctor['cases']if r['kind']==kind]
  assert len(source)==48 and all(r['output']['initialized_words']['8']==table for r in source)
  raw=ORIGINAL.range(table+8,4);assert raw==nm.modified.range(table+8,4)
  target=struct.unpack('<I',raw)[0];assert target==0x6a71d0
  tables[f'{table+8:08x}']=raw.hex()
  for pc in nm.PRECISIONS:
   results=[]
   for image in [ORIGINAL,nm.modified]:
    nm.original=image;m=nm.Machine(pc);owner=m.alloc(0x10);m.w(owner,table)
    result=m.call(target,owner,[]);assert result&255==1 and not m.events
    results.append(result&255);pins.update(m.visited)
   assert results==[1,1]
   rows.append({'component':kind,'precision':pc,'embedded_table':table,'virtual8':target,'al':1})
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 out={'schema':1,'scope':__doc__,'constructor_reference':str(CONSTRUCTOR.relative_to(ROOT)),
  'constructor_sha256':CONSTRUCTOR_SHA,'paired_rows':len(rows),'native_executions':len(rows)*2,
  'successful_child_seams':[],'rows':rows,'table_data_pins':tables,
  'executed_instruction_pins':{f'{va:08x}':raw.hex()for va,raw in sorted(pins.items())}}
 target=HERE/'native-embedded-updatable.json'
 if '--write'in sys.argv:
  with target.open('x')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert json.loads(target.read_bytes())==out
 print(json.dumps({'rows':len(rows),'executions':2*len(rows),'pins':len(pins),'sha256':hashlib.sha256(target.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
