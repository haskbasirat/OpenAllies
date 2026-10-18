"""Selected actual4624B0 ActorData ctor fields and source virtual38.

The inherited ObjectData49C760 constructor is an explicit BEFORE-entry
transport seam. Existing CString/allocator/exception-frame transports apply;
this certifies selected fresh fields, not ObjectData/ActorData LoadData or
allocator failure parity. Positive source Turret/Tire values are bound by the
existing canonical typed source APIs separately, never fabricated by this ctor.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP
class ActorSource(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False)and pc==0x49c760:
   self.ctor_seams.append(dict(address=hex(pc),name='ObjectData base constructor',arguments=[self.arg(),self.arg(1)]));self.ret(8,self.ecx());return
  super().hook(u,pc,size,user)
  if getattr(self,'armed',False)and u.reg_read(UC_X86_REG_EIP)!=pc:
   key=hex(pc);self.transports[key]=self.transports.get(key,0)+1
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=ActorSource(image)
  for precision in PC:
   m.fpu(precision);m.actual={};m.record_actual=True;m.ctor_seams=[];m.transports={};m.armed=True
   data=m.alloc(0x600);m.u.mem_write(data,b'\xa5'*0x600)
   result=m.call(0x4624b0,[0x12345678,0x10203040],this=data,callee_pop=8)
   assert result==data
   values={hex(offset):m.r(data+offset)for offset in [0x1fc,0x23c,0x244]};assert all(v==0 for v in values.values())
   actor=m.call(int.from_bytes(PE0.range(0xa05540+0x38,4),'little'),this=data)&0xff
   obj=m.call(int.from_bytes(PE0.range(0xa13318+0x38,4),'little'),this=data)&0xff
   assert actor==1 and obj==0
   pins.update(m.actual);rows.append(dict(image=image,precision=precision,fields=values,actor38=actor,object38=obj,seams=m.ctor_seams,inherited_transports=m.transports))
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 slots=[]
 for vt in [0xa05540,0xa13318]:
  raw=PE0.range(vt+0x38,4);assert raw==PE1.range(vt+0x38,4);slots.append(dict(va=hex(vt+0x38),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,source_slots=slots)
 out=HERE/'native-actor-source.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
