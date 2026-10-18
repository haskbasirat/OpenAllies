"""Original simple Mind state callbacks and real Element scheduling wrapper.
World state/effects and queue admission are explicit leaves; activation, default
reset, stack manipulation, command release and timer unlink run original code.
"""
from pathlib import Path
import copy,hashlib,json,sys,struct
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
M,A,P,C,W,QVT,QADD,HEAP=0x4000000,0x4010000,0x4020000,0x4030000,0x4040000,0x4050000,0x4060000,0x4100000
CMD=0x4070000
metadata=json.loads((ROOT/'research/phase7-mind/native-metadata.json').read_text())
layouts=metadata['state_layouts']['element']
sha=lambda b:hashlib.sha256(b).hexdigest()
class WorldMachine(Machine):
 def __init__(self,c):
  super().__init__();self.c=c;self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x200000);self.heap=HEAP;self.events=[];self.visited={}
  self.w(M,0xa35810);self.w(M+4,A);self.w(M+8,0xa357e8)
  for v in metadata['metadata']:
   ptr=int(v['metadata_va'],16);self.w(ptr+4,v['word4']);self.u.mem_write(ptr+8,bytes([v['discard_on_suspend']]))
  for v in layouts:
   ptr=M+v['offset'];self.w(ptr,int(v['vtable_va'],16));self.w(ptr+4,M);self.w(ptr+8,int(v['metadata_va'],16));self.w(ptr+12,17)
  self.u.mem_write(M+0x144,b'\x00');self.w(A+0x104,2 if c['state33'] else 0);self.w(A+0x74,P if c['performer'] else 0);self.w(P+4,A);self.w(P+0x10,c['stage']);self.w(A+0x6c,C if c['mover'] else 0);self.w(C+4,A)
  self.w(A+0xe4,{'none':0,'performer':P,'mover':C}[c['active']]);self.w(C+0xc,0x4999999 if c['mover_pending'] else 0)
  self.w(0xb36534,W);self.w(W+0x208,QVT);self.w(QVT+12,QADD)
  self.set_stack(c['stack']);self.w(CMD,0xa28450);self.w(CMD+4,2);self.w(M+0x20,CMD if c['current'] else 0)
  self.u.mem_write(0xb3d2d4,bytes([c.get('guard',False)]))
  for data in c['pending']:
   n=self.alloc(24);old=self.r(M+0xc);self.w(n+12,M+0xc);self.w(n+16,old);self.w(n+20,data)
   if old:self.w(old+12,n+16)
   self.w(M+0xc,n)
  if c['kind']=='regiment':self.w(M+8,0xa33578)
  if c['kind']=='aircraft':self.w(M,0xa32710)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,n):p=self.heap;self.heap+=(n+15)&~15;return p
 def set_stack(self,values):
  head=tail=0
  for offset in reversed(values):
   n=self.alloc(8);self.w(n,0 if offset is None else M+offset);self.w(n+4,head);head=n
   if not tail:tail=n
  self.w(M+0x10,head);self.w(M+0x14,tail)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc==0x6f7810:self.ret(self.alloc(self.r(s+4)))
  elif pc==0x6f7430:self.ret()
  elif pc==0x4cfc80:
   self.events.append(['stance',cx-A]);
   if self.c['reenter']:u.reg_write(UC_X86_REG_ECX,M);u.reg_write(UC_X86_REG_EIP,0x52d7c0)
   else:self.ret()
  elif pc==0x4d64f0:
   self.events.append(['state',cx-A,self.r(s+4),self.r(s+8)]);self.ret(pop=8)
  elif pc==0x4d1350:self.events.append(['rebuild',cx-A]);self.ret()
  elif pc==0x44dea0:self.ret(0)
  elif pc==0x4f1a70:self.events.append(['cancel',self.r(self.r(s+4)+20)]);self.ret(pop=4)
  elif pc==QADD:
   assert cx==W+0x208;assert self.r(s+4)==M+8
   self.events.append(['schedule',4,self.r(s+8),self.r(s+12)]);n=self.alloc(24);self.w(n+20,self.r(s+12));self.ret(n,12)
 def run(self):
  c=self.c;self.reset(0);regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  operation=c['operation'];ptr=M;pop=0;entry=None
  if operation in ['can','defers','enter','leave']:
   ptr=M+c['offset'];v=self.r(ptr);entry=self.r(v+{'can':4,'defers':8,'enter':24,'leave':28}[operation])
  elif operation=='sync':entry=0x52f350
  elif operation=='changed':entry=0x516ab0 if c['kind']=='aircraft' else 0x52d710
  elif operation=='init':entry=0x529ec0
  elif operation=='create':entry=0x438ac0;pop=4;self.w(self.STACK+4,0x12345678)
  elif operation=='default':entry=0x52d7c0
  elif operation=='schedule':entry=self.r(self.r(M+8)+4);ptr=M+8;pop=8;self.w(self.STACK+4,c['delay']);self.w(self.STACK+8,77)
  else:raise ValueError(operation)
  self.u.reg_write(UC_X86_REG_ECX,ptr);self.u.emu_start(entry,self.STOP,count=40000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP,hex(self.u.reg_read(UC_X86_REG_EIP))
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4+pop
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  stack=[];n=self.r(M+0x10)
  while n:stack.append(None if self.r(n)==0 else self.r(n)-M);n=self.r(n+4);assert len(stack)<30
  return {'answer':bool(self.u.reg_read(UC_X86_REG_EAX)&255) if operation in ['can','defers','init','create'] else None,'stack':stack,'statuses':{str(v['offset']):self.r(M+v['offset']+12) for v in layouts},'engage_byte':self.u.mem_read(M+0x144,1)[0],'current':bool(self.r(M+0x20)),'references':self.r(CMD+4),'guard':bool(self.u.mem_read(0xb3d2d4,1)[0]),'events':self.events}
def cases():
 base={'name':'base','operation':'can','offset':0x68,'kind':'element','state33':False,'performer':False,'stage':0,'mover':False,'active':'none','mover_pending':False,'stack':[],'current':False,'guard':False,'reenter':False,'pending':[],'delay':0}
 out=[]
 def add(name,**changes):c=copy.deepcopy(base);c.update(name=name,**changes);out.append(c)
 for v in layouts:
  off=v['offset']
  for stack in [[],[None],[0x34],[0x68],[0x7c]]:add(f'can_{off}_{stack}',offset=off,stack=stack)
  add(f'defers_{off}',operation='defers',offset=off)
 for off in [0x34,0x68,0xf8,0x12c,0x178]:
  for state33 in [False,True]:add(f'enter_{off}_{state33}',operation='enter',offset=off,state33=state33)
 for off in [0x34,0x68]:add(f'leave_{off}',operation='leave',offset=off)
 for off in [0x34,0x48,0x68,0x7c,0xf8,0x12c,0x164,0x178]:
  for state33 in [False,True]:add(f'sync_{off}_{state33}',operation='sync',stack=[off],state33=state33)
 for kind in ['element','aircraft']:add(f'changed_{kind}',operation='changed',kind=kind,stack=[0x68])
 for operation in ['init','default']:
  for state33 in [False,True]:
   for current in [False,True]:add(f'{operation}_{state33}_{current}',operation=operation,state33=state33,current=current)
 add('init_existing',operation='init',stack=[0x68])
 add('default_reentrant_stance',operation='default',reenter=True)
 add('default_denied_priority',operation='default',stack=[0x34],current=True)
 for kind in ['element','regiment','aircraft']:add(f'create_{kind}',operation='create',kind=kind)
 for kind in ['element','regiment']:
  for delay in [0,0x80000000,0x3f800000,0xbf800000]:
   for pending in [[],[9,10]]:
    for active in ['none','mover','performer']:
     add(f'schedule_{kind}_{delay}_{pending}_{active}',operation='schedule',kind=kind,delay=delay,pending=pending,active=active,performer=True,mover=True)
 for stage in [-1,0,1,2,3,4,5]:add(f'performer_stage_{stage}',operation='schedule',performer=True,active='performer',stage=stage,pending=[9],delay=0x3f800000)
 add('mover_has_timer',operation='schedule',mover=True,active='mover',mover_pending=True,delay=0x3f800000)
 return out
def main():
 rows=[];visited={}
 for c in cases():
  m=WorldMachine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,raw in visited.items():assert raw==original.range(a,len(raw))==modified.range(a,len(raw))
 doc={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':raw.hex()}for a,raw in sorted(visited.items())],'cases':rows}
 p=HERE/'native-world.json';p.write_text(json.dumps(doc,indent=2));print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
