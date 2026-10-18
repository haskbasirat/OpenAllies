"""Whole original GThing constructor/identity and random-direction startup.

No process or source-image writes. String-pool allocation and free are seams;
original CString assignment, constructor, normalization and GTID methods run.
"""
from pathlib import Path
import hashlib,json,struct,sys
from collections import deque
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-actors'))
from trace_registry import Machine as RegistryMachine,P,V,H
from unicorn.x86_const import *

def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]

class Machine(RegistryMachine):
 def __init__(self,pc):
  self.visited={};self.events=[];self.heap=0x2200000;self.recent=deque(maxlen=40)
  super().__init__({})
  self.u.mem_map(0x2200000,0x60000)
  self.obj=self.object(1,False);self.definition=0x2240000;self.record=0x2241000;self.world=0x2242000
  self.pool=0x2244000;self.poolvt=0x2244100;self.w(self.pool,self.poolvt)
  self.w(self.poolvt+12,0x301f300);self.w(self.poolvt+4,0x301f310);self.w(self.poolvt+16,0x301f320)
  self.w(0xb36534,self.world);self.w(0xb3655c,0x2245000);self.w(0xb36560,0x2245000);self.w(self.world+0x130,0x2246000)
  self.u.reg_write(UC_X86_REG_FPCW,pc);self.pc=pc
 def alloc(self,n):p=self.heap;self.heap+=(n+15)&~15;assert self.heap<0x2240000;return p
 def string(self,units):
  p=self.alloc(18+len(units)*2)
  for off,val in [(0,self.pool),(4,len(units)),(8,len(units)),(12,1)]:self.w(p+off,val)
  self.u.mem_write(p+16,struct.pack('<%dH'%(len(units)+1),*units,0));return p+16
 def hook(self,u,pc,size,user):
  self.visited[pc]=size
  self.recent.append(pc)
  if pc<0x1000:raise RuntimeError('bad PC '+hex(pc)+' trace '+str([hex(x)for x in self.recent]))
  if pc==0x6f7810:self.ret(self.alloc(self.arg()))
  elif pc==0x856370:self.ret(self.pool)
  elif pc==0x301f300:self.ret(self.string([])-16)
  elif pc==0x301f310:self.ret(pop=4)
  elif pc==0x301f320:self.ret(self.pool)
  elif pc==0x50c270:
   self.events.append({'kind':'facing_notification','position_bits':[self.r(self.obj+0x20+4*i) for i in range(3)],'facing_bits':[self.r(self.obj+0x2c+4*i) for i in range(3)]})
  else:super().hook(u,pc,size,user)
 def invoke(self,pc,this,args):
  self.error=None;self.w(self.stack,self.stop)
  for i,v in enumerate(args):self.w(self.stack+4+4*i,v)
  for reg,val in [(UC_X86_REG_ESP,self.stack),(UC_X86_REG_ECX,this),(UC_X86_REG_EBX,0x11223344),(UC_X86_REG_ESI,0x22334455),(UC_X86_REG_EDI,0x33445566),(UC_X86_REG_EBP,0x44556677)]:self.u.reg_write(reg,val)
  self.u.emu_start(pc,self.stop,count=4000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+len(args)*4
  for reg,val in [(UC_X86_REG_EBX,0x11223344),(UC_X86_REG_ESI,0x22334455),(UC_X86_REG_EDI,0x33445566),(UC_X86_REG_EBP,0x44556677)]:assert self.u.reg_read(reg)==val
  return self.u.reg_read(UC_X86_REG_EAX)
 def table(self):
  self.invoke(0x408ed0,0xb39a90,[2048]);p=self.r(0xb39a98)
  return [list(struct.unpack('<2I',self.u.mem_read(p+8*i,8)))for i in range(2048)]
 def use_table(self,values):
  # Reuse bytes produced by the separately executed original startup. The
  # GThing constructor only reads this table's count and value pointer.
  p=self.alloc(len(values)*8);self.u.mem_write(p,b''.join(struct.pack('<2I',*xy)for xy in values))
  self.w(0xb39af8,0xb39a90);self.w(0xb39a94,len(values));self.w(0xb39a98,p)
 def constructor(self,case):
  self.events=[]
  for off,val in [(4,self.definition),(0x1c,self.string(case['name_utf16'])),(0x24,case['saved_id'])]:self.w(self.record+off,val)
  for i,v in enumerate(case['position_bits']):self.w(self.record+8+4*i,v)
  for i,v in enumerate(case['direction_bits']):self.w(self.record+0x14+4*i,v)
  self.w(self.definition+0x1c,case['definition_flags']);self.w(self.world+0xe0,case['world_begin_bits']);self.w(self.world+0xd4,case['rng_state'])
  self.invoke(0x4ef2a0,self.obj,[self.record])
  p=self.r(self.obj+0x18);length=self.r(p-12)
  ctor={'flags':self.r(self.obj+8),'word_c':self.r(self.obj+12),'word_10':self.r(self.obj+16),'id':self.r(self.obj+20),'name_utf16':list(struct.unpack('<%dH'%length,self.u.mem_read(p,2*length))),'created_at_bits':self.r(self.obj+28),'position_bits':[self.r(self.obj+32+4*i) for i in range(3)],'facing_bits':[self.r(self.obj+44+4*i) for i in range(3)]}
  updated_direction=[self.r(self.record+0x14+4*i) for i in range(2)]
  if case.get('full_registry'):self.w(self.manager+0x60004,32768)
  initialized=bool(self.invoke(0x4ee460,self.obj,[self.record])&255)
  return {'input':case,'constructor':ctor,'record_direction_after':updated_direction,'rng_after':self.r(self.world+0xd4),'events':self.events,'initialized':initialized,'registered_id':self.r(self.obj+20),'registry':self.snapshot()}

def main():
 tables=[];cases=[];visited={}
 direction_cases=[[bits(x),bits(y)]for x,y in [(1,0),(0,-1),(0,0),(2,0),(0.6,0.8),(1.00004,0),(1.00009,0),(1.00011,0),(0.99996,0),(0.99991,0),(0.99989,0),(1e-30,1e-30),(-0.0,1.0)]]
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Machine(cw);table=m.table();tables.append({'precision':name,'values':table});visited.update(m.visited)
  for flags in [0,1<<12,1<<17,(1<<12)|(1<<17),0xffffffff,0x240000ff]:
   for direction in direction_cases:
    case={'precision':name,'startup_precision':name,'definition_flags':flags,'position_bits':[bits(10.25),bits(-8.5),0x80000000],'direction_bits':direction,'name_utf16':[65,0,0xd800,90],'world_begin_bits':bits(17.125),'rng_state':0x12345678,'saved_id':0}
    m=Machine(cw);m.use_table(table);cases.append(m.constructor(case));visited.update(m.visited)
  for rng in [0,1,0x12345678,0x8e4b12a5,0xffffffff]:
   for flags in [1<<15,(1<<15)|(1<<12),(1<<15)|(1<<17)]:
    case={'precision':name,'startup_precision':name,'definition_flags':flags,'position_bits':[0,0,0],'direction_bits':[bits(3),bits(4)],'name_utf16':[],'world_begin_bits':0,'rng_state':rng,'saved_id':0}
    m=Machine(cw);m.use_table(table);cases.append(m.constructor(case));visited.update(m.visited)
 # Saved-ID and ordinary capacity path use the complete4EE460 wrapper.
 for saved,full in [(0x12340005,False),(0xffffffff,False),(0,True)]:
  case={'precision':'Extended64','startup_precision':'Extended64','definition_flags':0,'position_bits':[0,0,0],'direction_bits':[0,bits(-1)],'name_utf16':[115,97,118,101,100],'world_begin_bits':bits(2),'rng_state':0x12345678,'saved_id':saved,'full_registry':full}
  m=Machine(0x37f);m.use_table(tables[2]['values']);cases.append(m.constructor(case));visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 windows=[]
 for a,z in spans:
  b=P.range(a,z-a);assert b==V.range(a,z-a),hex(a);windows.append({'start':hex(a),'end':hex(z),'sha256':H(b)})
 binary=b''.join(struct.pack('<2I',*xy)for table in tables for xy in table['values'])
 (HERE/'direction-tables.bin').write_bytes(binary)
 out={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'registry_machine_sha256':H((ROOT/'research/phase4-actors/trace_registry.py').read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'direction_table_sha256':H(binary),'tables':tables,'cases':cases,'native_windows':windows,'seams':['String-pool allocate/free and general successful allocation are fixture leaves; actual CString assignment/refcount functions execute.','Debug logger is present with disabled native state; no clock or RNG branch is replaced.','Full4EF2A0 ctor including4EE9F0 normalizer,2048 vector startup and4EE460 identity wrapper execute.','Ctor flags cannot contain spatial bit26 and visual link+C beginsnull, so unrelated spatial/visual object lifecycles are not exercised.','Full-registry case supplies original count32768 directly; it is not32768 constructor executions.','No world publication, complete derived actor constructor, ownership, save parsing or component default interpretation is claimed.']}
 p=HERE/'creation-native-oracle.json';p.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(cases),'table_sha256':H(binary),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
