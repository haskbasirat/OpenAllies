"""Ordinary class3 Actor virtualC actual vtable/getter receipt.
Native4D6BB0/4D6D70 stores4D6BE8/4D6DBD establish A273F0; slotC is4EA010.
Only4EA010 executes here. The constructor store and table word are static
original/v14 data pins; the prior full constructor fixture is cited separately.
"""
from pathlib import Path
import json,sys,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_member_deployment as common
nm=common.nm
def main():
 table=0xa273f0;slot=table+0xc;ctor=0x4d6dbd;fresh_ctor=0x4d6be8
 assert nm.original.range(ctor,6)==nm.modified.range(ctor,6)==bytes.fromhex('c706f073a200')
 assert nm.original.range(slot,4)==nm.modified.range(slot,4)==struct.pack('<I',0x4ea010)
 prior=HERE.parent/'skirmish-20261017-map-actor-creation/native-constructor.json';prior_doc=json.loads(prior.read_text())
 assert {'va':hex(fresh_ctor),'bytes':'c706f073a200'}in prior_doc['instructions']
 assert nm.original.range(fresh_ctor,6)==nm.modified.range(fresh_ctor,6)==bytes.fromhex('c706f073a200')
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for image in ['original','v14']:
   m=common.machine(image,pc);actor=m.alloc(0x180);definition=m.alloc(0x80)
   m.w(actor,table);m.w(actor+4,definition);entry=m.r(m.r(actor)+0xc)
   result=m.call(entry,actor);assert result==definition+0x38
   rows.append(dict(precision=pc,image=image,relative_shape_offset=result-definition));pins.update(m.visited)
 for va,raw in pins.items():assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'prior_constructor_fixture_sha256':hashlib.sha256(prior.read_bytes()).hexdigest(),'static_pins':[{'va':hex(ctor),'bytes':nm.original.range(ctor,6).hex(),'classification':'alternate constructor producer static pin, not executed in this getter lab'},{'va':hex(fresh_ctor),'bytes':nm.original.range(fresh_ctor,6).hex(),'classification':'fresh constructor producer static here; executed in cited prior fixture'},{'va':hex(slot),'bytes':nm.original.range(slot,4).hex(),'classification':'immutable vtable data'}]}
 path=HERE/'native-actor-shape-getter.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(pins),static_pins=3,sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
