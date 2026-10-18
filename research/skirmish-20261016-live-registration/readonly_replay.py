"""Re-execute frozen native fixtures without regenerating/replacing them."""
from pathlib import Path
import hashlib,json,struct
import native_registration as registration
import native_lifecycle as lifecycle
HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 h=registration.h;original=h.P;reports=[]
 for name,producer in [('native-registration.json',registration),('native-lifecycle.json',lifecycle)]:
  path=HERE/name;before=sha(path);doc=json.loads(path.read_text());count=0
  try:
   for row in doc['cases']:
    h.P=original if row['image']=='original'else h.V
    assert producer.Machine(row['input'],row['precision']).run()==row['output'],row['input']['name']
    count+=1
   if name=='native-registration.json':
    for row in doc['team_initialization']:
     h.P=original if row['image']=='original'else h.V
     m=h.Machine(row['precision']);owner=m.alloc(12);buffer=m.alloc(32);m.w32(owner,buffer,0,8);m.w32(buffer,*([0xdeadbeef]*8))
     m.seams.update({0x6f7810:lambda m:m.ret(value=m.alloc(m.arg())),0x6f7430:lambda m:m.ret()})
     m.call(0x430240,owner,[row['count']]);assert m.r32(m.r32(owner),row['count'])==row['initialized']
     assert m.r32(owner+8)==row['capacity'];count+=1
   h.P=original
   for pin in doc['instructions']:
    address=int(pin['va'],16);raw=bytes.fromhex(pin['bytes']);assert original.range(address,len(raw))==h.V.range(address,len(raw))==raw
  finally:h.P=original
  assert sha(path)==before
  reports.append({'fixture':name,'sha256':before,'replayed':count,'all_instruction_pins_verified':True,'fixture_preserved':True})
 doc=json.loads((HERE/'native-construction-pins.json').read_text())
 for label in ['class_constructor_vtable_writes','world_empty_list_stores']:
  for pin in doc[label]:
   raw=bytes.fromhex(pin['bytes']);a=int(pin['address'],16);assert original.range(a,len(raw))==h.V.range(a,len(raw))==raw
 report={'passed':True,'fixtures':reports,'native_executions':sum(r['replayed']for r in reports),'static_construction_pins_verified':True,'scope':'Read-only author replay; independent peer review is separate.'}
 out=HERE/'readonly-replay.json';out.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
if __name__=='__main__':main()
