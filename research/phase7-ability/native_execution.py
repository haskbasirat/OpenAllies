"""Original ability4C9550/area4C9C90 through DamageEffect4CA400 and real
Body5625C0. World query/virtual/FX/payment/projectile allocation remain explicit
leaves; area collection, native linked sorting and effect dispatch execute.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-weapon'))
import native_weapon as weapon
body=weapon.body;bits=weapon.bits
from unicorn.x86_const import *
EXTRA=0x20d0000;VT=0x20df000;OTHER=0x20df800;ALLEG=0x20df810;KING=0x20df820;RADIUS=0x20df830
class Machine(weapon.Machine):
 def __init__(self,c):
  self.execution_case=copy.deepcopy(c);self.action_events=[];self.query=None;self.alloc=0x20e0000
  super().__init__(c['weapon_case'])
  self.handle_ptrs={1:body.SOURCE,2:self.ACTOR,3:body.ALLEGIANCE,4:EXTRA+0x8000,5:EXTRA+0x8400,6:EXTRA+0x8800};self.ptr_handles={v:k for k,v in self.handle_ptrs.items()}
  self.w(body.SOURCE,VT);self.w(VT+0xa8,KING);self.w(VT+0x108,ALLEG)
  self.w(0x2091000,RADIUS)
  for h,a in self.handle_ptrs.items():
   data=c['actors'][str(h)];self.w(a+8,data['flags8']);self.put(a+0x20,*data['position'])
   if h>=4:self.w(a,0x2091000);self.w(a+4,a+0x200)
   self.w(self.r(a+4)+0x168,data['definition_flags168'])
  d=c['definition'];df=body.WEAPON_DATA
  if c['ability']['definition']==0:self.w(body.WEAPON+4,0)
  for key,offset in [('flags',0x54),('area',0x38),('volley_area',0x4c),('max_targets',0x50),('impact_impulse',0x44),('attenuation_minimum',0x58),('attenuation_factor',0x5c),('impulse_area',0x60)]:self.w(df+offset,d[key])
  self.w(df+0x3c,123 if d['projectile'] is not None else 0)
  self.w(body.WEAPON+0xc,c['ability']['use_count']);self.w(body.WEAPON+0x10,c['ability']['last_use']);self.w(body.WEAPON+0x20,c['ability']['power'])
  for i,e in enumerate(c['effects']):
   ptr=EXTRA+i*0x100;self.w(ptr,VT+0x200+i*0x10);self.w(VT+0x200+i*0x10+8,0x4ca400 if e['damage'] is not None else OTHER)
   self.w(ptr+4,e['damage']['kind'] if e['damage'] is not None else 99);self.w(ptr+8,e['targets']['required']);self.w(ptr+0xc,e['targets']['forbidden'])
   if e['damage'] is not None:self.w(ptr+0x14,e['damage']['damage_type']);self.w(ptr+0x18,e['damage']['amount'])
   self.put(EXTRA+0x4000+i*8,ptr,EXTRA+0x4000+(i+1)*8 if i+1<len(c['effects']) else 0)
  self.w(df+0x68,EXTRA+0x4000 if c['effects'] else 0)
  self.context_write(c['context'])
 def put(self,a,*w):self.u.mem_write(a,struct.pack('<'+'I'*len(w),*(v&0xffffffff for v in w)))
 def context_write(self,c):self.put(weapon.CONTEXT,c['event_kind'],c['phase'],self.handle_ptrs.get(c['source'],0),self.handle_ptrs.get(c['allegiance'],0),*c['source_position'],self.handle_ptrs.get(c['target'],0),*c['target_position'],c['word2c'])
 def context(self,p=weapon.CONTEXT):
  w=struct.unpack('<12I',self.u.mem_read(p,48));return{'event_kind':w[0],'phase':w[1],'source':self.ptr_handles.get(w[2]),'allegiance':self.ptr_handles.get(w[3]),'source_position':list(w[4:7]),'target':self.ptr_handles.get(w[7]),'target_position':list(w[8:11]),'word2c':w[11]}
 def action_event(self,name,**fields):self.action_events.append({'name':name,**fields})
 def hook(self,u,pc,size,user):
  if not hasattr(self,'handle_ptrs'):return super().hook(u,pc,size,user)
  if pc==KING:self.ret(value=int(self.execution_case['source_kingdom']));return
  if pc==ALLEG:self.ret(value=body.ALLEGIANCE);return
  if pc==RADIUS:
   self.u.mem_write(EXTRA+0x7000,struct.pack('<I',self.execution_case['radius']))
   self.u.mem_write(EXTRA+0x7100,b'\xd9\x05'+struct.pack('<I',EXTRA+0x7000)+b'\xc3');u.reg_write(UC_X86_REG_EIP,EXTRA+0x7100);return
  if pc==0x4c8500:self.action_event('fx',event=self.arg(),context=self.context(self.arg(1)));self.ret(8);return
  if pc==0x4e2590:self.action_event('payment');self.ret(4);return
  if pc==0x4c8cb0:self.action_event('projectile',context=self.context(self.arg()));self.ret(4,123 if self.execution_case['projectile_success'] else 0);return
  if pc==OTHER:self.action_event('other',effect=100+(u.reg_read(UC_X86_REG_ECX)-EXTRA)//0x100,context=self.context(self.arg(1)),scale=self.arg(2));self.ret(12,1);return
  if pc==0x4c9c90:
   self.action_event('begin_area',radius=self.arg(),center=[self.r(self.arg(1)+0x20),self.r(self.arg(1)+0x24)]);self.query=iter(self.execution_case['area_targets'])
  if pc==0x5581f0:self.ret(value=u.reg_read(UC_X86_REG_ECX));return
  if pc==0x552dd0:self.ret();return
  if pc==0x46d8e0:self.ret(4);return
  if pc==0x552250:self.ret(8);return
  if pc==0x558040:self.ret(8);return
  if pc==0x558580:
   target=next(self.query,None);self.action_event('next_area',target=target);self.ret(value=self.handle_ptrs.get(target,0));return
  if pc==0x557fd0:self.action_event('end_area');self.ret();return
  if pc==0x6f7810:
   ptr=self.alloc;self.alloc+=(self.arg()+15)&~15;self.ret(value=ptr);return
  if pc==0x6f7430:self.ret();return
  if pc==0x45d430:self.ret(value=int(self.execution_case['structure']));return
  if pc==0x50c6f0:self.action_event('impulse',actor=self.ptr_handles[self.arg()],vector=[self.arg(i) for i in [1,2,3]]);self.ret(16);return
  if pc==0x4f11b0:self.action_event('area_impulse',position=[self.arg(i) for i in [0,1,2]],amount=self.arg(3),radius=self.arg(4));self.ret(20);return
  if pc==0x552640:
   point=self.arg();xy=self.execution_case['clamped'];self.put(point,*xy);self.action_event('clamp',position=xy);self.ret(4);return
  if pc==0x4282a0:self.ret(value=int(self.execution_case['primary_allowed']));return
  if pc==0x42a530:self.ret(8,123);return
  if pc==0x7b9300:self.ret(value=123);return
  if pc==0x7b8fd0:self.ret(value=self.r(0xb3652c+0x2c));return
  super().hook(u,pc,size,user)
 def run(self):
  self.w(self.STACK,self.STOP);self.w(self.STACK+4,weapon.CONTEXT);self.w(self.STACK+8,self.execution_case['time']);self.u.reg_write(UC_X86_REG_ECX,body.WEAPON);self.u.reg_write(UC_X86_REG_ESP,self.STACK)
  regs={r:0x88ab0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  try:self.u.emu_start(0x4c9550,self.STOP,count=2000000)
  except Exception as exc:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP))) from exc
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
  for r,v in regs.items():assert self.u.reg_read(r)==v
  return{'result':self.u.reg_read(UC_X86_REG_EAX),'context':self.context(),'use_count':self.r(body.WEAPON+0xc),'last_use':self.r(body.WEAPON+0x10),'actions':self.action_events,'requests':self.deliveries,'body':self.state(),'environment':self.env,'events':self.events}

def base(name):
 w=weapon.base(name);w['body_input']['view']['wall_link']=False
 return{'name':name,'precision':'Double53','weapon_case':w,'time':bits(9.5),
 'ability':{'definition':10,'local_definition':11,'use_count':2,'last_use':bits(-100),'word14':0,'enabled':True,'recharge_bypass':False,'fuzzy_range':0,'power':w['hit']['weapon']['amount'],'summoned_ids':[],'word28':0},
 'definition':{'flags':0,'area':0,'volley_area':0,'projectile':None,'max_targets':0,'impact_impulse':0,'attenuation_minimum':bits(1),'attenuation_factor':bits(.5),'impulse_area':bits(10)},
 'effects':[{'targets':{'required':0,'forbidden':0},'damage':w['hit']['rule']}],
 'context':{'event_kind':1,'phase':0,'source':1,'allegiance':3,'source_position':[0,0,0],'target':2,'target_position':[0,0,0],'word2c':0},
 'actors':{str(h):{'flags8':0x4000,'position':list(map(bits,[h*3,h*4,2])),'definition_flags168':0}for h in range(1,7)},
 'source_kingdom':False,'projectile_success':True,'revalidation':0,'area_targets':[2], 'radius':bits(.5),'structure':False,'clamped':list(map(bits,[5,6])),'primary_allowed':True}
def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  c['weapon_case']['hit']['weapon']['flags']=c['definition']['flags'];c['weapon_case']['hit']['weapon']['propagation']={'definition_word38':c['definition']['area'],'definition_word4c':c['definition']['volley_area']}
  c['weapon_case']['body_input']['request']['weapon_definition']=copy.deepcopy(c['weapon_case']['hit']['weapon']['propagation'])
  c['weapon_case']['body_input']['source_present']=c['context']['source'] is not None
  c['weapon_case']['body_input']['allegiance_present']=c['context']['allegiance'] is not None
  if c['context']['source'] is None:c['weapon_case']['body_input']['request']['record']['source_id']=0
  if c['context']['allegiance'] is None:c['weapon_case']['body_input']['request']['record']['allegiance_id']=0
  rows.append(c)
 add('direct_damage')
 add('missing_definition',lambda c:c['ability'].update(definition=0))
 add('no_effects',lambda c:c.update(effects=[]))
 add('no_target',lambda c:c['context'].update(target=None))
 add('location_allowed',lambda c:(c['context'].update(target=None),c['definition'].update(flags=0x20)))
 add('no_source',lambda c:c['context'].update(source=None))
 add('no_allegiance',lambda c:c['context'].update(allegiance=None))
 add('air_positions',lambda c:(c['actors']['1'].update(flags8=0x5000),c['actors']['2'].update(flags8=0x5000)))
 add('source_is_kingdom',lambda c:c.update(source_kingdom=True))
 add('self_other_effect',lambda c:c['effects'].insert(0,{'targets':{'required':0x20,'forbidden':0},'damage':None}))
 for phase in [1,2,3]:add('phase_'+str(phase),lambda c,v=phase:c['context'].update(phase=v))
 add('revalidate_failed',lambda c:(c['context'].update(phase=2),c['actors']['2'].update(flags8=1)))
 for ok in [False,True]:add('projectile_'+str(ok),lambda c,v=ok:(c['definition'].update(projectile=12),c.update(projectile_success=v)))
 add('projectile_arrival',lambda c:(c['definition'].update(projectile=12),c['context'].update(phase=2)))
 for max_targets in [0,1,2,8]:add('area_limit_'+str(max_targets),lambda c,v=max_targets:(c['definition'].update(area=bits(30),max_targets=v),c.update(area_targets=[4,2,5,6])))
 add('area_equal_distance',lambda c:(c['definition'].update(area=bits(30),max_targets=2),c.update(area_targets=[4,2,5,6]),[c['actors'][str(h)].update(position=list(map(bits,[6,8,2])))for h in [4,5,6]]))
 add('area_empty',lambda c:(c['definition'].update(area=bits(3)),c.update(area_targets=[])))
 add('volley_projectile',lambda c:(c['definition'].update(volley_area=bits(10),projectile=12,max_targets=2),c.update(area_targets=[4,2,5])))
 add('area_source_centered',lambda c:(c['definition'].update(area=bits(10),flags=0x10),c.update(area_targets=[2])))
 for distance in [.1,1,5,15,30,100]:add('attenuation_'+str(distance),lambda c,v=distance:(c['definition'].update(area=bits(30)),c['context'].update(phase=3,target_position=[0,0,0]),c['actors']['2'].update(position=list(map(bits,[v,0,2])))))
 add('impact',lambda c:(c['definition'].update(impact_impulse=bits(1.7)),c['actors']['2'].update(definition_flags168=4)))
 add('area_impulse',lambda c:c['definition'].update(area=bits(30),impact_impulse=bits(.3)))
 for flags in [1,2,3]:add('weapon_flags_'+str(flags),lambda c,f=flags:c['definition'].update(flags=f))
 return [{**copy.deepcopy(c),'precision':p,'name':c['name']+'_'+p,'weapon_case':{**copy.deepcopy(c['weapon_case']),'body_input':{**copy.deepcopy(c['weapon_case']['body_input']),'precision':p}}} for p in ['Single24','Double53','Extended64'] for c in rows]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as exc:raise RuntimeError(c['name']) from exc
  rows.append({'input':c,'output':out});visited.update(m.executed)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':body.pe.ORIGINAL_SHA,'accepted_sha256':body.pe.ACCEPTED_SHA,'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':body.IMAGE.at(a,size).hex()} for a,size in sorted(visited.items())]}
 (HERE/'native-execution-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
