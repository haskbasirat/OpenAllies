"""Original ElementMove Think/entry/exit with real small branch/arithmetic leaves.
World/component virtuals, routing operations and lazy-setting registration are
explicit prepared fixture boundaries, not claimed implementations of those leaves.
"""
from pathlib import Path
import copy,json,struct,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
F=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ACT=0x4000000;MIND=0x4010000;MOVER=0x4020000;ELEM=0x4030000;WORLD=0x4040000;GRID=0x4050000;IDS=0x4060000;VT=0x40d0000;HOOK=0x40e0000;NODE=0x40f0000
def actor(k):return ACT+k*0x400
def mind(k):return MIND+k*0x400
def mover(k):return MOVER+k*0x100
def element(k):return ELEM+k*0x100
class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.trace=[];self.visited={};self.seams={}
  self.u.mem_map(0,0x1000);self.u.mem_map(ACT,0x100000);self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.reset(0xa354a4);self.w(self.STATE+4,mind(1));self.w(self.STATE+8,0xb3d350);self.w(self.STATE+12,c['status'])
  self.w(0xb36534,WORLD);self.w(0xb364c0,IDS);self.w(WORLD+0x30,GRID);self.w(WORLD+0xe0,c['time']);self.w(0xb3d304,1);self.w(0xb3d300,WORLD+0x200);self.w(WORLD+0x200,c['detached_seconds']);self.u.mem_write(0xb3d248,bytes([c['diagnostic']]))
  self.put(GRID+12,*c['extents']);self.w(GRID+4,1);self.w(GRID+0x1c,64);self.w(GRID+0x28,0x4080000)
  for k,s in c['actors'].items():
   k=int(k);a=actor(k);self.w(a,VT);self.w(a+4,WORLD+0x300);self.w(a+8,s['flags']);self.w(a+0x14,0x10000+k);self.put(a+0x20,*s['position']);self.put(a+0x2c,F(0),F(-1))
   for off,name,fn in [(0x6c,'mover',mover),(0x70,'mind',mind),(0x80,'element',element),(0x74,'component74',lambda x:x),(0x7c,'organization',lambda x:x),(0x88,'component88',lambda x:x)]:self.w(a+off,fn(s[name])if s[name]is not None else 0)
   self.w(a+0xd0,int(s['component_d0']));self.w(a+0x14c,len(s['kingdom_slots']));self.w(a+0x148,a+0x200);self.put(a+0x200,*[x or 0 for x in s['kingdom_slots']])
   self.u.mem_write(IDS+4+k*2,struct.pack('<H',1));self.w(IDS+0x20004+k*4,a)
  self.w(WORLD+0x308,0)
  for k,s in c['elements'].items():self.w(element(int(k))+0x10,actor(s['organization_actor'])if s['organization_actor'] else 0);self.u.mem_write(element(int(k))+0x1c,bytes([s['special']]))
  for k,s in c['movers'].items():
   a=mover(int(k));self.w(a+0x1c,s['last_step_speed']);self.w(a+0x20,s['speed']);self.w(a+0x24,s['requested']);self.w(a+0x28,s['maximum_speed']);self.w(a+0x50,s['factor']);self.u.mem_write(a+0x4d,bytes([s['locked']]));self.u.mem_write(a+0x55,bytes([s['turning'],s['falling']]))
  for k in range(1,4):
   self.w(mind(k),VT+0x400);self.w(mind(k)+4,actor(k));self.w(mind(k)+8,VT+0x800);self.w(mind(k)+0x10,mind(k)+0x200 if c['active']else 0);self.w(mind(k)+0x200,self.STATE)
  self.w(mind(1)+4,actor(c['owner']));self.w(VT+0x804,HOOK+1)
  for off in [0,0x108,0x138,0x88]:self.w(VT+off,HOOK+0x100+off)
  for off in [0xfc,0x124,0x140,0xf4,0x144,0x4c,0x50,0x54,0x6c]:self.w(VT+0x400+off,HOOK+0x400+off)
  self.load_payload(c['payload']);self.w(0xb3652c,WORLD+0x400)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def load_payload(self,p):
  m=p['plan']['movement'];self.put(self.STATE+0x10,m['target_id'],*m['destination_bits'],*m['auxiliary_bits'],m['requested_speed_bits'],p['plan']['attempts'],m['phase'],m['mode'],*p['plan']['previous_destination_bits'],*p['collision'],*p['collision_vector'],m['waypoint_offset_bits'])
  path=m['path'];self.w(self.STATE+0x5c,NODE if path['nodes']else 0);self.w(self.STATE+0x60,NODE+(len(path['nodes'])-1)*0x30 if path['nodes']else 0)
  self.put(self.STATE+0x64,path['kind1_count'],path['kind2_count']);self.u.mem_write(self.STATE+0x6c,bytes([path['dirty']]));self.w(self.STATE+0x70,path['word18']);self.put(self.STATE+0x74,p['detached_time_bits'],p['word78'])
  for i,n in enumerate(path['nodes']):self.put(NODE+i*0x30,*n['words'],NODE+(i-1)*0x30 if i else 0,NODE+(i+1)*0x30 if i+1<len(path['nodes'])else 0)
 def ret(self,value=0,n=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
 def fp(self,value,wide=False):
  self.u.mem_write(HOOK+0x900,struct.pack('<Q'if wide else'<I',value));self.u.mem_write(HOOK+0x910,(b'\xdd\x05'if wide else b'\xd9\x05')+struct.pack('<I',HOOK+0x900)+b'\xc3');self.u.reg_write(UC_X86_REG_EIP,HOOK+0x910)
 def mutation(self,name):
  change=self.c['mutations'].get(name,{})
  if 'owner'in change:self.w(mind(1)+4,actor(change['owner']))
  if 'destination'in change:self.put(self.STATE+0x14,*change['destination'])
  if 'mode'in change:self.w(self.STATE+0x30,change['mode'])
  if 'time'in change:self.w(WORLD+0xe0,change['time'])
  if 'detached_seconds'in change:self.w(WORLD+0x200,change['detached_seconds'])
 def log(self,name,**data):self.trace.append(dict(op=name,**data));self.mutation(name)
 def hook(self,u,pc,size,_):
  sp=u.reg_read(UC_X86_REG_ESP);ecx=u.reg_read(UC_X86_REG_ECX);c=self.c;cb=c['callbacks'];r=self.r
  if pc==0x404ed0:self.log('diagnostic');self.ret();return
  if pc==0x578e90:self.log('organization_ready',organization=ecx);self.ret(int(cb['organization_ready']));return
  if pc==0x54ef40:self.log('destination_has_kingdom_object',point=[r(self.STATE+0x14),r(self.STATE+0x18)],kingdom=r(sp+4));self.ret(int(cb['destination_has_kingdom_object']),4);return
  if pc==0x5693b0:self.log('component88_blocks',component=ecx);self.ret(int(cb['component88_blocks']));return
  if pc==0x4d10a0:self.log('actor_can_move',actor=(ecx-ACT)//0x400);self.ret(int(cb['actor_can_move']));return
  if pc==0x533c30:self.log('update_target');self.ret(actor(cb['update_target'])if cb['update_target']else 0);return
  if pc==0x52fe60:
   # Keep real52FE60 predicate; only its full descriptor/range service is a seam.
   pass
  if pc==0x4ea0f0:
   out=r(sp+4);self.put(out,ecx,0,0,0,0,0);self.ret(out,4);return
  if pc==0x46f100:
   source=r(r(sp+8));target=r(ecx);self.log('soft_shapes_in_range',source=(source-ACT)//0x400,target=(target-ACT)//0x400,range_bits=r(sp+4));self.ret(int(cb['soft_shapes_in_range']),12);return
  if pc==0x7aa3d0:self.ret(WORLD+0x500,8);self.w(WORLD+0x500,cb['region']);self.u.mem_write(WORLD+0x570,bytes([len(cb['neighbors'])]));self.w(WORLD+0x574,WORLD+0x600);self.put(WORLD+0x600,*[WORLD+0x500+v*0x1000 for v in cb['neighbors']]);self.w(WORLD+0x414,WORLD+0x700);self.w(WORLD+0x700+r(self.STATE+0x70)*4,WORLD+0x500+r(self.STATE+0x70)*0x1000);return
  if pc in [0x5336b0,0x5379c0]:
   if pc==0x5336b0:self.log('result_plan',trigger=(r(sp+4)-ACT)//0x400 if r(sp+4)else None,argument=bool(r(sp+8)&255));self.ret(n=8)
   else:self.log('continue_path');self.ret()
   return
  if pc==0x5767c0:self.log('stop_mover',mover=(ecx-MOVER)//0x100);self.ret(n=8);return
  if pc==0x4db0f0:self.log('mover_active',mover=(ecx-MOVER)//0x100);self.ret(int(c['movers'][str((ecx-MOVER)//0x100)]['active']));return
  if pc==0x4cfc80:self.log('stop_character',actor=(ecx-ACT)//0x400);self.ret();return
  if pc==0x57dea0:self.log('clear_component74',component=ecx);self.ret();return
  if pc==0x57dd90:
   target=r(sp+8);point=r(sp+12);self.log('component74_target',component=ecx,mode=r(sp+4),target=(target-ACT)//0x400 if target else None,point=[r(point),r(point+4)]);self.ret(n=12);return
  if pc==0x85c2b0:self.ret(n=4);return
  if pc==HOOK+1:self.log('schedule',mind=(ecx-MIND-8)//0x400,delay_bits=r(sp+4));self.ret(n=8);return
  if pc==HOOK+0x100:self.log('radius',actor=(ecx-ACT)//0x400);self.fp(cb.get('radius64',cb['radius']),'radius64'in cb);return
  if pc==HOOK+0x208:self.log('property',actor=(ecx-ACT)//0x400);self.ret(cb['property']);return
  if pc==HOOK+0x238:self.log('accepts_target_property',actor=(ecx-ACT)//0x400,property=r(sp+4));self.ret(int(cb['accepts_target_property']),4);return
  if pc==HOOK+0x188:
   self.log('target_point',target=(ecx-ACT)//0x400,property=r(sp+4));self.put(r(sp+8),*cb['target_point']);self.ret(int(cb['target_point_ok']),8);return
  if HOOK+0x400<=pc<HOOK+0x600:
   slot=pc-HOOK-0x400;k=(ecx-MIND)//0x400
   if slot in [0xfc,0x124,0x140,0xf4,0x144]:
    event={0xfc:'DetachedFc',0x124:'Failed124',0x140:{'TargetLost140':r(sp+4)},0xf4:{'TargetReachedF4':(r(sp+4)-ACT)//0x400},0x144:'Completed144'}[slot]
    self.log('mind_event',mind=k,event=event);self.ret(int(cb['detached_result']),4 if slot in[0x140,0xf4]else 0)
   elif slot in[0x4c,0x50]:self.log('mind_target',mind=k,slot=slot);v=cb['mind_target4c'if slot==0x4c else'mind_target50'];self.ret(actor(v)if v else 0)
   elif slot==0x6c:self.log('mind_has_location',mind=k);self.ret(int(cb['mind_has_location']))
   elif slot==0x54:self.log('mind_location',mind=k);self.put(WORLD+0x800,*cb['mind_location']);self.ret(WORLD+0x800)
   else:raise RuntimeError(hex(pc))
   return
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
 def output(self):
  p=copy.deepcopy(self.c['payload']);m=p['plan']['movement'];r=self.r
  m.update(target_id=r(self.STATE+0x10),destination_bits=[r(self.STATE+0x14),r(self.STATE+0x18)],auxiliary_bits=[r(self.STATE+0x1c),r(self.STATE+0x20)],requested_speed_bits=r(self.STATE+0x24),phase=r(self.STATE+0x2c),mode=r(self.STATE+0x30),waypoint_offset_bits=r(self.STATE+0x54))
  nodes=[];ptr=r(self.STATE+0x5c)
  while ptr:nodes.append({'words':[r(ptr+i*4)for i in range(8)]});ptr=r(ptr+0x24)
  m['path'].update(nodes=nodes,kind1_count=struct.unpack('<i',struct.pack('<I',r(self.STATE+0x64)))[0],kind2_count=struct.unpack('<i',struct.pack('<I',r(self.STATE+0x68)))[0],dirty=bool(self.u.mem_read(self.STATE+0x6c,1)[0]),word18=r(self.STATE+0x70))
  p['plan'].update(attempts=struct.unpack('<i',struct.pack('<I',r(self.STATE+0x28)))[0],previous_destination_bits=[r(self.STATE+0x34),r(self.STATE+0x38)])
  p.update(collision=[r(self.STATE+0x3c+i*4)for i in range(4)],collision_vector=[r(self.STATE+0x4c),r(self.STATE+0x50)],detached_time_bits=r(self.STATE+0x74),word78=r(self.STATE+0x78))
  return {'payload':p,'status':r(self.STATE+12),'owner':(r(mind(1)+4)-ACT)//0x400,'time':r(WORLD+0xe0),'detached_seconds':r(WORLD+0x200),'diagnostic':bool(self.u.mem_read(0xb3d248,1)[0]),'movers':{k:{'requested':r(mover(int(k))+0x24),'factor':r(mover(int(k))+0x50)}for k in self.c['movers']},'trace':self.trace}
 def run(self):
  entry={'think':0x537b90,'enter':0x52fc50,'leave':0x52e0f0}[self.c['operation']];self.u.emu_start(entry,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  return self.output()
def base():
 a=lambda:{'flags':0,'position':list(map(F,[5,5])),'mover':1,'mind':1,'component74':None,'organization':None,'element':None,'component88':None,'component_d0':False,'kingdom_slots':[]}
 actors={str(k):a()for k in range(1,4)};actors['2'].update(mover=2,mind=2,organization=77,kingdom_slots=[None]*4+[55]);actors['3'].update(mover=3,mind=3,position=list(map(F,[30,30])),component_d0=True)
 return {'status':2,'operation':'think','owner':1,'active':True,'diagnostic':False,'time':F(10),'detached_seconds':F(2),'extents':list(map(F,[128,128])),
 'actors':actors,'elements':{'1':{'organization_actor':2,'special':False}},'movers':{str(k):{'last_step_speed':0,'speed':0,'maximum_speed':F(4),'locked':False,'turning':False,'falling':False,'requested':F(3),'factor':F(.5),'active':True}for k in range(1,4)},
 'payload':{'plan':{'movement':{'target_id':0,'destination_bits':list(map(F,[20,20])),'auxiliary_bits':[0,0],'requested_speed_bits':F(3),'phase':0,'mode':0,'waypoint_offset_bits':0,'path':{'nodes':[],'kind1_count':0,'kind2_count':0,'dirty':False,'word18':0xffffffff}},'attempts':2,'previous_destination_bits':[F(1),F(2)]},'collision':[0]*4,'collision_vector':[0,0],'detached_time_bits':F(1),'word78':9},
 'callbacks':{'organization_ready':True,'destination_has_kingdom_object':True,'component88_blocks':False,'actor_can_move':True,'detached_result':False,'update_target':3,'radius':F(1),'property':111,'accepts_target_property':True,'soft_shapes_in_range':False,'region':1,'neighbors':[2,3],'mind_target4c':None,'mind_target50':None,'mind_has_location':False,'mind_location':list(map(F,[7,8])),'target_point':list(map(F,[11,12])),'target_point_ok':True},'mutations':{}}
def cases():
 rows=[]
 for i in range(84):
  c=base();p=c['payload'];m=p['plan']['movement'];a=c['actors']['1'];v=c['movers']['1'];cb=c['callbacks']
  if i<8:c['status']=i%4;c['diagnostic']=i>=4
  elif i<20:
   a['element']=1;cb['organization_ready']=i%3!=0;cb['destination_has_kingdom_object']=i%4==0;c['active']=i%5!=0;cb['detached_result']=i%2==0;c['time']=F(3 if i<14 else 10)
   if i==18:c['mutations']['organization_ready']={'detached_seconds':F(20)}
   if i==19:c['elements']['1']['special']=True
  elif i<30:
   if i==20:a['mover']=None
   elif i==21:v['falling']=True
   elif i<24:v['locked']=True;v['speed']=F(i%2)
   elif i<26:a['component88']=88;cb['component88_blocks']=True;v['turning']=bool(i%2)
   elif i<28:a['flags']|=1<<29;v['last_step_speed']=F(i%2)
   else:cb['actor_can_move']=False;v['active']=False
  elif i<45:
   m.update(target_id=0x10003,mode=2);c['actors']['3']['position']=list(map(F,[5+(i-30),5]));cb['update_target']=None if i==30 else 3
   if i in[31,32]:c['actors']['3']['flags']=1 if i==31 else 1<<29
   if i==33:cb['update_target']=1
   if i in[34,35]:c['actors']['3']['component_d0']=i!=34;cb['accepts_target_property']=False
   if i==36:c['mutations']['radius']={'owner':2}
   if i==37:c['active']=False
  elif i<58:
   a['component74']=74;cb['mind_target4c']=3 if i%3==0 else None;cb['mind_target50']=3 if i%4==0 else None;cb['mind_has_location']=i%2==0;cb['target_point_ok']=i%5!=0
   if i>=54:a['element']=1
   if i==56:a['organization']=99
   if i==57:c['actors']['2']['mind']=None
  elif i<67:
   m['path'].update(nodes=[{'words':[F(6),F(7),0,0,1,0,0,0]}],kind1_count=1,word18=[0xffffffff,1,2,8][i%4]);m['phase']=i%3;p['collision']=[2,0x10003,0,0];c['actors']['3']['flags']=0 if i%2 else 1<<29
  else:
   m.update(mode=i%4,target_id=0x10003 if i%2 else 0,destination_bits=list(map(F,[5+[0,.01,.099,.1,.49,.5,.51,1][i%8],5])));c['actors']['3']['flags']=1<<10;cb['soft_shapes_in_range']=i%3==0
   if i==83:c['mutations']['actor_can_move']={'destination':list(map(F,[5,5]))}
  rows.append(c)
 for i in range(12):
  c=base();c['payload']['plan']['movement'].update(target_id=0x10003,mode=2);c['actors']['3']['position']=list(map(F,[12,5]));c['callbacks']['radius64']=struct.unpack('<Q',struct.pack('<d',1.+(i-5)*1e-10))[0];rows.append(c)
 for i in range(24):
  c=base();c['operation']='enter';m=c['payload']['plan']['movement'];m.update(phase=i%3,mode=i%4,requested_speed_bits=[0,F(.00001),F(3),F(10)][i%4],destination_bits=list(map(F,[[-1,5,128,129][i%4],[-0.,5.49,5.5,200][i%4]])));m['path'].update(nodes=[{'words':[0]*8}],kind1_count=2,kind2_count=3,word18=5);rows.append(c)
 for i in range(8):
  c=base();c['operation']='leave';c['actors']['1']['component74']=74 if i%2 else None;c['movers']['1']['active']=i%3==0;c['payload']['plan']['movement']['path']['nodes']=[{'words':[0]*8}]
  if i==7:c['mutations']['clear_component74']={'owner':2}
  rows.append(c)
 return [{**copy.deepcopy(c),'precision':pc,'name':f'{pc}_{i}_{c["operation"]}'}for pc in ['Single24','Double53','Extended64']for i,c in enumerate(rows)]
def leaf_cases():
 rows=[];visited={}
 from capstone import Cs,CS_ARCH_X86,CS_MODE_32
 for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Base(cw);m.u.mem_map(ACT,0x100000)
  def hook(u,address,size,_):
   if original.base<=address<original.base+original.image_size:
    raw=original.range(address,size);assert raw==modified.range(address,size);visited[address]=raw.hex()
  m.u.hook_add(UC_HOOK_CODE,hook)
  for origin in [[0.,0.],[5.123456,9.876543],[65536.125,-65536.],[1e-20,-1e-20]]:
   for delta in [[0.,0.],[.5,0.],[.49999997,0.],[.50000006,0.],[0.,.1],[.099999994,0.],[.1,.1],[.35355338,.35355338],[.35355341,.35355341]]:
    for entry,threshold in [(0x52a1f0,0x3e800000),(0x52a240,0x3c23d70a)]:
     point=[F(origin[i]+delta[i])for i in range(2)];position=list(map(F,origin));m.reset(0);m.w(m.STATE+4,ACT);m.w(ACT+0x20,position[0]);m.w(ACT+0x24,position[1]);m.w(m.STACK+4,point[0]);m.w(m.STACK+8,point[1]);m.u.emu_start(entry,m.STOP,count=1000)
     assert m.u.reg_read(UC_X86_REG_EIP)==m.STOP and m.u.reg_read(UC_X86_REG_ESP)==m.STACK+12
     rows.append({'precision':pc,'operation':'near','position':position,'point':point,'threshold':threshold,'result':bool(m.u.reg_read(UC_X86_REG_EAX)&255)})
  for extent in [[128.,256.],[.001,.002],[65536.,131072.],[1.234567,7.777777]]:
   for point in [[-.1,-0.],[0.,0.],[extent[0],extent[1]],[extent[0]-.0001,extent[1]-.0001],[extent[0]-.00009,extent[1]-.00009],[extent[0]*.5,extent[1]*.5]]:
    m.reset(0);m.w(m.STATE+12,F(extent[0]));m.w(m.STATE+16,F(extent[1]));m.w(ACT,F(point[0]));m.w(ACT+4,F(point[1]));m.w(m.STACK+4,ACT);m.u.emu_start(0x5525d0,m.STOP,count=1000)
    assert m.u.reg_read(UC_X86_REG_EIP)==m.STOP and m.u.reg_read(UC_X86_REG_ESP)==m.STACK+8
    rows.append({'precision':pc,'operation':'clamp','point':list(map(F,point)),'extents':list(map(F,extent)),'result':[m.r(ACT),m.r(ACT+4)]})
 return rows,visited
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as e:raise RuntimeError(c['name'])from e
  rows.append({'input':c,'output':out});visited.update(m.visited)
 leaves,lv=leaf_cases();visited.update(lv)
 constants=[]
 for lo,length in [(0xa354a4,40),(0xa033bc,4),(0xa03078,4),(0xa3aa10,4),(0xa34770,4),(0xa02b30,4)]:
  raw=original.range(lo,length);assert raw==modified.range(lo,length);constants.append({'va':hex(lo),'bytes':raw.hex()})
 result={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__)),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'cases':rows,'leaf_cases':leaves,'constants':constants,'executed_windows':[{'va':hex(a),'bytes':v}for a,v in sorted(visited.items())]}
 path=HERE/'native-element-move.json';path.write_text(json.dumps(result,indent=2)+'\n');print({'cases':len(rows),'sha256':H(path)})
if __name__=='__main__':main()
