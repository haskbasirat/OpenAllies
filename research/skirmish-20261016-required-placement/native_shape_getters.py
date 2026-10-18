"""Actual runtime class2/3/4/6/7/9 vtable0C/B0/B4 bodies.
Constructor vtable identities are statically pinned; allocation/constructors do
not execute here. No virtual getter seam is installed. Other classes excluded.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
ORIGINAL=h.P
def main():
 rows=[];tables=[]
 cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for cls,table,constructor in [(2,0xa2a2a0,0x4ea5e0),(3,0xa273f0,0x4d6bb0),
      (4,0xa299d8,0x4e5370),(6,0xa28a00,0x4db6f0),(7,0xa28b88,0x4dbf40),(9,0xa28e90,0x4dc6f0)]:
  for slot in [0xc,0xb0,0xb4]:
   raw=ORIGINAL.range(table+slot,4);assert raw==h.V.range(table+slot,4)
   address=int.from_bytes(raw,'little')
   stores=[i for i in cs.disasm(ORIGINAL.range(constructor,0x100),constructor)
           if i.mnemonic=='mov' and i.op_str.endswith(', '+hex(table))]
   assert stores
   store=stores[0];assert store.bytes==h.V.range(store.address,len(store.bytes))
   tables.append(dict(runtime_class=cls,vtable=hex(table),constructor=hex(constructor),
      constructor_store=dict(address=hex(store.address),bytes=store.bytes.hex(),asm=store.mnemonic+' '+store.op_str),
      slot=slot,target=hex(address),bytes=raw.hex()))
   for image in ['original','v14']:
    h.P=ORIGINAL if image=='original' else h.V
    for pc in h.PRECISION:
     m=h.Machine(pc);actor=m.alloc(0x100);definition=m.alloc(0x100)
     m.w32(actor,table,definition);m.w32(actor+0x3c,h.f2b(4))
     m.w32(definition+0x38,3,h.f2b(7),h.f2b(-3),h.f2b(-4),h.f2b(3),h.f2b(4))
     result=m.call(address,actor)
     output={'shape_definition_offset':result-definition} if slot==0xc else {'eax':result,'al':result&255}
     assert output==({'shape_definition_offset':0x38} if slot==0xc else {'eax':1 if slot==0xb0 or cls in [3,4] else 0,'al':1 if slot==0xb0 or cls in [3,4] else 0})
     rows.append(dict(runtime_class=cls,image=image,precision=pc,slot=slot,output=output))
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
          producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),vtables=tables,cases=rows,
          executed_instructions=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
 path=HERE/'native-shape-getters.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),instructions=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
