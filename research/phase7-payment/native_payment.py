"""Original4E2590/4CDD90/4BEB50/4E1690 ability payment.
Only allocator/free and statistics callbacks are intercepted; vector resize,
copy, signed integer XP conversion and real stockpile subtraction execute.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
OWNER,ABILITY,LOCAL,FACTION,STATS,STOCKS,COSTS=range(0x4000000,0x4007000,0x1000)
bits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.events=[];self.visited={};self.heap=0x4010000
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x20000);self.w(ABILITY+8,LOCAL if c['local'] else 0);self.w(OWNER+0x230,FACTION if c['faction'] else 0);self.w(OWNER+0x1c4,STOCKS);self.w(OWNER+0x1c8,len(c['stocks']));self.w(LOCAL+0x68,COSTS);self.w(LOCAL+0x6c,len(c['costs']));self.w(LOCAL+0x74,c['xp_cost']);self.w(FACTION+0xc,c['experience']);self.w(FACTION+8,OWNER if c['statistics'] else 0);self.w(OWNER+0x32c,STATS)
  for i,v in enumerate(c['stocks']):self.w(STOCKS+i*4,v)
  for i,v in enumerate(c['costs']):self.w(COSTS+i*4,v)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
  if pc==0x6f7810:
   ptr=self.heap;self.heap+=max(16,(self.arg()+15)&~15);self.ret(ptr)
  elif pc==0x6f7430:self.ret()
  elif pc==0x58e870:
   self.events.append({'statistics':9,'index':self.arg(),'amount':self.arg(1)})
   if self.c['mutate'] and self.arg()==45:
    self.w(FACTION+0xc,bits(3.17));self.w(OWNER+0x32c,0)
    for i in range(len(self.c['costs'])):self.w(COSTS+i*4,bits((i+1)*1.717))
   self.ret(pop=8)
 def run(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,OWNER);self.w(self.STACK+4,ABILITY)
  regs={r:0x12990000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  self.u.emu_start(0x4e2590,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert all(self.u.reg_read(r)==v for r,v in regs.items())
  return {'experience':self.r(FACTION+0xc) if self.c['faction'] else None,'stocks':[self.r(STOCKS+i*4)for i in range(len(self.c['stocks']))],'events':self.events,'statistics':9 if self.c['statistics'] and self.r(OWNER+0x32c) else None}
def cases():
 rows=[]
 for count in [0,1,4,7]:
  for cost in [0,1,-3,16777217,2147483647]:
   for faction,statistics in [(False,False),(True,False),(True,True)]:
    for local in [False,True]:
     rows.append({'name':f'{count}_{cost}_{faction}_{statistics}_{local}','local':local,'faction':faction,'statistics':statistics,'mutate':False,'experience':bits(100.17),'xp_cost':cost,'stocks':list(map(bits,[(-1)**i*(i+1)*1.3 for i in range(count)])),'costs':list(map(bits,[(i-2)*.317 for i in range(count)]))})
 for count in [0,4]:rows.append({'name':'mutable_'+str(count),'local':True,'faction':True,'statistics':True,'mutate':True,'experience':bits(7.17),'xp_cost':2,'stocks':[bits(17.77)]*count,'costs':[bits(.3)]*count})
 return [{**copy.deepcopy(c),'precision':pc,'name':c['name']+pc}for pc in ['Single24','Double53','Extended64']for c in rows]
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-payment-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
