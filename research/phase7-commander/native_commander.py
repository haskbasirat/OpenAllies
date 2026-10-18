"""Original4E0F00/4CE1D0/4CE310 plus saved4CE0B0/4C8B40.
Only allocation/free, stream/version transport and mismatch diagnostic hooks.
All original action constructors, list linking and destruction execute.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
KINGDOM,WORLD,STREAMVT,READ=range(0x4000000,0x4004000,0x1000)
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
def words(*v):return struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v))
def wire(s):
 out=words(s['version'],s['experience'],s['serialized_count'])
 for a in s['abilities']:
  out+=words(a['version'],a['use_count'],a['last_use'],a['word14'])+bytes([a['enabled_byte']])
  if a['version']>=1:out+=words(a['summoned']['serialized_count'],*a['summoned']['values'])
 return out
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=c;self.visited={};self.heap=0x4040000;self.buffer=b'';self.at=0;self.diagnostics=0;self.freed=[];self.locals={};self.defs={}
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x100000);self.w(WORLD+0x3c,STREAMVT);self.w(STREAMVT,READ)
  for d in c['definitions']:
   dp=0x4004000+d['id']*0x100;arr=0x4020000+d['id']*0x100;self.defs[d['id']]=dp;self.w(dp+0x24,arr);self.w(dp+0x28,len(d['abilities']))
   for i,a in enumerate(d['abilities']):
    lp=0x4024000+a['local_definition']*0x80;ap=0x4010000+a['definition']*0x80;self.locals[lp]=a['local_definition'];self.w(arr+i*4,lp);self.w(lp,ap);self.w(ap+0x64,a['main_power'])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def take(self,n):
  b=self.buffer[self.at:self.at+n];assert len(b)==n;self.at+=n;return b
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
  if pc==0x6f7810:
   ptr=self.heap;self.heap+=max(16,(self.arg()+15)&~15);self.ret(ptr)
  elif pc==0x6f7430:self.freed.append(self.arg());self.ret()
  elif pc==READ:self.u.mem_write(self.arg(),self.take(self.arg(1)));self.ret(pop=8)
  elif pc==0x8589b0:
   v=struct.unpack('<I',self.take(4))[0];assert v<=self.arg(1);self.w(u.reg_read(UC_X86_REG_ECX),v);self.ret(u.reg_read(UC_X86_REG_ECX),8)
  elif pc==0x404ed0:self.diagnostics+=1;self.ret()
 def action(self,p):
  ids=[];node=self.r(p+0x24);tail=0;previous=0
  while node:
   ids.append(self.r(node));previous=node;tail=node;node=self.r(node+4);assert len(ids)<1000
  assert self.r(p+0x28)==tail # physical tail is Vec ownership in Rust, not a copied address
  return{'definition':(self.r(p+4)-0x4010000)//0x80,'local_definition':self.locals.get(self.r(p+8)),'use_count':self.r(p+0xc),'last_use':self.r(p+0x10),'word14':self.r(p+0x14),'enabled':bool(self.u.mem_read(p+0x18,1)[0]),'recharge_bypass':bool(self.u.mem_read(p+0x19,1)[0]),'fuzzy_range':self.r(p+0x1c),'power':self.r(p+0x20),'summoned_ids':ids,'word28':0}
 def state(self):
  p=self.r(KINGDOM+0x230)
  if not p:return None
  return{'definition':next(k for k,v in self.defs.items()if v==self.r(p+4)),'kingdom':1,'experience':self.r(p+0xc),'abilities':[self.action(self.r(self.r(p+0x10)+i*4))for i in range(self.r(p+0x14))]}
 def run(self):
  out=[]
  for step in self.c['steps']:
   self.reset(0);self.w(self.STACK+4,self.defs.get(step.get('definition'),0));entry=0x4e0f00;receiver=KINGDOM
   if step['kind']=='load':entry=0x4ce0b0;receiver=self.r(KINGDOM+0x230);self.buffer=wire(step['saved']);self.at=0;self.w(self.STACK+4,WORLD)
   regs={r:0x88770000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
   for r,v in regs.items():self.u.reg_write(r,v)
   self.u.reg_write(UC_X86_REG_ECX,receiver)
   try:self.u.emu_start(entry,self.STOP,count=1000000)
   except Exception as e:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP)))from e
   assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
   assert all(self.u.reg_read(r)==v for r,v in regs.items())
   if step['kind']=='load':assert self.at==len(self.buffer)
   out.append({'state':self.state(),'diagnostics':self.diagnostics})
  return out
def saved(count,version=1,use_count=3,ids=None):
 return {'version':0,'experience':bits(9.17),'serialized_count':count&0xffffffff,'abilities':[{'version':version,'use_count':use_count,'last_use':bits(13.71),'word14':0x80000017+i,'enabled_byte':[0,1,255][i%3],'summoned':{'serialized_count':len(ids or []),'values':ids or []}if version else None}for i in range(max(count,0))]}
def cases():
 out=[]
 for count in [0,1,2,4,8]:
  defs=[{'id':10,'abilities':[{'definition':20+i,'local_definition':30+i,'main_power':bits((i-2)*.317)}for i in range(count)]},{'id':11,'abilities':[{'definition':40,'local_definition':50,'main_power':bits(3.717)}]}]
  for loaded in [0,count,count+1,-1]:
   for v in [0,1]:
    s=saved(loaded,v,0 if v else 3,[0x10001,0x20002,0x10001]);second=saved(count,1,7,[9,13]);third=saved(count,0,0)
    steps=[{'kind':'replace','definition':10},{'kind':'load','saved':s},{'kind':'load','saved':second},{'kind':'load','saved':third},{'kind':'replace','definition':11},{'kind':'load','saved':saved(1,1,0,[99])},{'kind':'replace','definition':None}]
    out.append({'name':f'{count}_{loaded}_{v}','definitions':defs,'steps':steps})
 return out
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);got=m.run()
  except Exception as e:raise RuntimeError(c['name'])from e
  rows.append({'input':c,'output':got});visited.update(m.visited)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-commander-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows),'transitions':sum(len(r['output'])for r in rows)}))
if __name__=='__main__':main()
