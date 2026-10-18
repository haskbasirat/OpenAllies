"""Original Performer lifecycle57F290/57ED20/57F040/57E8C0/57E360.
Ability eligibility/use, animator, scheduler and actor virtual operations are
explicit required fixture callbacks. No live-world combat success is implied.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
PERF,OWNER,TARGET,ABILITY,USEDEF,DEF,ANIM,OLDANIM,IDLE,VT,REG,WORLD,TURRET,TARRAY,ARRAY,POINT=range(0x4000000,0x4010000,0x1000)
ACTIVATE,FINISH=0x4020000,0x4020010
bits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
fields={'stage':0x10,'mana':0x20,'target_id':0x28,'active_index':0x38,'started_at':0x3c,'animation_length':0x40,'event_kind':0x44,'turret_index':0x58}
handles={1:OWNER,2:TARGET,3:ABILITY,4:ABILITY+0x100,5:ANIM,6:OLDANIM,7:IDLE}
ids={v:k for k,v in handles.items()}
def signed(v):return v-(1<<32) if v>=1<<31 else v
class Machine(Base):
 def __init__(self,c):
  self.c=copy.deepcopy(c);super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x40000);self.events=[];self.calls=0;self.executed={}
  self.w(0xb36534,WORLD);self.w(WORLD+0xe0,c['now']);self.w(0xb364c0,REG)
  self.w(PERF,VT);self.w(VT+0x38,ACTIVATE);self.w(PERF+4,OWNER);self.w(OWNER,VT+0x400)
  self.w(OWNER+0x14,0x10001);self.w(OWNER+0x68,OWNER+0x400 if c['animator'] else 0)
  self.w(OWNER+0x6c,OWNER+0x500 if c['mover'] else 0);self.w(OWNER+0x70,OWNER+0x600)
  self.w(OWNER+0x600,VT+0x800);self.w(VT+0x800+0x138,FINISH)
  self.w(OWNER+0xe4,PERF if c['active'] else 0)
  self.put(OWNER+0x20,*c['owner_position']);self.put(TARGET+0x20,*c['target_position']);self.w(TARGET+0x14,0x10002)
  self.u.mem_write(REG+4+2*2,struct.pack('<H',1));self.w(REG+0x20004+2*4,TARGET if c['target_live'] else 0)
  s=c['state'];self.w(PERF+0x14,ARRAY);self.w(PERF+0x18,len(s['abilities']))
  for i,a in enumerate(s['abilities']):self.w(ARRAY+i*4,handles[a])
  for key,offset in fields.items():self.w(PERF+offset,s[key])
  self.put(PERF+0x2c,*s['target_position']);self.w(PERF+0x34,handles.get(s['active_ability'],0));self.w(PERF+0x48,handles.get(s['animation'],0))
  self.w(PERF+0x4c,TARRAY);self.w(TARRAY,TURRET);self.u.mem_write(TURRET+0xc,bytes([1]))
  for a in [ABILITY,ABILITY+0x100]:
   self.w(a+4,DEF);self.w(a+8,USEDEF);self.u.mem_write(a+0x18,bytes([c['enabled']]))
  self.w(USEDEF+4,c['reload']);self.w(USEDEF+0x54,c['turret_index'] if c['uses_turret'] else 0xffffffff)
  for a,key in [(ANIM,'animation'),(OLDANIM,'old_animation')]:
   self.w(a+0x10,c[key]['length']);self.w(a+0x14,c[key]['use_fraction'])
   ptr=ANIM+0x400 if a==ANIM else OLDANIM+0x400
   self.w(a+0x50,ptr if c[key]['cues'] else 0)
   for i,time in enumerate(c[key]['cues']):self.put(ptr+i*16,time,0,0,ptr+(i+1)*16 if i+1<len(c[key]['cues']) else 0)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*words):self.u.mem_write(a,struct.pack('<'+'I'*len(words),*(w&0xffffffff for w in words)))
 def arg(self,n=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+n*4)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def ctx(self,p):
  w=struct.unpack('<12I',self.u.mem_read(p,48))
  return {'event_kind':w[0],'phase':w[1],'source':ids.get(w[2]),'allegiance':ids.get(w[3]),'source_position':list(w[4:7]),'target':ids.get(w[7]),'target_position':list(w[8:11]),'word2c':w[11]}
 def event(self,name,**args):self.events.append({'name':name,**args})
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.executed[pc]=raw.hex()
  if pc==0x4c8f20:
   result=self.c['rejections'][min(self.calls,len(self.c['rejections'])-1)];self.calls+=1
   self.event('can_use',ability=ids[u.reg_read(UC_X86_REG_ECX)],context=self.ctx(self.arg()),check_range=bool(self.arg(1)),result=result);self.ret(result,8)
  elif pc==ACTIVATE:
   self.event('activate',result=self.c['activate']);self.ret(int(self.c['activate']))
  elif pc==0x57dd90:
   p=self.arg(2);self.event('track',event_kind=self.arg(),target=ids.get(self.arg(1)),point=[self.r(p),self.r(p+4)]);self.ret(pop=12)
  elif pc==0x529c30:self.ret(DEF+0x400)
  elif pc==0x45dc50:self.ret(int(self.c['turret_wait']),4)
  elif pc==0x4eebe0:
   p=self.arg();self.event('face',direction=[self.r(p),self.r(p+4)]);self.ret(pop=4)
  elif pc==0x4c8500:self.event('fx',ability=ids[u.reg_read(UC_X86_REG_ECX)],event=self.arg(),context=self.ctx(self.arg(1)));self.ret(pop=8)
  elif pc==0x4d0750:self.ret(0)
  elif pc==0x4c8170:
   self.event('select',ability=ids[u.reg_read(UC_X86_REG_ECX)],variant=self.arg());self.ret(handles.get(self.c['selected'],0),4)
  elif pc==0x4be590:self.ret(int(self.c['keep_previous']),4)
  elif pc==0x4be610:self.ret(int(self.c['belongs']),8)
  elif pc==0x563770:self.event('play',animation=ids[self.arg()]);self.ret(pop=8)
  elif pc==0x563640:self.event('animator_begin');self.ret()
  elif pc==0x563620:self.event('animator_use');self.ret()
  elif pc==0x563aa0:self.event('cue',cue=signed(self.arg()));self.ret(pop=4)
  elif pc==0x4c81b0:self.ret(handles.get(self.c['idle'],0),4)
  elif pc==0x4db0a0:self.event('idle',animation=ids[self.arg()],duration=self.arg(1));self.ret(pop=8)
  elif pc==0x4d03d0:self.ret(int(self.c['paused']))
  elif pc==0x57e030:
   self.event('use',ability=ids[self.arg()],context=self.ctx(self.arg(1)));self.ret(pop=8)
  elif pc==FINISH:self.event('finish',stage=signed(self.r(PERF+0x10)));self.ret()
  elif pc in [0x50acc0,0x50ad20]:self.event('after' if pc==0x50acc0 else 'at',time=self.arg(),event=signed(self.arg(1)));self.ret(pop=8)
  elif pc==0x50ac00:self.event('immediate',event=signed(self.arg()));self.ret(pop=4)
  elif pc==0x50b1c0:self.event('cancel',event=signed(self.arg()),periodic=bool(self.arg(1)));self.ret(pop=8)
  elif pc==0x404f60:self.ret()
 def state(self):
  s={k:self.r(PERF+o) for k,o in fields.items()}
  for key in ['stage','active_index','turret_index']:s[key]=signed(s[key])
  s.update(owner=1,abilities=self.c['state']['abilities'],target_position=[self.r(PERF+0x2c),self.r(PERF+0x30)],active_ability=ids.get(self.r(PERF+0x34)),animation=ids.get(self.r(PERF+0x48)))
  return s
 def run(self):
  steps=[]
  for op in self.c['operations']:
   self.reset(0)
   regs={r:0x11aa0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
   for r,v in regs.items():self.u.reg_write(r,v)
   if op['kind']=='start':
    self.u.reg_write(UC_X86_REG_ECX,PERF);r=self.c['request']
    if r['point'] is not None:self.put(POINT,*r['point'])
    self.put(self.STACK+4,handles[r['ability']],r['event_kind'],handles.get(r['target'],0),POINT if r['point'] is not None else 0)
    entry,pop=0x57f290,20
   elif op['kind']=='begin':self.u.reg_write(UC_X86_REG_ECX,PERF);entry,pop=0x57ed20,4
   else:self.u.reg_write(UC_X86_REG_ECX,PERF+8);self.w(self.STACK+4,op['event']);entry,pop=0x57f040,8
   try:self.u.emu_start(entry,self.STOP,count=100000)
   except Exception as exc:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP))) from exc
   assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+pop
   for r,v in regs.items():assert self.u.reg_read(r)==v
   steps.append({'return':bool(self.u.reg_read(UC_X86_REG_EAX)&255) if op['kind']=='start' else None,'state':self.state(),'events':copy.deepcopy(self.events),'turret_pending':bool(self.u.mem_read(TURRET+0xc,1)[0])})
  return steps

def base(name):
 return {'name':name,'precision':'Double53','now':bits(12.25),'enabled':True,'active':True,'activate':True,'target_live':True,
  'animator':True,'mover':True,'paused':False,'uses_turret':False,'turret_wait':False,'turret_index':0,
  'owner_position':list(map(bits,[1,2,3])),'target_position':list(map(bits,[8,12,4])),'rejections':[0],
  'reload':bits(.75),'selected':5,'keep_previous':False,'belongs':False,'idle':7,
  'animation':{'length':bits(2.3),'use_fraction':bits(.4),'cues':[bits(.1),bits(1.2)]},
  'old_animation':{'length':bits(4),'use_fraction':bits(.7),'cues':[]},
  'state':{'owner':1,'stage':4,'abilities':[4,3],'mana':bits(30),'target_id':0x10002,'target_position':[bits(100),bits(200)],'active_ability':3,'active_index':-1,'started_at':0,'animation_length':0,'event_kind':1,'animation':None,'turret_index':-1},
  'request':{'ability':3,'event_kind':1,'target':2,'point':None},
  'operations':[{'kind':'start'},{'kind':'event','event':6},{'kind':'event','event':1},{'kind':'event','event':2},{'kind':'event','event':3}]}
def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  rows.append(c)
 add('whole_cycle')
 add('disabled',lambda c:(c.update(enabled=False),c.update(operations=[{'kind':'start'}])))
 for rejected in [1,2,3,4,5]:add('rejected_'+str(rejected),lambda c,r=rejected:(c.update(rejections=[r]),c.update(operations=[{'kind':'start'}])))
 add('activation_failed',lambda c:(c.update(activate=False),c.update(operations=[{'kind':'start'}])))
 add('point_target',lambda c:c['request'].update(target=None,point=list(map(bits,[6,9]))))
 add('actor_plus_point',lambda c:c['request'].update(point=list(map(bits,[30,50]))))
 add('ability_not_listed',lambda c:c['state'].update(abilities=[4]))
 add('turret_wait',lambda c:(c.update(uses_turret=True,turret_wait=True),c['operations'].insert(1,{'kind':'event','event':0})))
 add('no_mover',lambda c:c.update(mover=False))
 add('near_target',lambda c:c.update(target_position=list(map(bits,[1.02,2.01,5]))))
 add('no_animator',lambda c:c.update(animator=False))
 add('no_animation',lambda c:c.update(selected=None))
 add('retain_old',lambda c:(c.update(keep_previous=True,belongs=True),c['state'].update(animation=6)))
 add('old_wrong_variant',lambda c:(c.update(keep_previous=True,belongs=False),c['state'].update(animation=6)))
 add('end_without_idle',lambda c:c.update(idle=None))
 for reload in [-2,0,.00001,.0001,.00011,3.7]:add('reload_'+str(reload),lambda c,r=reload:c.update(reload=bits(r)))
 add('rejected_at_use',lambda c:c.update(rejections=[0,1]))
 add('target_stale_at_use',lambda c:c.update(target_live=False))
 add('pause_reschedules',lambda c:c.update(paused=True))
 add('inactive_ignores',lambda c:(c.update(active=False),c.update(operations=[{'kind':'event','event':1}])))
 add('mismatched_event',lambda c:c.update(operations=[{'kind':'start'},{'kind':'event','event':2}]))
 add('negative_event',lambda c:c.update(operations=[{'kind':'event','event':-1}]))
 for x in [.33333334,1e-7,16777216.25]:add('fractional_'+str(x),lambda c,v=x:(c.update(now=bits(v)),c['animation'].update(length=bits(v*1.17),use_fraction=bits(.717))))
 return [{**copy.deepcopy(c),'precision':pc,'name':c['name']+'_'+pc} for pc in ['Single24','Double53','Extended64'] for c in rows]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as exc:raise RuntimeError(c['name']) from exc
  rows.append({'input':c,'steps':out});visited.update(m.executed)
 report={'schema':1,'passed':True,'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-performer-fixtures.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
 print(json.dumps({'passed':True,'sequences':len(rows),'transitions':sum(len(r['steps']) for r in rows)}))
if __name__=='__main__':main()
