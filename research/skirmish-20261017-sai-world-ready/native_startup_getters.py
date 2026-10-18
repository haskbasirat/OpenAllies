"""Actual constructed runtime B0/B4/8C and source Data+38 SAI startup getters.

Real vtable slots execute with poison EAX. Runtime ctor vtable stores are
static byte/address pins, not a claim of full ctor execution in this fixture.
Object8C reads only the supplied authoritative count58. No body seams.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX
H=lambda b:hashlib.sha256(b).hexdigest();O=h.P;V=h.V
CLASSES={1:(0x4ef020,0xa2a680),2:(0x4ea690,0xa2a2a0),3:(0x4d6d70,0xa273f0),4:(0x4e5370,0xa299d8),
 5:(0x4e67c0,0xa29780),6:(0x4db6f0,0xa28a00),7:(0x4dbf40,0xa28b88),9:(0x4dc6f0,0xa28e90)}
SOURCE={1:0xa13318,2:0xa05540,20:0xa13448,21:0xa0f6e0}
def run(c,image):
 prior=h.P;h.P=O if image=='original'else V
 try:m=h.Machine(c['precision'])
 finally:h.P=prior
 h.VISITED.clear();actor=m.alloc(0x300);m.w32(actor+0x58,c['count58']);m.u.reg_write(UC_X86_REG_EAX,c['eax'])
 vt=SOURCE[c['kind']]if c['operation']=='source38'else CLASSES[c['kind']][1]
 m.w32(actor,vt);value=m.call(O.u32(vt+c['offset']),actor)
 assert m.final_esp==h.STACK+4
 return dict(value=value if c['operation']=='source38'else bool(value&255)),dict(h.VISITED)
def main():
 rows=[];pins={};data=[]
 for pc in h.PRECISION:
  for cls in CLASSES:
   for off in [0xb0,0xb4,0x8c]:
    for count in [-1,0,3]:
     c=dict(precision=pc,kind=cls,offset=off,count58=count,eax=0xdeadbeef,operation='runtime')
     a,ap=run(c,'original');b,bp=run(c,'v14');assert a==b;pins.update(ap|bp);rows.append(dict(input=c,output=a))
  for kind in SOURCE:
   c=dict(precision=pc,kind=kind,offset=0x38,count58=0,eax=0xdeadbeef,operation='source38')
   a,ap=run(c,'original');b,bp=run(c,'v14');assert a==b;pins.update(ap|bp);rows.append(dict(input=c,output=a))
 for cls,(ctor,vt)in CLASSES.items():
  stores=[i for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(O.range(ctor,0x200),ctor)if i.mnemonic=='mov'and i.op_str.endswith(', '+hex(vt))]
  assert len(stores)>=1,(cls,hex(ctor))
  i=stores[0];assert O.range(i.address,i.size)==V.range(i.address,i.size)
  data.append(dict(kind=cls,ctor=hex(ctor),vtable=hex(vt),store_va=hex(i.address),store_bytes=O.range(i.address,i.size).hex()))
 for kind,vt in list((c,vt)for c,(_,vt)in CLASSES.items())+list(SOURCE.items()):
  offsets=[0xb0,0xb4,0x8c]if vt in [v for _,v in CLASSES.values()]else [0x38]
  for off in offsets:
   assert O.range(vt+off,4)==V.range(vt+off,4);data.append(dict(kind=kind,va=hex(vt+off),bytes=O.range(vt+off,4).hex(),target=hex(O.u32(vt+off))))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),v14_sha256=H(V.b),
  rows=rows,machines=2*len(rows),executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],intercepted_entries=[],data_pins=data,
  dependency_sha256=H((ROOT/'research/phase8-movement/harness.py').read_bytes()))
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-startup-getters.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),seams=0,sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
