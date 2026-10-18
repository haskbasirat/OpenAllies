"""Actual AVar30 ReplacementColorDefault registration, Point3 ctor/setter.

402B1C..402B7E's actual name/index/Point3/default/flags argument production
executes;4B3470 is an intercepted registration transport, not manager proof.
401880 and4B32B0->4B2B60->4B29B0 execute over flags0 current Point3 storage.
CString/TLS/allocation helpers are inherited named transport seams. Source TGI
token parsing/profile overrides are separate, not certified by these rows.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ESI,UC_X86_REG_EDI
class Complete(Exception):pass
class Orientation(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'orientation_ready',False) and pc==0x4b3470:
   p=self.arg(1);self.registration=dict(index=self.arg(),name=self.rs(p+0x1c),kind=self.r(p+0x24),flags=self.r(p+0x28),value=[self.r(p+i)for i in [4,8,12]])
   raise Complete()
  super().hook(u,pc,size,user)
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Orientation(image)
  for precision in PC:
   m.fpu(precision);m.actual={};m.record_actual=True;m.orientation_ready=True
   m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ESI,m.alloc(0x40));m.u.reg_write(UC_X86_REG_EDI,0xffffffff)
   try:m.u.emu_start(0x402b1c,0x402b7e,count=100000)
   except Complete:pass
   else:raise AssertionError('registration not reached')
   rows.append(dict(image=image,precision=precision,registration=m.registration));pins.update(m.actual);m.orientation_ready=False
   for value in [[0,0,0],[0x3f800000,0xbf800000,0x80000000],[0x7fc12345,0x7f800000,0xff800000],[0x3eaaaaab,0x41200000,0xc1200000]]:
    m.actual={};field=m.alloc(0x2c);m.u.mem_write(field,b'\xa5'*0x2c);name=m.alloc(4);m.cs(name,'ReplacementColorDefault')
    m.call(0x401880,[m.r(name),0x3f800000,0x3f800000,0x3f800000,0],this=field,callee_pop=20)
    before=[m.r(field+i)for i in [4,8,12]];m.call(0x4b32b0,value,this=field,callee_pop=12)
    rows.append(dict(image=image,precision=precision,input=value,before=before,after=[m.r(field+i)for i in [4,8,12]],kind=m.r(field+0x24),flags=m.r(field+0x28)))
    pins.update(m.actual)
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 assert all(r['registration']==dict(index=30,name=list(map(ord,'ReplacementColorDefault')),kind=5,flags=0,value=[0x3f800000]*3)for r in rows if 'registration'in r)
 assert all(r['after']==r['input']for r in rows if 'input'in r)
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-replacement-color.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
