"""40B840/40B7C0 ->40B5C0 over native MessageSystem list, both images/all PCs.

OS lock/atomic allocation and typed current-source transport are explicit seams.
No whole printf, texture upload, process UI task queue or readiness claim.
Default immutable replay; --write creates the author fixture.
"""
from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-orders'))
from recover_orders import OrdersMachine,PE0,PE1
from unicorn.x86_const import *
PC={'Single':0x7f,'Double':0x27f,'Extended':0x37f}
LOCK=0x2110100;UNLOCK=LOCK+0x10;INC=LOCK+0x20;DEC=LOCK+0x30
class MessageMachine(OrdersMachine):
 def __init__(self,image):
  self.active=False;super().__init__()
  if image=='v14':
   for vs,r,size,off in PE1.sections:
    if PE1.base+r in(0x401000,0x9f0000):self.u.mem_write(PE1.base+r,PE1.b[off:off+size])
  self.executed={};self.seams={};self.steps=[];self.stop_reason=None;self.loading_type=False
  self.gd=self.alloc(0x900);self.w(0xb36530,self.gd)
  self.types=self.alloc(39*4);self.ty=self.alloc(0x40);self.w(self.gd+0x4ec,self.types);self.w(self.types+15*4,self.ty)
  self.view=self.alloc(0x100);self.filter=self.alloc(4);self.filter_slot=self.alloc(4);self.w(self.filter_slot,self.filter)
  self.uvars=self.alloc(0x400);self.maximum=self.alloc(4);self.w(self.uvars+0x10c,self.maximum);self.w(0xb39940,self.uvars)
  self.lockvt=self.alloc(16);self.w(self.lockvt+4,LOCK);self.w(self.lockvt+8,UNLOCK)
  self.w(0x9f01e8,INC);self.w(0x9f01ec,DEC)
  self.system=self.alloc(0x40);self.active=True;self.call(0x40b510,this=self.system);self.active=False
  self.ctor=self.snapshot();self.events=[]
 def hook(self,u,pc,size,user):
  seam=True
  if pc==0x857d60:self.w(self.ecx(),self.lockvt);self.ret()
  elif pc in(LOCK,UNLOCK):self.events.append('lock' if pc==LOCK else 'unlock');self.ret()
  elif pc in(INC,DEC):
   p=self.arg();v=(self.r(p)+(1 if pc==INC else -1))&0xffffffff;self.w(p,v);self.ret(4,v)
  elif pc==0x59aba0:self.events.append('timer');self.ret(value=self.inp['milliseconds'])
  elif pc==0x405370:self.events.append('filter');self.ret(4,self.filter_slot)
  elif pc==0x404f60:
   self.events.append('diagnostic')
   if self.inp.get('stop')=='diagnostic':self.stop_reason='diagnostic';u.emu_stop()
   else:self.ret()
  elif pc==0x4976b0:self.steps.append({'ordinal':self.arg(),'name':self.text(self.arg(1)),'third':self.arg(2)});self.ret(12)
  elif pc==0x96cad6:
   #Named CRT case comparison transport. Only the actual ASCII predefined
   #names from497BA0 are supplied; no locale/default process init is claimed.
   a,b=self.text(self.arg()),self.text(self.arg(1));assert a.isascii()and b.isascii()
   self.ret(value=0 if a.lower()==b.lower()else 1)
  elif self.loading_type and pc in(0x5b5620,0x5b54d0):
   field=self.text(self.arg());self.events.append('source:'+field)
   value={'name':'message_types_GENERIC_EVENT_name','icon':'MessageTypeIcons/IconGenericEvent.png','filter':'UIMessageFilterGeneric','message_sound':''}[field]
   slot=self.alloc(4);self.w(slot,self.string(value));self.ret(4,slot)
  elif self.loading_type and pc==0x5b4c30:
   self.events.append('source:'+self.text(self.arg(1)));out=self.arg()
   for i,value in enumerate((0x3f800000,0x3f800000,0x3f800000)):self.w(out+i*4,value)
   self.ret(8,out)
  elif self.loading_type and pc==0x4763c0:self.events.append('resource_path_transport');self.ret(4)
  elif self.loading_type and pc==0x5a77e0:self.events.append('texture_resource_transport');self.ret(value=self.icon)
  else:
   before=u.reg_read(UC_X86_REG_EIP);super().hook(u,pc,size,user)
   seam=u.reg_read(UC_X86_REG_EIP)!=before
  if self.active and 0x400000<=pc<0xb30000:
   raw=bytes(u.mem_read(pc,size)).hex();assert PE0.range(pc,size)==PE1.range(pc,size),hex(pc)
   (self.seams if seam else self.executed)[f'{pc:08x}']=raw
  if self.active and not seam:
   if pc==0x40b810:self.events.append('view')
   if pc==0x40b6fb:self.events.append('app_kind')
   if pc==0x40b736:self.events.append('maximum')
 def ecx(self):return self.u.reg_read(UC_X86_REG_ECX)
 def snapshot(self):
  messages=[];node=self.r(self.system+0x28);seen=set()
  while node:
   assert node not in seen;seen.add(node);message=self.r(node)
   messages.append({'id':self.r(message+0xc),'ordinal':15 if self.r(message+8)==self.ty else None,
                    'time':self.r(message+0x10),'view':self.r(message+0x14),'text':self.units(self.r(message+0x18))})
   node=self.r(node+4)
  return {'count':self.r(self.system+0x30),'generation':self.r(self.system+0x34),'last':self.r(self.system+0x38),'messages':messages}
 def send(self,inp):
  self.inp=inp;self.events=[];self.stop_reason=None
  self.w(self.ty+0x1c,self.string('UIMessageFilterGeneric'));self.w(self.ty+0x30,0)
  self.w(self.filter,inp['filter']);self.w(self.maximum,inp['maximum']);self.w(0xb396c0,inp['app'])
  self.w(0xb36538,self.view if inp['view'] is not None else 0)
  self.w(self.view+0x8c,inp['view'] or 0)
  message=self.alloc(0x34);self.active=True
  text=self.alloc(4);self.w(text,self.string(inp['text']))
  self.call(0x40b840,[15,text],this=message,callee_pop=8);self.w(message+4,1)
  header={'time':self.r(message+0x10),'view':self.r(message+0x14)}
  self.w(self.stack,self.stop);self.w(self.stack+4,message);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.system)
  self.u.emu_start(0x40b5c0,self.stop,count=500000)
  if self.stop_reason is None:assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  accepted=None if self.stop_reason else bool(self.u.reg_read(UC_X86_REG_EAX)&255)
  self.active=False
  return {'events':self.events,'header':header,'accepted':accepted,'stop':self.stop_reason,'system':self.snapshot()}
 def predefined_types(self):
  #Native48DB60's exact MessageType precreation loop, not a guessed mapping
  #from source root order. Enum CString data comes from actual497BA0 above.
  array=self.alloc(39*4)
  for row in self.steps:
   obj=self.alloc(12);self.w(obj,row['ordinal']);self.w(obj+4,self.string(row['name']));self.w(array+4*row['ordinal'],obj)
  self.w(0xb3b09c,array);self.active=True
  self.call(0x480c30,[26,2],this=self.gd+0x4c8,callee_pop=8)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_EBP,self.gd)
  self.u.emu_start(0x48ddc5,0x48deaa,count=1000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x48deaa
  self.active=False;self.types=self.r(self.gd+0x4ec);self.ty=self.r(self.types+15*4)
  output=[]
  assert self.r(self.gd+0x4c8+0x28)==39
  for ordinal,row in enumerate(self.steps):
   obj=self.r(self.types+ordinal*4)
   assert self.r(obj)==0xa0b76c and self.r(obj+0xc)==ordinal and self.text(self.r(obj+8))==row['name']
   output.append({'ordinal':self.r(obj+0xc),'ids':self.text(self.r(obj+8)),'vtable':hex(self.r(obj))})
  return output
 def source_type(self):
  #Real48D0A0 is supplied typed getter values; parser/template correctness is
  #separate aa-rules proof, and texture allocation is explicitly transported.
  self.events=[];self.icon=self.alloc(4);target=self.ty
  self.loading_type=True;self.active=True;self.call(0x48d0a0,[self.alloc(4)],this=target,callee_pop=4)
  self.active=False;self.loading_type=False
  return {'events':self.events,'filter':self.text(self.r(target+0x1c)),'sound_is_null':self.r(target+0x30)==0,
          'loaded':self.u.mem_read(target+0x14,1)[0],'color':[self.r(target+o)for o in(0x20,0x24,0x28,0x2c)]}
def inputs():
 cases=[]
 for milliseconds in(0,1,0x1000001,0x7fffffff,0x80000001,0xffffffff):
  for view in(None,0x80000000):
   for filter in(0,128):cases.append(dict(milliseconds=milliseconds,view=view,filter=filter,maximum=50,app=2,text='native message',stop=None))
 cases.extend([dict(milliseconds=0xffffffff,view=0xdeadbeef,filter=255,maximum=max_,app=app,text='limit',stop=stop)
               for max_,app,stop in[(0,1,None),(-1,2,None),(1,2,None),(50,2,'diagnostic')]])
 return cases
def main():
 rows=[];enum=[];constructors=[];types=[];predefined=[];pins={};seams={}
 for image in('original','v14'):
  for p,cw in PC.items():
   m=MessageMachine(image);m.u.reg_write(UC_X86_REG_FPCW,cw);m.active=True;m.call(0x497ba0,this=m.alloc(4));m.active=False
   enum.append({'image':image,'precision':p,'entries':m.steps});constructors.append({'image':image,'precision':p,'output':m.ctor})
   predefined.append({'image':image,'precision':p,'definitions':m.predefined_types()})
   types.append({'image':image,'precision':p,'output':m.source_type()})
   for inp in inputs():
    #A new native list constructor per independent branch.
    m.active=True;m.call(0x40b510,this=m.system);m.active=False
    rows.append({'image':image,'precision':p,'input':inp,'output':m.send(inp)})
   #A real one-owner sequence exercises shrinking max and caught-prefix count.
   m.active=True;m.call(0x40b510,this=m.system);m.active=False
   seq=[]
   for maximum,stop in[(5,None),(5,None),(5,None),(1,None),(0,None),(10,'diagnostic'),(10,None)]:
    inp=dict(milliseconds=100,view=99,filter=1,maximum=maximum,app=2,text='sequence',stop=stop)
    seq.append({'input':inp,'output':m.send(inp)})
   rows.append({'image':image,'precision':p,'sequence':seq})
   pins.update(m.executed);seams.update(m.seams)
 for a,b in zip(rows[:len(rows)//2],rows[len(rows)//2:]):
  assert {k:v for k,v in a.items()if k!='image'}=={k:v for k,v in b.items()if k!='image'}
 out={'scope':'Actual40B510/40B840/40B7C0/40B5C0 including native list helpers;497BA0 enum;480C30 and48DDC5..48DEAA MessageType precreation with actual4746B0/4B4E60, then real48D0A0 on SAME ordinal15. Allocation/CString platform/CRT ASCII compare/lock/atomic and typed source/UVar reads are named transport seams; diagnostic cutoff preserves list prefix. Not whole UI/printf/texture readiness.',
      'constructors':constructors,'enums':enum,'predefined_type_rows':predefined,'source_type_field_rows':types,'rows':rows,'executed_pins':pins,'seam_entries_not_executed':seams}
 data=json.dumps(out,indent=2,sort_keys=True)+'\n';path=HERE/'native-ui-messages.json'
 if '--write' in sys.argv:path.write_text(data,encoding='utf-8',newline='\n')
 else:assert path.read_text(encoding='utf-8')==data,'immutable message replay differs'
 print(f'{len(rows)} rows, {len(pins)} genuine executed pins PASS')
if __name__=='__main__':main()
