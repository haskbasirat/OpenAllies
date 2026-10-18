"""Execute original concrete Mind constructors and retain exact initialized bytes."""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
WORLD,ACTOR,ALLOC,AVT,ASTUB,EMPTY,PERFORMER,ABILITIES=0x4000000,0x4001000,0x4002000,0x4002100,0x4700000,0x4003000,0x4004000,0x4005000
sha=lambda b:hashlib.sha256(b).hexdigest()
KINDS={'base':(0x52c130,0x34),'element':(0x535ba0,0x1bc),'regiment':(0x526260,0x2c4),'aircraft':(0x51bff0,0x1d8)}
class ConstructorMachine(Machine):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=c;self.visited={};self.writes=[];self.heap=0x4400000
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x800000);self.w(0xb36534,WORLD);self.w(WORLD+0xd4,c['seed']);self.w(ALLOC,AVT);self.w(AVT+12,ASTUB);self.u.mem_write(self.STATE,bytes([c['fill']])*0x2c4)
  self.w(ACTOR+0x74,PERFORMER if c['abilities'] else 0);self.w(PERFORMER+0x14,ABILITIES);self.w(PERFORMER+0x18,c['abilities'])
  for i in range(c['abilities']):self.w(ABILITIES+i*4,0x4006000+i*0x100);self.w(0x4006000+i*0x100+0x1c,0x12345678)
  self.u.hook_add(UC_HOOK_CODE,self.hook);self.u.hook_add(UC_HOOK_MEM_WRITE,self.onwrite)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  if pc==0x856370:self.ret(ALLOC)
  elif pc==ASTUB:self.ret(EMPTY)
  elif pc==0x77da70:self.ret(pop=12)
  elif pc==0x6f7810:
   n=self.r(u.reg_read(UC_X86_REG_ESP)+4);p=self.heap;self.heap+=(n+15)&~15;self.ret(p)
 def onwrite(self,u,access,address,size,value,_):
  if self.STATE<=address<self.STATE+KINDS[self.c['kind']][1]:self.writes.append({'offset':address-self.STATE,'size':size,'value':value})
 def run(self):
  self.reset(0);self.u.mem_write(self.STATE,bytes([self.c['fill']])*0x2c4);self.writes=[];self.w(self.STACK+4,ACTOR)
  self.u.emu_start(KINDS[self.c['kind']][0],self.STOP,count=30000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  return {'initialized_hex':bytes(self.u.mem_read(self.STATE,KINDS[self.c['kind']][1])).hex(),'writes':self.writes,'rng_state':self.r(WORLD+0xd4),'ability_words':[self.r(0x4006000+i*0x100+0x1c)for i in range(self.c['abilities'])]}
def main():
 rows=[];visited={}
 for kind in KINDS:
  for p in ['Single24','Double53','Extended64']:
   for seed in [0,1,0xdeadbeef]:
    for abilities in [0,3]:
     c={'kind':kind,'precision':p,'seed':seed,'abilities':abilities,'fill':0xcc};m=ConstructorMachine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b))
 d={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'addresses':{k:hex(v)for k,v in {'this':Machine.STATE,'actor':ACTOR,'empty_string':EMPTY+16}.items()},'method':'Original concrete constructors/base/helpers execute. CString/collection allocation and seed diagnostic logging are the only behavior seams. Native RNG draw and extended jitter conversion run, including the ability jitter write loop. CC fill and observed write extents distinguish initialized fields from padding.','instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 path=HERE/'native-constructors.json';path.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
