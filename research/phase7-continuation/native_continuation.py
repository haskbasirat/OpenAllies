"""Native outer movement continuation with explicit mutable world leaves."""
from pathlib import Path
import sys,json,copy,hashlib,struct
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase5-navigation'))
import native_oracle as base
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
TARGET=0x2350000;REGISTRY=0x2360000;FLOAT=base.STUB+0x400;SHP=base.STUB+0x500
class Machine(base.Machine):
 def __init__(self,pe,c,visited):
  super().__init__(pe,c);self.c=c;self.obstacle=None
  table=(ROOT/'research/phase4-trig/lookup-tables.bin').read_bytes();self.u.mem_write(0x2201000,table);self.put(0xb39a48,0,4096,0x2201000,0x40c90fdb,1);self.put(0xb39a60,0,4096,0x2205000,0x40c90fdb,1);self.put(0xb39b08,0xb39a48);self.put(0xb39b10,0xb39a60)
  self.put(base.ACTOR,base.VT);self.put(TARGET,base.VT);self.put(base.VT+0x18,base.STUB+0x80);self.put(base.VT+0xc,base.STUB+0x90);self.put(base.VT+0x30,base.STUB+0xb0)
  self.put(base.ACTOR+0x2c,*c['facing']);self.put(base.MOVER+0x38,*c['direction']);self.put(base.MOVER+0x30,c['radius']);self.put(TARGET+8,c['target_flags']);self.put(TARGET+0x20,*c['target_position'])
  self.put(base.WORLD+0xe8,c['step']);self.put(base.STATE+0x3c,*c['collision']);self.put(base.STATE+0x4c,*c['collision_vector'])
  self.put(0xb364c0,REGISTRY);self.u.mem_write(REGISTRY+4+14,struct.pack('<H',1 if c['target_valid'] else 2));self.put(REGISTRY+0x20004+7*4,TARGET)
  self.u.hook_add(UC_HOOK_CODE,lambda u,at,size,_:visited.setdefault(at,size) if at<0x1000000 else None)
 def float_return(self,bits):
  self.put(SHP,bits);self.u.mem_write(FLOAT,b'\xd9\x05'+struct.pack('<I',SHP)+b'\xc3');self.u.reg_write(UC_X86_REG_EIP,FLOAT)
 def ident(self,p):
  assert p in (base.ACTOR,TARGET);return 1 if p==base.ACTOR else 2
 def ev(self,name,bits=()):
  self.events.append({'name':name,'bits':list(bits)})
  m=self.c.get('changes',{}).get(name,{})
  if 'phase' in m:self.put(base.STATE+0x2c,m['phase'])
  if 'head_kind' in m:self.put(self.ints(base.PATH+4)[0]+0x10,m['head_kind'])
  if m.get('clear_path'):self.put(base.PATH+4,0,0)
  if 'facing' in m:self.put(base.ACTOR+0x2c,*m['facing'])
 def hook(self,u,at,size,_):
  ecx=u.reg_read(UC_X86_REG_ECX)
  if at==0x5300a0:self.ev('ActionGo',self.args());self.ret(4,int(self.c['action_success']));return
  if at==0x4cede0:self.ev('Moving',[self.ident(ecx)]);self.ret(value=int(self.c['moving']));return
  if at==0x532b50 and self.c['op'] not in ('back','fullback'):self.ev('BackToPath');self.ret(value=int(self.c['back_result']));return
  if at==0x532c64 and self.c['op']=='back':
   stack=u.reg_read(UC_X86_REG_ESP);self.recovery=dict(head={'words':self.ints(self.ints(base.PATH+4)[0],8)},facing=self.ints(stack+0x14,2),requested_direction=self.ints(stack+0x34,2),position=self.ints(stack+0x3c,2),perpendicular_squared_bits=self.ints(stack+0x24)[0]);self.ev('TurnRecovery');u.reg_write(UC_X86_REG_EIP,0x532c57 if self.c['back_result'] else 0x533128);return
  if at==base.STUB+0x80:self.ev('MovementRadius',[self.ident(ecx)]);self.float_return(self.c['movement_radius']);return
  if at==0x531170:
   args=self.args(4);self.ev('ForwardProbe',args[:3]);self.put(args[3],*self.c['probe_result']);self.ret(16,int(self.c['probe_result'][0]!=0));return
  if at==0x536840:self.ev('Avoid',[self.ident(self.args()[0])]);self.ret(4,int(self.c['avoid']));return
  if at==0x4d67f0:self.ev('Refresh',[self.ident(ecx)]);self.ret();return
  if at==base.STUB+0xb0:self.ev('Overlap',[self.ident(ecx),self.ident(self.args()[0])]);self.ret(4,int(self.c['overlap']));return
  if at==0x404f60:
   if self.c['op']=='fullback':
    args=self.args(10);dist=base.bits(struct.unpack('<d',struct.pack('<2I',*args[6:8]))[0]);offset=base.bits(struct.unpack('<d',struct.pack('<2I',*args[8:10]))[0]);self.ev('RecoveryDiagnostic',[1,dist,offset])
   else:self.ev('Diagnostic',[1,2])
   self.ret();return
  if at==base.STUB+0x90:self.ret(value=SHP+self.ident(ecx)*16);return
  if at==0x444420:
   id=(ecx-SHP)//16;assert id in (1,2);self.ev('BoundsRadius',[id]);self.float_return(self.c['own_radius' if id==1 else 'target_radius']);return
  if at==0x536ad0:self.ev('StopDistance',[self.ident(self.args()[0]),self.args(2)[1]]);self.ret(8);return
  if at==0x530f40:assert self.args()==[1];self.ev('ResultPath');self.ret(4,self.c['path_result']);return
  if at==0x52e130:self.ev('Wait');self.ret();return
  if at==0x4cf210:self.obstacle={'Actor':self.ident(ecx)};self.ret(4,self.args()[0]);return
  if at==0x52f3e0:
   args=self.args(3);self.obstacle={'Tile':args[1:]};self.ret(12,args[0]);return
  if at==0x534a40:
   args=self.args(4);assert args[3]==0;self.ev('CollisionPlan',args[1:3]);self.ret(16);return
  if at==0x554350:
   self.descriptor={};self.u.mem_write(ecx,bytes(96));self.ret(8,ecx);return
  if at in (0x552250,0x5522b0):self.descriptor['endpoint' if at==0x552250 else 'direction']=self.args(2);self.ret(8);return
  if at==0x553ba0:
   self.ev('RecoveryPlacement',self.descriptor['endpoint']+self.descriptor['direction']);out=self.args()[0];self.put(out,int(self.c.get('placement_blocked',False)),0,0,0);self.ret(8,out);return
  if at==0x5312c0:
   args=self.args(6);self.ev('RecoveryProbe',args[:5]);self.put(args[5],*self.c['probe_result']);self.ret(24,int(self.c.get('recovery_probe_blocked',False)));return
  if at==base.STUB+0x30:self.ev('Schedule',self.args(2));self.ret(8);return
  if at==base.STUB+0x10:self.ev('NodeReached');self.ret();return
  super().hook(u,at,size,_)
 def execute(self):
  c=self.c;op=c['op'];entry={'continue':0x5379c0,'forward':0x537150,'collision':0x536f40,'save':0x52fd50,'back':0x532b50,'fullback':0x532b50}[op]
  args=[c['distance']] if op=='forward' else [base.VALUES,*c['facing']] if op=='save' else []
  if op=='save':self.put(base.VALUES,*c['collision'])
  stack=0x3008000;self.put(stack,base.STOP,*args);self.u.reg_write(UC_X86_REG_ESP,stack);self.u.reg_write(UC_X86_REG_ECX,base.STATE)
  regs=[(UC_X86_REG_EBX,0x11223344),(UC_X86_REG_ESI,0x33445566),(UC_X86_REG_EDI,0x55667788),(UC_X86_REG_EBP,0x778899aa)]
  for r,v in regs:self.u.reg_write(r,v)
  self.u.emu_start(entry,base.STOP,count=500000)
  assert self.u.reg_read(UC_X86_REG_EIP)==base.STOP and self.u.reg_read(UC_X86_REG_ESP)==stack+4+4*len(args)
  assert all(self.u.reg_read(r)==v for r,v in regs)
  result=dict(events=self.events,phase=self.ints(base.STATE+0x2c)[0],offset=self.ints(base.STATE+0x54)[0],collision=self.ints(base.STATE+0x3c,4),collision_vector=self.ints(base.STATE+0x4c,2),path=self.read_path(base.PATH),facing=self.ints(base.ACTOR+0x2c,2),obstacle=self.obstacle)
  if op in ('back','fullback'):result.update(recovery=getattr(self,'recovery',None),success=bool(self.u.reg_read(UC_X86_REG_EAX)&255))
  return result

def cases():
 initial=copy.deepcopy(base.cases()[0]);initial['state']['phase']=1
 initial.update(op='continue',facing=[base.bits(1),0],direction=[base.bits(1),0],radius=base.bits(2),target_flags=0,target_position=[base.bits(9),base.bits(12)],step=base.bits(.05),collision=[0,0,base.bits(3),base.bits(4)],collision_vector=[base.bits(1),0],target_valid=True,action_success=True,moving=False,back_result=False,movement_radius=base.bits(2),probe_result=[0]*4,avoid=False,overlap=False,own_radius=base.bits(1.25),target_radius=base.bits(2.5),path_result=0,distance=base.bits(3),changes={})
 rows=[]
 def add(name,**values):
  c=copy.deepcopy(initial);c['name']=name
  for k,v in values.items():
   if k=='state':c[k].update(v)
   elif k=='actor':c[k].update(v)
   else:c[k]=v
  rows.append(c)
 add('stopped');add('stopped_kind0',state={'path':base.path([base.node(kind=0)])});add('action_clears_path',changes={'ActionGo':{'clear_path':True}});add('action_changes_kind',changes={'ActionGo':{'head_kind':0}})
 add('different_direction',moving=True,direction=[0,base.bits(1)]);add('different_rejected',moving=True,direction=[0,base.bits(1)],action_success=False);add('different_phase_mutation',moving=True,direction=[0,base.bits(1)],changes={'ActionGo':{'phase':0}})
 add('back_success',moving=True,back_result=True);add('back_phase0',moving=True,state={'phase':0});add('back_changes_phase',moving=True,changes={'BackToPath':{'phase':0}});add('moving_forward',moving=True)
 for name,extra in [('mismatched',dict(facing=[0,base.bits(1)])),('special',dict(actor={'flags':1<<18})),('done',dict(state={'path':base.path([base.node(-2,0)])})),('clear',{}),('clamp_projection',dict(distance=base.bits(100))),('clamp_step',dict(step=base.bits(50))),('probe_changes_facing',dict(probe_result=[1,0,base.bits(4),base.bits(6)],changes={'ForwardProbe':{'facing':[base.bits(3),base.bits(4)]}}))]:add('forward_'+name,op='forward',**extra)
 for name,extra in [('invalid',dict(target_valid=False)),('avoided',dict(avoid=True)),('overlap',dict(overlap=True)),('distance',{}),('step_min',dict(step=base.bits(500))),('flagged',dict(target_flags=1<<29)),('wait',dict(path_result=1,target_flags=1<<29)),('retry',dict(path_result=2,target_flags=1<<29))]:add('collision_'+name,op='collision',collision=[2,0x10007,0,0],**extra)
 add('collision_tile',op='collision',collision=[1,0,base.bits(4),base.bits(5)]);add('continue_phase2',state={'phase':2},collision=[1,0,base.bits(4),base.bits(5)],path_result=1)
 for direction in [[0,0],[0x80000000,0],[base.bits(3),base.bits(4)],[base.bits(.00001),0],[base.bits(.0001),0],[base.bits(.00010001),0]]:add('save_'+str(direction),op='save',facing=direction)
 for name,y,kind in [('on',0,1),('near',.9999,1),('edge',1,1),('drift',1.0001,1),('kindzero',2,0),('fractional',2.456,1)]:add('back_'+name,op='back',state={'path':base.path([base.node(10,y,1,0,kind)])},back_result=name=='drift')
 add('back_facing_mismatch',op='back',facing=[0,base.bits(1)])
 add('back_multiplezero',op='back',state={'path':base.path([base.node(10,2,1,0,0),base.node(20,2,1,0,0),base.node(30,2,1,0,1)])})
 add('back_nonunit',op='back',direction=[base.bits(.731),base.bits(.482)],state={'path':base.path([base.node(10,2)])})
 for name,xy,radius in [('right',[10,2],2),('left',[10,-2],2),('near_turn',[10,1.1],20),('far_behind',[-10,2],2),('no_radius',[10,2],0),('short',[.5,1.1],2),('far',[100,10],4)]:
  add('full_'+name,op='fullback',state={'path':base.path([base.node(*xy)])},radius=base.bits(radius))
 add('full_blocked',op='fullback',placement_blocked=True)
 add('full_probe_blocked',op='fullback',recovery_probe_blocked=True)
 add('full_action_rejected',op='fullback',action_success=False)
 import random,math
 r=random.Random(0x532b50)
 for n in range(20):
  angle=r.uniform(-3,3);dx,dy=math.cos(angle),math.sin(angle)
  add('full_fractional'+str(n),op='fullback',facing=[base.bits(dx),base.bits(dy)],direction=[base.bits(dx),base.bits(dy)],state={'path':base.path([base.node(r.uniform(-50,50),r.uniform(-50,50),dx,dy)])},radius=base.bits(r.uniform(.25,50)))
 return [{**copy.deepcopy(c),'precision':pc,'name':c['name']+'_'+pc} for pc in ('Single24','Double53','Extended64') for c in rows]

def main():
 pe=base.Pe(ROOT.parent/'AA.exe',base.ORIGINAL_SHA);v14=base.Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',base.ACCEPTED_SHA);visited={};rows=[]
 for c in cases():
  try:r=Machine(pe,c,visited).execute()
  except Exception as e:raise RuntimeError(c['name']) from e
  rows.append(dict(input=c,output=r))
 for at,size in visited.items():assert pe.at(at,size)==v14.at(at,size)
 out=dict(schema=1,passed=True,original_sha256=base.ORIGINAL_SHA,v14_sha256=base.ACCEPTED_SHA,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),cases=rows,visited=[dict(address=hex(a),size=n,bytes=pe.at(a,n).hex()) for a,n in sorted(visited.items())])
 (OUT/'native-continuation.json').write_text(json.dumps(out,indent=2)+'\n');print(len(rows))
if __name__=='__main__':main()
