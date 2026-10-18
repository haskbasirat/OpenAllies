"""Original588FB0 constructor, complete empty-queue58A770 Load v0..13,
and4CFB00 required-component check through actual589090/4D1050. Transport and
allocation are host seams; serialized object reference discard and all field
gates execute. Positive queue/reference counts are explicitly outside this path.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v))
class Machine(Base):
 def __init__(self,payload):
  super().__init__(payload,28);self.component=self.alloc(0x38);self.u.mem_write(self.component,b'\xcd'*0x38)
 def hook(self,u,pc,size,arg):
  if pc==0x50b250:
   raw=P.range(pc,size);assert raw==V.range(pc,size);VISITED[pc]=raw.hex();return
  super().hook(u,pc,size,arg)
 def call(self,pc,arg=None,receiver=None):
  self.write(self.stack,self.stop)
  if arg is not None:self.write(self.stack+4,arg)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.component if receiver is None else receiver)
  regs={r:0x77bb0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  self.u.emu_start(pc,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+(8 if arg is not None else 4)
  assert all(self.u.reg_read(r)==v for r,v in regs.items())
  return self.u.reg_read(UC_X86_REG_EAX)
 def snapshot(self):
  return {'owner_matches':self.read(self.component+4)==self.object,'head':self.read(self.component+0xc),
   'queue_count':self.read(self.component+0x18),'capacity':self.read(self.component+0x1c),
   'words':{hex(o):self.read(self.component+o)for o in [0x10,0x20,0x24,0x28,0x2c,0x34]},'byte30':self.u.mem_read(self.component+0x30,1)[0]}
def main():
 rows=[]
 for version,uv,count,flags in itertools.product(range(14),[0,1],[0,-1],[0,0xffffffff]):
  payload=W(version)
  if version>=6:payload+=W(uv,*([-1]if uv else []))
  payload+=W(0x12340007)
  if version==0:payload+=W(count)
  if version>=2:
   payload+=W(count,0x7fa01234,0x80000000)
   if version>=11:payload+=W(0xffa05678)
   if version>=12:payload+=W(0x87654321)
   if version>=7:payload+=W(0x12345678)
  m=Machine(payload);m.write(m.object+0x100,flags);m.call(0x588fb0,m.object);before=m.snapshot()
  m.call(0x58a770,m.world);assert m.r.at==len(payload);after=m.snapshot()
  m.write(m.object+0x94,m.component);assert m.call(0x4cfb00,receiver=m.object)&255
  rows.append({'version':version,'payload_hex':payload.hex(),'before':before,'after':after,'owner_state_low':flags})
 p=Path(__file__).with_name('native-structure.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'requirement_vtable_pin':{'va':'0xa3fae8','bytes':P.range(0xa3fae8,4).hex()},'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n')
 print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
if __name__=='__main__':main()
