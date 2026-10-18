"""Original Admin284 ownership boundaries only, not packet/gameplay services.
751B60 executes its whole empty-peer path; 74FD10 executes original history
selection/cursor/284 cleanup with explicit packet-reference/dispatcher/sync
seams. Dispatcher seam writes actual284 then either returns or stops, so error
rows certify retained native boundary prefix, not the omitted dispatcher body.
74B3A0 is the actual sole-slot setter. Original/v14, all three x87 PCs.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
P=h.P;V=h.V;H=lambda b:hashlib.sha256(b).hexdigest()
class Stop(Exception):pass
class Machine(h.Machine):
 def __init__(self,pc):
  super().__init__(pc);self.admin=self.alloc(0x400);self.peer=self.alloc(0x80);self.events=[]
 def empty(self,c):
  self.w32(self.admin+0x284,c['current']);self.w32(self.admin+0x208,c['previous']);self.w32(self.peer+0xc,0,0)
  self.call(0x751b60,self.admin,[self.peer]);assert self.final_esp==h.STACK+8 and self.fpu_top()==0
  return {'current':self.r32(self.admin+0x284),'previous':self.r32(self.admin+0x208)}
 def setter(self,c):
  self.w32(self.admin+0x284,c['current']);self.call(0x74b3a0,self.admin,[c['value']]);assert self.final_esp==h.STACK+8 and self.fpu_top()==0
  return {'current':self.r32(self.admin+0x284)}
 def client(self,c):
  a=self.admin;table=self.alloc(4);self.w32(table,0);self.w32(a+0x20c,0);self.w32(a+0x214,0,1,1,table);self.w32(a+0x248,0xffffffff,0xffffffff,c['resume']);self.w32(a+0x284,c['current']);self.w32(0xb3655c,self.alloc(0x100))
  def refcopy(m):m.w32(m.u.reg_read(UC_X86_REG_ECX),0);m.ret(4)
  def dispatch(m):
   self.events.append({'entry284':m.r32(a+0x284)});m.w32(a+0x284,c['selected'])
   if c['stop']:raise Stop('required_dispatch_child')
   m.ret(16)
  self.seams[0x766f60]=refcopy;self.seams[0x74af40]=refcopy;self.seams[0x766f90]=lambda m:m.ret();self.seams[0x74d710]=dispatch;self.seams[0x77da70]=lambda m:m.ret(12)
  status='complete'
  try:self.call(0x74fd10,a,[]);assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  except Stop as e:status=str(e)
  return {'status':status,'current':self.r32(a+0x284),'resume_player':self.r32(a+0x250),'next':self.r32(a+0x20c),'cursor':self.r32(a+0x248,2),'calls':self.events}
def main():
 cases=[('setter',{'current':17,'value':v})for v in [0,1,0xffffffff]]+[('empty',{'current':v,'previous':99})for v in [0,1,17,0xffffffff]]+[('client',dict(resume=r,current=99,selected=s,stop=stop))for r,s,stop in itertools.product([0,17],[0,2,0xffffffff],[False,True])]
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for kind,c in cases:rows.append({'image':label,'precision':pc,'kind':kind,'input':c,'output':getattr(Machine(pc),kind)(c)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 out={'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-admin-ordering-owner.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
