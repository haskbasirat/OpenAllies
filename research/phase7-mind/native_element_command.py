"""Original element contextual Move538340, with state activation as explicit leaves."""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
MIND,ACTOR,COMMAND,REGISTRY,VT,UPVT,TARGET,ACCEPT,SCHEDULE=0x4000000,0x4010000,0x4020000,0x4100000,0x4200000,0x4201000,0x4030000,0x4700000,0x4700010
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()
class ElementMachine(Machine):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.events=[];self.visited={};self.accept_count=0
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x800000);self.w(0xb364c0,REGISTRY);self.w(MIND+4,ACTOR);self.w(ACTOR,VT);self.w(TARGET,VT);self.w(VT+0x16c,ACCEPT);self.w(MIND+8,UPVT);self.w(UPVT+4,SCHEDULE)
  self.w(MIND+0x7c+4,MIND);self.w(MIND+0x68+4,MIND)
  self.w(0x4040000,MIND+0x7c);self.write_fields();self.u.hook_add(UC_HOOK_CODE,self.hook)
 def write_fields(self):
  c=self.c;self.w(ACTOR+8,c['actor']['flags']);self.w(ACTOR+0x20,c['actor']['position_bits'][0]);self.w(ACTOR+0x24,c['actor']['position_bits'][1]);self.w(ACTOR+0x74,0x4050000 if c['actor']['performer'] is not None else 0)
  for offset,key in [(0xc,'flags'),(0x10,'target')]:self.w(COMMAND+offset,c['command'][key])
  for offset,values in [(0x18,c['command']['position_bits']),(0x20,c['command']['auxiliary_bits'])]:
   for i,v in enumerate(values):self.w(COMMAND+offset+4*i,v)
  self.w(COMMAND+0x28,c['command']['speed_bits']);self.w(MIND+0xc,int(c['element']['update_pending']));self.w(MIND+0x10,0x4040000 if c['active'] else 0)
  self.w(MIND+0x90,c['element']['move_destination_bits'][0]);self.w(MIND+0x94,c['element']['move_destination_bits'][1]);self.w(MIND+0x8c,c['element']['move_target'])
  for raw,target in c['registry'].items():
   raw=int(raw);self.u.mem_write(REGISTRY+4+(raw&65535)*2,struct.pack('<H',raw>>16));self.w(REGISTRY+0x20004+(raw&65535)*4,TARGET if target is not None else 0)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop)
 def args(self,n):s=self.u.reg_read(UC_X86_REG_ESP);return[self.r(s+4+i*4)for i in range(n)]
 def event(self,name,args):
  self.events.append([name,*args]);mut=self.c['mutations']
  if mut and mut[0]['event']==name:
   m=mut.pop(0)
   for section in ['actor','command','element']:
    self.c[section].update(m.get(section,{}))
   self.write_fields()
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  if pc==0x57e280:self.event('indirect',[5]);self.ret(int(self.c['indirect']))
  elif pc==ACCEPT:
   self.event('accept',[2,1]);values=self.c['accept'];v=values[min(self.accept_count,len(values)-1)];self.accept_count+=1;self.ret(int(v),4)
  elif pc==SCHEDULE:self.event('schedule',[1,*self.args(2)]);self.ret(pop=8)
  elif pc==0x52a810:self.event('sleep',[3,*self.args(1)]);self.ret(int(self.c['sleep_result']),4)
  elif pc==0x52df60:self.event('move',[2,*self.args(7)]);self.ret(int(self.c['move_result']),28)
  elif pc==0x5351c0:self.event('kill',[4,self.args(2)[0],2]);self.ret(int(self.c['kill_result']),8)
  elif pc==0x538120:self.event('bombard',[5,self.args(2)[0],2]);self.ret(int(self.c['bombard_result']),8)
  elif pc==0x532930:
   a=self.args(6);self.event('follow',[2,a[0],2,*a[2:]]);self.ret(int(self.c['follow_result']),24)
 def run(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,MIND);self.w(self.STACK+4,COMMAND);self.u.emu_start(0x538340,self.STOP,count=20000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert not self.c['mutations']
  return{'accepted':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'events':self.events}
def cases():
 base={'name':'base','actor':{'flags':0,'position_bits':[bits(10),bits(10)],'performer':5},'command':{'flags':28,'target':0x10002,'position_bits':[bits(20),bits(20)],'auxiliary_bits':[bits(1),bits(-2)],'speed_bits':bits(3)},'element':{'actor':1,'move_state':2,'sleep_state':3,'kill_state':4,'bombard_state':5,'move_destination_bits':[bits(20),bits(20)],'move_target':0x10002,'update_pending':False},'active':False,'registry':{'65538':2},'indirect':False,'accept':[True],'sleep_result':True,'move_result':True,'kill_result':True,'bombard_result':True,'follow_result':True,'mutations':[]}
 specs=[]
 def add(name,fn):c=copy.deepcopy(base);c['name']=name;fn(c);specs.append(c)
 add('ground',lambda c:None)
 for flags in [1,1<<29,(1<<29)|1,0x400]:add(f'actorflags{flags}',lambda c,v=flags:c['actor'].update(flags=v))
 for flags in [0,4,12,20,28]:add(f'groundflags{flags}',lambda c,v=flags:c['command'].update(flags=v))
 for x,y in [(10,10),(10.099,10),(10.1,10),(10,10.1),(10.0707107,10.0707107),(20,20)]:add(f'point{x}_{y}',lambda c,x=x,y=y:c['command'].update(position_bits=[bits(x),bits(y)]))
 for pending in [False,True]:add(f'active_ground{pending}',lambda c,v=pending:(c.update(active=True),c['element'].update(update_pending=v)))
 add('active_different_ground',lambda c:(c.update(active=True),c['element'].update(move_destination_bits=[bits(21),bits(20)])))
 for indirect in [False,True]:
  for accept in [[False],[True],[False,True]]:
   for result in [False,True]:add(f'target_{indirect}_{accept}_{result}',lambda c,i=indirect,a=accept,r=result:(c['command'].update(flags=29),c.update(indirect=i,accept=a,kill_result=r,bombard_result=r)))
 for value in [None,5]:add(f'performer{value}',lambda c,v=value:(c['command'].update(flags=29),c['actor'].update(performer=v)))
 for pending in [False,True]:add(f'active_target{pending}',lambda c,v=pending:(c['command'].update(flags=29),c.update(active=True),c['element'].update(update_pending=v)))
 add('target_missing',lambda c:(c['command'].update(flags=29),c.update(registry={'65538':None})))
 add('target_stale',lambda c:(c['command'].update(flags=29,target=0x20002)))
 add('follow_fails',lambda c:(c['command'].update(flags=29),c.update(accept=[False],follow_result=False)))
 add('sleep_fails',lambda c:(c['command'].update(position_bits=[bits(10),bits(10)]),c.update(sleep_result=False)))
 add('move_fails',lambda c:c.update(move_result=False))
 add('accept_mutates_parameters',lambda c:(c['command'].update(flags=29),c.update(kill_result=False,mutations=[{'event':'accept','command':{'auxiliary_bits':[bits(90),bits(91)],'speed_bits':bits(99)}}])))
 for p in ['Single24','Double53','Extended64']:
  for c in specs:yield {**copy.deepcopy(c),'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  m=ElementMachine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,b in visited.items():assert original.range(a,len(b))==b==modified.range(a,len(b))
 doc={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'scope':'Original538340 includes original52A240 near-distance,4515F0 ordered equality,52A000 state-head query and530F10 generation lookup. Performer indirect-fire, target acceptance and typed state activations are explicit supplied leaves. Native field rereads and auxiliary/speed caches execute.','instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 path=HERE/'native-element-command.json';path.write_text(json.dumps(doc,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
