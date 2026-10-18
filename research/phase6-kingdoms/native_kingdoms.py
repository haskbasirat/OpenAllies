"""Execute original GKingdom::Load suffix with separately proven Actor wire seam."""
from pathlib import Path
import sys,struct,json,hashlib
from collections import deque
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
sys.path.insert(0,str(ROOT/'research/phase6-actor-payloads'))
from native_teams import Machine as Base,P,V,VISITED,read_chunks
from native_actor_payload import TailMachine
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self,data,world_version):
  super().__init__(data)
  self.write(0xb36534,self.world);self.write(self.world+0x2c,world_version)
  self.write(0xb364c0,self.alloc(0x60010))
  table=self.alloc(0x10);self.write(table,65535);self.write(table+4,self.alloc(65536*4));self.write(self.world+0x284,table)
  self.write(self.object+0x1fc,0xa296a8)
  objvt=self.alloc(0x200);self.u.mem_write(objvt,P.range(0xa299d8,0x178));self.write(objvt+0x24,0x2e00050);self.write(self.object,objvt)
  for off in (0x14,0x194,0x198,0x240,0x248,0x348):self.write(self.object+off,self.empty)
  self.write(self.object+0x344,self.alloc(0x200));self.write(self.object+0x330,self.alloc(0x200))
  self.actor_seams=[];self.calls=[];self.references=[];self.trace=deque(maxlen=25)
 def actor_wire(self):
  start=self.r.at;v=self.r.u32();assert v<=2
  if v>=2:self.r.string();self.r.take(4)
  self.r.take(24);flags=self.r.u32()
  if v==1:self.r.take(4)
  ov=self.r.u32();assert ov<=5;self.r.take(4)
  if ov>=1:self.r.take(4)
  if ov>=2:self.r.take(4)
  if ov>=3:self.r.take(2)
  if ov>=4:assert self.r.u32()==0
  av=self.r.u32();self.r.take(8)
  if av>=9:self.r.take(4)
  tailstart=self.r.at;mask=struct.unpack_from('<I',self.r.b,tailstart)[0]
  tm=TailMachine(self.r.b[tailstart:]);tm.w(0x2088000,65535);tail=tm.run_tail(av,mask);self.r.take(tail['consumed'])
  self.actor_seams.append({'start':start,'tail_start':tailstart,'end':self.r.at,'actor_version':av,'default_components_mask':mask,'saved_components_mask':struct.unpack_from('<I',self.r.b,tailstart+4)[0]})
  self.write(self.object+8,flags)
 def hook(self,u,pc,size,arg):
  self.trace.append(hex(pc))
  if pc<0x400000:raise RuntimeError((hex(pc),self.r.at,self.calls[-10:],list(self.trace)))
  if pc==0x2e00050:self.write(self.arg(),self.empty);self.ret(4,value=self.arg());return
  if pc==0x4d8cf0:self.actor_wire();self.ret(4);return
  if pc==0x50b250:self.ret();return
  if pc==0x4e0f00:
   commander=self.alloc(0x40);self.write(commander,0xa2713c);definition=self.alloc(0x40);self.write(definition+0x10,self.empty);self.write(commander+4,definition);self.write(self.object+0x230,commander);self.ret(4);return
  if pc in (0x4e7f80,0x4e7e00,0x4e7ec0,0x4daaa0,0x4cdbb0,0x58db20):
   start=self.r.at;value=self.r.string();self.write(self.arg(1),0);self.references.append({'va':hex(pc),'start':start,'end':self.r.at,'value':value});self.ret(value=self.arg(0));return
  if pc==0x4e7050:self.ret(4,value=self.alloc(8));return
  if pc in (0x404f60,0x404f90,0x404ed0):self.ret();return
  if pc in (0x858ad0,0x4011d0,0x5a1040):raise RuntimeError(('native error',hex(pc),self.r.at))
  if pc in (0x4e6c20,0x58dbe0,0x58ee80,0x58f2e0,0x503b10,0x50bcd0,0x4ce0b0,0x4c8b40):self.calls.append({'va':hex(pc),'offset':self.r.at})
  super().hook(u,pc,size,arg)
 def kingdom(self):
  start=self.r.at;definition=self.r.string();category=self.r.u32();identity=self.r.u32();assert category==4
  self.write(self.stack,self.stop);self.write(self.stack+4,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object)
  try:self.u.emu_start(0x4e8550,self.stop,count=1000000)
  except Exception as e:print('crash',hex(self.u.reg_read(UC_X86_REG_EIP)),self.r.at,self.calls[-8:],self.references[-3:]);raise
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
  return {'offset':start,'end':self.r.at,'definition':definition,'serialized_id':identity,'word_188':self.read(self.object+0x188),'word_18c':self.read(self.object+0x18c),'word_190':self.read(self.object+0x190)}
def main():
 rows=[]
 for p in sorted((ROOT.parent/'reconstruction/assets/original/Data/Maps').rglob('*.rmp')):
  chunks=read_chunks(p);data=chunks['KINGDOMS'][0];world=struct.unpack_from('<I',chunks['GWORLD'][0])[0]
  m=Machine(data,world);major=m.r.u32();count=m.r.u32();at=m.r.at;entries=[]
  for slot in range(count):
   m=Machine(data,world);m.r.at=at;present=m.r.take(1)[0]
   if present:
    try:row=m.kingdom()
    except Exception:print(p,slot);raise
    entries.append({'slot':slot,'present_byte':present,**row,'actor_seams':m.actor_seams,'calls':m.calls,'references':m.references,'reads':m.reads})
   else:entries.append({'slot':slot,'present_byte':0})
   at=m.r.at
  assert at==len(data),(p,at,len(data))
  rows.append({'map':str(p),'world_version':world,'sha256':hashlib.sha256(data).hexdigest(),'major_count':major,'slot_count':count,'decoded_bytes':at,'entries':entries})
 out={'schema':1,'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'maps':rows,'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())]}
 path=Path(__file__).with_name('native-kingdoms.json');path.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'maps':len(rows),'kingdoms':sum(sum(bool(e['present_byte']) for e in r['entries']) for r in rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
