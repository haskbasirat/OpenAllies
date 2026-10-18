"""Bounded original/v14 local Reset and first-player constructor oracle.

Session/Sim/Node objects are explicit constructor-field projections, NOT a full
native application startup. Native whole Reset76EC70 -> EndSession76E0F0,
DestroyWorld76AB80(no-world), notification constructors/copy/push, SetFrozen,
seed assignment, property assignment and Host75B126..75B1B1 run. Constructor
values are separately checked by native options, launch-state, properties,
RNG and Sim ctor field windows. Windows locks/events/tick/GUID, logging,
allocation, CString and text-table lookups are named successful seams.
No original files or profiles are written. Page zero is guarded by the reused
phase8-net machine. All executed native byte windows are compared across PEs.
"""
from pathlib import Path
import json,sys,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net'))
from native_lobby import LobbyMachine,PE0,PE1,PC,NODE,NETNODE,LOCK_VT,LOCK_RET,units,SYNC_A
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

STUB=0x7700800
class BootstrapMachine(LobbyMachine):
 def call(self,pc,args=(),this=None,callee_pop=0):
  self.w(self.stack,self.stop)
  for i,x in enumerate(args):self.w(self.stack+4+4*i,x)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.admin if this is None else this)
  regs={UC_X86_REG_EBX:0xabcdef01,UC_X86_REG_ESI:0xabcdef02,UC_X86_REG_EDI:0xabcdef03,UC_X86_REG_EBP:0xabcdef04}
  for r,v in regs.items():self.u.reg_write(r,v)
  self.u.emu_start(pc,self.stop,count=50_000_000 if not self.bootstrap_ready or pc==0x77ccb0 else 100_000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+callee_pop,(hex(pc),self.u.reg_read(UC_X86_REG_ESP))
  for r,v in regs.items():assert self.u.reg_read(r)==v,(hex(pc),r)
  return self.u.reg_read(UC_X86_REG_EAX)
 def __init__(self,image):
  self.bootstrap_ready=False
  super().__init__(False,image)
  self.bootstrap_ready=True
  self.native_events=[]
  self.tick=0;self.guid=bytes(16);self.notification=[]
  self.w(0x9f0284,STUB);self.w(0x9f0540,STUB+0x10)
  self.formats={'static_group_wildcard_name':'All Units'}
  self.exits={}
 def hook(self,u,pc,size,user):
  if self.bootstrap_ready:
   if pc==STUB:
    self.native_events.append({'kind':'GetTickCount','value':self.tick});self.ret(value=self.tick);return
   if pc==STUB+0x10:
    self.native_events.append({'kind':'CoCreateGuid'});u.mem_write(self.arg(),self.guid);self.ret(4,0);return
   if pc==0x405d60:
    fmt=self.text(self.arg(1))
    assert fmt=='%X%X%X%X%X%X%X%X%X%X%X',repr(fmt)
    vals=[self.arg(2+i)for i in range(11)]
    text=''.join(format(v,'X')for v in vals)
    self.native_events.append({'kind':'GuidFormat','values':vals,'text':text})
    self.cs(self.arg(),text);self.ret(value=self.arg());return
   if pc==0x59aba0:
    self.native_events.append({'kind':'diagnostic_os_milliseconds','value':256});self.ret(value=256);return
   if pc in (0x857e10,0x857de0,0x8573a0):
    self.native_events.append({'kind':{0x857e10:'queue_enter',0x857de0:'queue_leave',0x8573a0:'queue_signal'}[pc]})
    self.ret();return
   if pc==LOCK_RET:
    self.native_events.append({'kind':'world_lock','return':self.r(u.reg_read(UC_X86_REG_ESP))});self.ret();return
   if pc==0x7851c0:
    sp=u.reg_read(UC_X86_REG_ESP)
    # Full by-value 0x3C notification follows return address.
    self.notification.append(self.read_notification(sp+4))
   if pc in (0x76e0f0,0x76ab80,0x76dd90,0x85d880,0x74d460,0x79c910,0x773710,0x7661d0,0x4ed6d0,0x770280,0x766560,0x77b8f0):
    self.native_events.append({'kind':'native_call','address':f'{pc:08X}'})
  super().hook(u,pc,size,user)
 def read_notification(self,p):
  return {'kind':self.r(p),'text':self.rs(p+4),'state':self.r(p+8),'launch_state':self.read_state(p+0xc),
          'flag_34':self.rb(p+0x34)!=0,'word_38':self.r(p+0x38)}
 def read_state(self,p):
  return {'kind':self.r(p),'flag_04':self.rb(p+4)!=0,'text':self.rs(p+8),'field_0c':self.r(p+0xc),
   'field_10':self.r(p+0x10),'flag_14':self.rb(p+0x14)!=0,'flag_15':self.rb(p+0x15)!=0,
   'flag_16':self.rb(p+0x16)!=0,'field_18':self.r(p+0x18),'field_1c':self.r(p+0x1c),
   'flag_20':self.rb(p+0x20)!=0,'field_24':self.r(p+0x24)}
 def begin(self,tick,guid,application_kind):
  self.tick=tick;self.guid=bytes(guid);self.native_events=[];self.notification=[]
  self.S=self.alloc(0x178);self.u.mem_write(self.S,b'\xa5'*0x178);self.w(0xb36550,self.S)
  # Read fields for this bounded path; values have native constructor pins.
  for off,value in [(0x1c,0),(0x58,3),(0xbc,0),(0xc0,0),(0xc4,0),(0xe0,0),(0xe4,0),(0xf8,0),(0x158,0),(0x15c,0),(0x164,0)]:self.w(self.S+off,value)
  for off in (0xc8,0xc9,0xca,0xf4,0x168,0x16a,0x16b,0x16c,0x16d,0x16e):self.b(self.S+off,0)
  self.cs(self.S+0x160,'');self.call(0x77b7b0,this=self.S+0xcc)
  prop=self.alloc(0x14);self.call(0x79c6e0,this=prop);self.w(self.S+0x170,prop)
  src=self.alloc(0x14);self.call(0x79c6e0,this=src);self.w(0xb36568,src)
  self.call(0x85d870,this=self.S+0x128)
  self.sim=self.alloc(0x48);self.u.mem_write(self.sim,b'\xa5'*0x48)
  self.b(self.sim+0x29,1);self.w(self.sim+0x3c,0);self.w(0xb36554,self.sim)
  self.call(0x77ccb0,[0x10000],this=SYNC_A,callee_pop=4)
  self.w(NETNODE+0x24,LOCK_VT)
  self.w(NODE,0xa913b0);self.w(NODE+0x2ac,0);self.w(NODE+0x2b0,0)
  self.w(NODE+0x2c4,0);self.w(NODE+8,0);self.w(NODE+0x2c,0);self.b(NODE+0x2c0,0)
  self.cs(NODE+0x74,'')
  queue=self.alloc(0x3c);self.w(0xb40ba8,queue)
  descriptors=self.alloc(12);descriptor=self.alloc(8);self.cs(descriptor+4,'FixtureNotificationKind2');self.w(descriptors+8,descriptor);self.w(0xb40bb0,descriptors)
  # 784EF0's owned ring starts at queue+24. Windows mutex/event constructors
  # are platform seams; queue counters really begin empty.
  for off in (0x28,0x2c,0x30,0x34):self.w(queue+off,0)
  self.queue=queue;self.w(0xb396c0,application_kind)
  self.native_events=[];self.notification=[]
 def read_after(self):
  S=self.S
  p=self.r(S+0xc4)
  players=[]
  n=self.r(S+0xbc)
  while n:players.append(self.read_player(self.r(n)));n=self.r(n+4)
  return {'changed':[bool(self.rb(S+o))for o in (0xc8,0xc9,0xca)],'state':self.r(S+0xe4),'phase':self.r(S+0xf8),
   'last_player_id':self.r(S+0xe8),'rng_state':self.r(S+0x128),'multiplayer':bool(self.rb(S+0xf4)),
   'field_154':self.r(S+0x154),'text_160':self.rs(S+0x160),'sim_frozen':bool(self.rb(self.sim+0x29)),
   'players':players,'local_player':self.r(self.r(NODE+8)+0x20)if self.r(NODE+8)else None,
   'ordering_player':self.r(self.r(NODE+0x2c)+0x20)if self.r(NODE+0x2c)else None,
   'host':'Null'if self.r(NODE+0x2c4)==0 else 'Node','node_active':bool(self.rb(NODE+0x2c0)),
   'local_identity':{'name':self.rs(NODE+0x74),'serial':self.r(NODE+0x78),'a':self.r(NODE+0x7c),'b':self.r(NODE+0x80),'c':self.r(NODE+0x84)},
   'sync_a_enabled':bool(self.rb(SYNC_A)),'sync_a_hash':struct.unpack('<i',self.u.mem_read(SYNC_A+8,4))[0],
   'sync_a_end':struct.unpack('<i',self.u.mem_read(SYNC_A+0x14,4))[0]}
 def first_player(self,nickname,identity):
  user=self.alloc(0x140);self.cs(user+0x2c,nickname);self.w(0xb39d04,user)
  flags=self.alloc(0xc0);self.b(flags+0xbc,0);self.w(0xb3654c,flags)
  account=self.alloc(0x40);self.cs(account+0x1c,identity);self.w(0xb40c24,account)
  args=self.alloc(0x40);self.b(args+0x35,0)
  self.u.reg_write(UC_X86_REG_ESI,NODE);self.u.reg_write(UC_X86_REG_EDI,args)
  self.u.reg_write(UC_X86_REG_ESP,self.stack)
  self.u.emu_start(0x75b126,0x75b1b1,count=5_000_000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x75b1b1
  p=self.r(NODE+8);view=self.r(p+0x50);group=self.r(view+0x24)
  return {'player_vector_count':self.r(p+0x48),'player_vector_capacity':self.r(p+0x4c),
    'view_present':view!=0,'view_kingdom':self.r(view),'view_word_0c':self.r(view+0xc),
    'view_zero_fields':[self.r(view+o)for o in (4,8,0x10,0x14,0x18,0x1c,0x20,0x28,0x2c)],
    'group':{'key':self.rs(group),'title':self.rs(group+4),'text':self.rs(group+8),
     'count':self.r(group+0x10),'capacity':self.r(group+0x14),'flag_18':bool(self.rb(group+0x18)),
     'word_1c':self.r(group+0x1c),'flag_20':bool(self.rb(group+0x20))}}

def proof_ranges(visited):
 intervals=[]
 for a,size in sorted(visited.items()):
  if not PE0.base<=a<PE0.base+PE0.image_size:continue
  if intervals and intervals[-1][1]>=a:intervals[-1][1]=max(intervals[-1][1],a+size)
  else:intervals.append([a,a+size])
 out=[]
 for a,b in intervals:
  x=PE0.range(a,b-a);y=PE1.range(a,b-a)
  assert x==y,(hex(a),x.hex(),y.hex())
  out.append({'start':f'{a:08X}','end':f'{b:08X}','sha256':hashlib.sha256(x).hexdigest()})
 return out

def main():
 rows=[];visited={}
 cases=[(0,[0]*16,0,'','local-key'),(0xffffffff,list(range(16)),1,'Commander','identity-key'),
  (0x12345678,[0xff,0,0x80,1,2,3,4,5,6,7,8,9,10,11,12,13],3,'A\0B','profile'),
  (0x8e4b12a5,[15]*16,2,'\ud800\udfff','')]
 for image in ('original','v14'):
  m=BootstrapMachine(image)
  for precision in PC:
   for tick,guid,app,nickname,identity in cases:
    m.fpu(precision);m.begin(tick,guid,app)
    try:m.call(0x76ec70,this=m.S)
    except Exception:
     print('failure at',hex(m.u.reg_read(UC_X86_REG_EIP)),'events',m.native_events[-12:],flush=True)
     print('last addresses',[hex(v)for v in list(m.visited)[-20:]],flush=True)
     raise
    reset=m.read_after();notifications=list(m.notification)
    queued=m.r(m.queue+0x34)
    before=list(m.native_events)
    view=m.first_player(nickname,identity)
    rows.append({'image':image,'precision':precision,'tick':tick,'guid_bytes':guid,'application_kind':app,
     'nickname':units(nickname),'identity_key':units(identity),'reset':reset,'notification':notifications,
     'queued_count':queued,'reset_events':before,'after_player':m.read_after(),'player_view':view})
  visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 assert a==b
 out={'schema':1,'passed':True,'scope':'Projected fresh owners, whole Reset, and first local player prefix; no full constructor/Host/launch claim',
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'page_zero':'guarded by phase8-net null_guard',
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-bootstrap.json').write_text(json.dumps(out,ensure_ascii=True,indent=2)+'\n')
 print('passed',len(rows),'rows')
if __name__=='__main__':main()
