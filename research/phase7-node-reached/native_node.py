"""Original530AE0/535E00/52CDB0 with explicit lifecycle/query callbacks.
Visited windows include intercepted seam entries, not proof of seam bodies.
"""
from pathlib import Path
import sys,json,copy,hashlib,struct,random
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase5-navigation'))
import native_oracle as base
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
REG=0x2360000;TARGET=0x2370000;LEADMIND=0x2380000
class Machine(base.Machine):
 def __init__(self,pe,c,visited):
  super().__init__(pe,c)
  self.put(base.VT+0x44,0x535e00);self.put(base.VT+0x158,base.STUB+0x80)
  self.put(base.LEADER+0x70,LEADMIND);self.put(LEADMIND,base.VT);self.put(base.VT+0x14c,base.STUB+0x90)
  self.put(base.UAI+0x54,c['kind54']);self.put(base.WORLD+0xe8,c['step']);self.put(base.STATE+0x3c,*c['collision']);self.put(base.STATE+0x4c,*c['collision_vector'])
  self.put(0xb364c0,REG);self.u.mem_write(REG+4+7*2,struct.pack('<H',1 if c['valid_target'] else 2));self.put(REG+0x20004+7*4,TARGET)
  self.u.hook_add(UC_HOOK_CODE,lambda u,at,size,_:visited.setdefault(at,size) if at<0x1000000 else None)
 def ev(self,name,bits=()):
  self.events.append(dict(name=name,bits=list(bits)))
  m=self.c.get('changes',{}).get(name,{})
  if m.get('clear_path'):self.put(base.PATH+4,0,0)
  if 'position' in m:self.put(base.ACTOR+0x20,*m['position'])
  if 'speed' in m:self.put(base.UAI+0xa0,m['speed'])
  if 'target' in m:self.put(base.UAI+0x8c,m['target'])
  if 'kind54' in m:self.put(base.UAI+0x54,m['kind54'])
  if 'collision' in m:self.put(base.STATE+0x3c,*m['collision'])
 def hook(self,u,at,size,_):
  ecx=u.reg_read(UC_X86_REG_ECX)
  if at==0x52fd90:self.ev('Offset');self.put(base.STATE+0x54,self.c['offset_result']);self.ret();return
  if at==base.STUB+0x90:assert self.args()==[base.ACTOR];self.ev('LeaderNode',[1]);self.ret(4);return
  if at==0x554350:
   assert self.args(2)==[base.ACTOR,0];self.query=ecx;self.u.mem_write(ecx,bytes(64));self.put(ecx+0x38,self.c['query_flags'],self.c['query_mask']);self.ev('BeginQuery',[1]);self.ret(8,ecx);return
  if at==0x552250:self.ev('QueryPosition',self.args(2));self.position=self.args(2);self.ret(8);return
  if at==0x5522b0:self.ev('QueryFacing',self.args(2));self.facing=self.args(2);self.ret(8);return
  if at==0x553ba0:
   out,q=self.args(2);assert q==self.query;ignore=self.ints(q+0x20)[0];assert ignore in (0,TARGET)
   self.ev('FirstHit',self.position+self.facing+[2 if ignore else 0]+self.ints(q+0x38,2));self.put(out,*self.c['hit']);self.ret(8,out);return
  if at==base.STUB+0x80:self.ev('CollisionResponse');self.ret(value=int(self.c['response']));return
  if at==0x4cfc80:assert ecx==base.ACTOR;self.ev('DefaultStance',[1]);self.ret();return
  if at==base.STUB+0x30:assert self.args(2)==[0,0];self.ev('Schedule',[0,0]);self.ret(8);return
  super().hook(u,at,size,_)
 def execute(self):
  sp=0x3008000;self.put(sp,base.STOP);self.u.reg_write(UC_X86_REG_ESP,sp);self.u.reg_write(UC_X86_REG_ECX,base.UAI)
  regs=[(UC_X86_REG_EBX,0x12345678),(UC_X86_REG_ESI,0x23456789),(UC_X86_REG_EDI,0x34567890),(UC_X86_REG_EBP,0x456789ab)]
  for r,v in regs:self.u.reg_write(r,v)
  self.u.emu_start(0x530ae0,base.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==base.STOP and self.u.reg_read(UC_X86_REG_ESP)==sp+4
  assert all(self.u.reg_read(r)==v for r,v in regs)
  return dict(events=self.events,path=self.read_path(base.PATH),offset=self.ints(base.STATE+0x54)[0],collision=self.ints(base.STATE+0x3c,4),collision_vector=self.ints(base.STATE+0x4c,2),position=self.ints(base.ACTOR+0x20,2),speed=self.ints(base.UAI+0xa0)[0],kind54=self.ints(base.UAI+0x54)[0],target=self.ints(base.UAI+0x8c)[0],success=bool(self.u.reg_read(UC_X86_REG_EAX)&255),freed=self.freed)
def cases():
 c=copy.deepcopy(base.cases()[0]);c.update(step=base.bits(.05),kind54=0,offset_result=base.bits(.2),collision=[0]*4,collision_vector=[0]*2,valid_target=True,hit=[0]*4,response=False,query_flags=0x1234,query_mask=0x5678,changes={})
 c['state']['path']=base.path([base.node(0,0,kind=1),base.node(2,0,kind=1)]);rows=[]
 def add(name,**values):
  r=copy.deepcopy(c);r['name']=name
  for k,v in values.items():
   if k in ('state','actor'):r[k].update(v)
   else:r[k]=v
  rows.append(r)
 add('empty',state={'path':base.path([])});add('single',state={'path':base.path([base.node(kind=2)])});add('normal');add('leader',actor={'has_leader':True})
 for kind in [0,1,2,99]:
  for end in [-2,0,.01,.15,.150001,1,2]:add(f'kind{kind}_end{end}',state={'path':base.path([base.node(kind=kind),base.node(end,0)])})
 add('behind_chain',state={'path':base.path([base.node(kind=0),base.node(-2,0),base.node(-1,0),base.node(2,0),base.node(3,0,kind=2)])})
 for name,values in [('hit',{'hit':[2,0x10007,3,4],'response':True}),('hitfalse',{'hit':[1,0,3,4]}),('kind2',{'kind54':2}),('invalid',{'valid_target':False}),('zero_speed',{'state':{'requested_speed_bits':0}}),('negative_speed',{'state':{'requested_speed_bits':base.bits(-3)}})]:
  d={'state':{'path':base.path([base.node(kind=0),base.node(3,0)])}};d.update(values)
  if 'path' not in d['state']:d['state']['path']=base.path([base.node(kind=0),base.node(3,0)])
  add(name,**d)
 for name,changes in [('leader_clear',{'LeaderNode':{'clear_path':True}}),('offset_clear',{'Offset':{'clear_path':True}}),('offset_pose',{'Offset':{'position':[base.bits(5),0]}}),('begin_pose',{'BeginQuery':{'position':[base.bits(1),base.bits(3)]}}),('position_pose',{'QueryPosition':{'position':[base.bits(-1),base.bits(2)]}}),('facing_target',{'QueryFacing':{'target':0,'kind54':2}}),('hit_pose',{'FirstHit':{'position':[base.bits(20),base.bits(30)]}}),('collision_clear',{'CollisionResponse':{'clear_path':True}})]:
  add(name,changes=changes,actor={'has_leader':True},state={'path':base.path([base.node(kind=1 if name=='leader_clear' else 0),base.node(10,2)])},hit=[1,0,5,6] if name=='collision_clear' else [0]*4)
 r=random.Random(0x530ae0)
 for i in range(45):
  x,y=r.uniform(-10,10),r.uniform(-10,10);dx,dy=r.uniform(-1,1),r.uniform(-1,1)
  add('fractional'+str(i),actor={'position_bits':[base.bits(x),base.bits(y)],'has_leader':i%3==0},state={'path':base.path([base.node(kind=i%3),base.node(x+dx*10,y+dy*10,dx,dy,kind=(i+1)%3),base.node(x+20,y+15)]),'requested_speed_bits':base.bits(r.uniform(.01,10))},step=base.bits(r.uniform(.001,1)),kind54=i%4,hit=[i%3,0x10007,0,0],response=i%2==0)
 return [{**copy.deepcopy(c),'precision':pc,'name':c['name']+'_'+pc} for pc in ('Single24','Double53','Extended64') for c in rows]
def main():
 pe=base.Pe(ROOT.parent/'AA.exe',base.ORIGINAL_SHA);v14=base.Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',base.ACCEPTED_SHA);visited={};rows=[]
 for c in cases():
  try:r=Machine(pe,c,visited).execute()
  except Exception as e:raise RuntimeError(c['name']) from e
  rows.append(dict(input=c,output=r))
 for a,size in visited.items():assert pe.at(a,size)==v14.at(a,size)
 ranges=[(0x530ae0,735),(0x535e00,7),(0x52cdb0,80),(0x52fd50,60),(0x409230,103)]
 for a,n in ranges:assert pe.at(a,n)==v14.at(a,n)
 doc=dict(passed=True,original_sha256=base.ORIGINAL_SHA,v14_sha256=base.ACCEPTED_SHA,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),cases=rows,ranges=[dict(address=hex(a),size=n,bytes=pe.at(a,n).hex()) for a,n in ranges],visited=[dict(address=hex(a),size=n,bytes=pe.at(a,n).hex()) for a,n in sorted(visited.items())])
 (OUT/'native-node.json').write_text(json.dumps(doc,indent=2)+'\n');print(len(rows))
if __name__=='__main__':main()

