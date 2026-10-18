"""Original57F4D0 then complete57F970/59E300/4C8B40 saved load, v5..9.
Allocation, wire reads and actor notification are explicit host seams. Original
Updatable registration, version gates, mismatched turret/Ability prefix behavior
and list prepend execute. No InitCreate or PostLoad is run. Arithmetic uses finite
inputs; NaN/infinity multiplication remains outside the Rust numeric policy.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase7-performer'))
from native_constructors import Machine as Base,PERF,OWNER,original,modified,bits
from unicorn.x86_const import *
WORLD,VT,READ,MODE,IDS,ITEMS=range(0x4040000,0x4046000,0x1000)
W=lambda *v:struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v))
def wire(version,turrets,abilities,multiplier,index):
 b=W(1,3,version,2,multiplier,0x10007,bits(11),bits(-17),index,bits(.3),bits(7.1),turrets)
 for i in range(max(turrets,0)):
  b+=W(bits(i+.75),bits(i-3.125))
  if version>=7:b+=W(bits(i*.37))
  if version>=8:b+=bytes([i%3])+W(bits(i*2.25),bits(i*-1.75))
  if version>=9:b+=W(bits(i*.625))
 b+=W(-1)
 if version>=6:b+=W(6)
 b+=W(abilities)
 for i in range(max(abilities,0)):
  av=i%2;b+=W(av,i,bits(i+15.5),0x90000000+i)+bytes([i%3])
  if av:b+=W(3,0x10001,0x20002,0x10001)
 if version>=7:b+=bytes([255])
 return b
class Machine(Base):
 def __init__(self,c,payload):
  self.payload=payload;self.at=0;self.diagnostics=0;self.registrations=[];self.trace=[]
  super().__init__(c);self.u.mem_map(0x4040000,0x10000)
  self.w(WORLD+0x3c,VT);self.w(VT,READ);self.w(VT+0xc,MODE)
  self.w(0xb36534,WORLD);self.w(WORLD+0x284,IDS);self.w(IDS,64);self.w(IDS+4,ITEMS)
 def hook(self,u,pc,size,arg):
  self.trace.append(hex(pc));self.trace=self.trace[-30:]
  if pc==READ:
   n=self.arg(1);data=self.payload[self.at:self.at+n];assert len(data)==n
   self.at+=n;u.mem_write(self.arg(),data);self.ret(pop=8);return
  if pc==MODE:self.ret(0);return
  if pc==0x404f60:self.diagnostics+=1;self.ret();return
  if pc==0x858a00:raise RuntimeError(('native active-index error',self.at))
  if pc==0x50b250: # empty removal is executed, never silently skipped
   assert self.r(u.reg_read(UC_X86_REG_ECX)+4)==0
  super().hook(u,pc,size,arg)
 def action(self,p):
  out=super().action(p);ids=[];node=self.r(p+0x24);last=0
  while node:last=node;ids.append(self.r(node));node=self.r(node+4);assert len(ids)<100
  assert self.r(p+0x28)==last
  out['summoned_ids']=ids;out['word28']=0
  return out
 def state(self):
  count=self.r(PERF+0x18);ptrs=[self.r(self.r(PERF+0x14)+i*4)for i in range(count)]
  signed=lambda v:v-(1<<32)if v>=1<<31 else v
  p={'owner':1,'stage':signed(self.r(PERF+0x10)),'abilities':list(range(1,count+1)),
   'mana':self.r(PERF+0x20),'target_id':self.r(PERF+0x28),'target_position':[self.r(PERF+0x2c),self.r(PERF+0x30)],
   'active_ability':ptrs.index(self.r(PERF+0x34))+1 if self.r(PERF+0x34)else None,
   'active_index':signed(self.r(PERF+0x38)),'started_at':self.r(PERF+0x3c),'animation_length':self.r(PERF+0x40),
   'event_kind':self.r(PERF+0x44),'animation':None,'turret_index':signed(self.r(PERF+0x58))}
  turrets=[]
  for i in range(self.r(PERF+0x50)):
   t=self.r(self.r(PERF+0x4c)+i*4)
   turrets.append({'current_angle':self.r(t),'target_angle':self.r(t+4),'time':self.r(t+8),
    'pending':bool(self.u.mem_read(t+0xc,1)[0]),'target':[self.r(t+0x10+j*4)for j in range(3)]})
  return {'owned':{'state':p,'maximum_mana':self.r(PERF+0x24),'turrets':turrets,
    'turret_update_pending':bool(self.u.mem_read(PERF+0x5c,1)[0]),'word60':self.r(PERF+0x60)},
    'abilities':[self.action(a)for a in ptrs], 'diagnostics':self.diagnostics}
def main():
 rows=[];visited={}
 for pc,v,n,mode in itertools.product(['Single24','Double53','Extended64'],range(5,10),[0,1,3],range(4)):
  tc,ac=([n,n],[n+2,n+2],[0,0],[-1,-1])[mode]
  c={'kind':'performer','precision':pc,'energy':bits(25),'energy_after_notify':bits(37.25),
   'abilities':[{'main_power':bits(i*.31)}for i in range(n)],'angles':[bits(i*.7)for i in range(n)],
   'startup':[bits(17.25),bits(-4.5)],'properties':False}
  payload=wire(v,tc,ac,[bits(.37),bits(-.5),0,0x80000000][mode],n-1 if n else -1)
  m=Machine(c,payload)
  try:
   assert m.call_native(0x57f4d0,PERF,OWNER)==PERF
   before=m.state();m.call_native(0x57f970,PERF,WORLD);after=m.state();assert m.at==len(payload)
   assert m.r(ITEMS+3*4)==PERF+8
  except Exception as e:raise RuntimeError((pc,v,n,mode,hex(m.u.reg_read(UC_X86_REG_EIP)),m.at,m.trace))from e
  second=bytearray(wire(v,n,n,bits(.5),-1));second[4:8]=W(-1)
  m.payload=bytes(second);m.at=0;m.call_native(0x57f970,PERF,WORLD);assert m.at==len(second)
  m.w(OWNER+0x74,PERF);assert m.call_native(0x4cfb00,OWNER)&255
  rows.append({'input':c,'payload_hex':payload.hex(),'before':before,'after':after,'second_payload_hex':second.hex(),'second_after':m.state()})
  visited.update(m.visited)
 out={'scope':__doc__,'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),
  'requirement_vtable_pin':{'va':'0xa3df60','bytes':original.range(0xa3df60,4).hex()},'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 p=Path(__file__).with_name('native-performer-load.json');p.write_text(json.dumps(out,indent=2)+'\n')
 print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
if __name__=='__main__':main()
