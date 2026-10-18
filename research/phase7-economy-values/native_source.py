"""Original economy loader fragment4666C0..466822, cost473CB0, and enum462EC0.
Typed field getters are the explicit input boundary; original dense allocation,
resource-category filtering, stores, comparison and native field-read order run.
Enum registration callback is observed, not represented as full map insertion.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
class SourceMachine(Machine):
 def __init__(self,pc):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[pc]);self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x100000);self.heap=0x4050000;self.visited={};self.calls=[];self.nodes={};self.enums=[]
  self.u.mem_write(0x4040000,b'\xd9\x05'+struct.pack('<I',0x4040100)+b'\xc2\x04\x00');self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,n):p=self.heap;self.heap+=(max(n,16)+15)&~15;assert self.heap<0x4100000;return p
 def string(self,p):
  text=b''
  while (v:=bytes(self.u.mem_read(p,2)))!=b'\0\0':text+=v;p+=2
  return text.decode('utf-16le')
 def ret(self,value=0,pop=0):s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop)
 def hook(self,u,pc,n,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,n))
  s=u.reg_read(UC_X86_REG_ESP);cx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x6f7810:self.ret(self.alloc(self.r(s+4)))
  elif pc==0x6f7430:self.ret()
  elif pc==0x461cf0:self.enums.append({'word':self.r(s+4),'name':self.string(self.r(s+8)),'flags':self.r(s+12)});self.ret(pop=12)
  elif pc==0x5b4410:
   name=self.string(self.r(s+4));self.calls.append([self.nodes[cx]['name'],name]);self.ret(self.nodes[cx][name],8)
  elif pc==0x5b5380:
   name=self.string(self.r(s+4));self.calls.append([self.nodes[cx]['name'],name]);self.ret(self.nodes[cx][name],4)
  elif pc==0x5b5130:
   name=self.string(self.r(s+4));self.calls.append([self.nodes[cx]['name'],name]);self.w(0x4040100,self.nodes[cx][name]);u.reg_write(UC_X86_REG_EIP,0x4040000)
  elif pc==0x410c40:raise ValueError('Native loader rejected values')
 def enum(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,0x4000000);self.w(self.STACK+4,1)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x462ec0,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert[self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  return self.enums
 def load(self,c):
  self.reset(0);a=0x4000000;domain=0x4010000;ids=c['resource_ids'];self.w(0xb3a660,domain);self.w(domain+0x28,len(ids));self.w(domain+0x24,domain+0x100)
  for i,name in enumerate(ids):
   p=domain+0x200+i*0x100;sp=p+0x40;self.w(domain+0x100+i*4,p);self.w(p+8,sp);self.w(p+0x18,c['kinds'][i]);self.u.mem_write(sp,name.encode('utf-16le')+b'\0\0')
  self.nodes={0x4020000:{'name':'actor','EconomyComponent':0x4020100 if c['present'] else 0},0x4020100:{'name':'EconomyComponent','supply_required':c['supply_required'],'Cost':0x4020200,'Upkeep':0x4020300,'Production':0x4020400}}
  for offset,name in [(0x200,'Cost'),(0x300,'Upkeep'),(0x400,'Production')]:self.nodes[0x4020000+offset]={'name':name,**dict(zip(ids,c[name]))}
  self.w(a+0x174,c['initial_flags'])
  for off in [0x324,0x330,0x33c]:self.w(a+off,self.alloc(16*4));self.w(a+off+4,0);self.w(a+off+8,16)
  self.u.reg_write(UC_X86_REG_EBP,a);self.u.reg_write(UC_X86_REG_EBX,0x4020000);self.u.emu_start(0x4666c0,0x466822,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x466822 and self.u.reg_read(UC_X86_REG_ESP)==self.STACK
  return {'flags174':self.r(a+0x174),'supply_required':bool(self.u.mem_read(a+0x348,1)[0]),'vectors':{name:[self.r(self.r(a+off)+4*i)for i in range(self.r(a+off+4))]for off,name in[(0x324,'Cost'),(0x330,'Upkeep'),(0x33c,'Production')]},'read_order':self.calls}
def cases():
 for pc in ['Single24','Double53','Extended64']:
  for kinds in [[],[0],[0,1,1,1,2,1,2],[1,0,2,0,1]]:
   for present in [False,True]:
    n=len(kinds);yield {'precision':pc,'present':present,'supply_required':n%2==1,'initial_flags':0x1080,'resource_ids':[f'resource{i}'for i in range(n)],'kinds':kinds,'Cost':[bits(1.1+i)for i in range(n)],'Upkeep':[bits(.3+i)for i in range(n)],'Production':[bits(2.2*i)for i in range(n)]}
  for actor in json.loads((HERE/'original-economy-inputs.json').read_text()):
   r=actor['resources'];assert r['resource_ids']==['money','ammo','oil','unit_limit_provided','unit_limit_consumed','building_limit_provided','building_limit_consumed']
   c={'precision':pc,'source_actor':actor['id'],'present':True,'supply_required':r['supply_required']['Boolean']['value'],'initial_flags':0x1080,'resource_ids':r['resource_ids'],'kinds':[0,1,1,1,2,1,2]}
   for name in ['Cost','Upkeep','Production']:c[name]=[0x4f000123 if name=='Cost' and c['kinds'][i]!=0 else r['fields'][name+'.'+id]['Scalar']['bits']for i,id in enumerate(c['resource_ids'])]
   yield c
def main():
 visited={};m=SourceMachine('Double53');enums=m.enum();visited.update(m.visited);rows=[]
 for c in cases():
  m=SourceMachine(c['precision']);o=m.load(c);visited.update(m.visited);rows.append({'input':c,'output':o})
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 out={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'authored_input_sha256':sha((HERE/'original-economy-inputs.json').read_bytes()),'enums':enums,'cases':rows,'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
 p=HERE/'native-source.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(rows),'enums':enums,'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
