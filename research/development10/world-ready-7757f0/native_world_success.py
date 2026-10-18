"""7738B0 successful-callback control/numeric/notification prefix, with
explicit clock-return/type-query/request transports. Actual76DCE0,7844F0,
7519C0,7851C0,74FEC0 and empty4F7930 execute. Required stops are not C++ SEH.
Global diagnostics clear4F7910 and zero-tail provenance are independently run.
No genuine task completion, Host activation or positive warning formatting.
"""
from pathlib import Path
import hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/resource-work-depot'))
from native_work_depot import Machine as Base,base
from native_bootstrap import NODE
from unicorn.x86_const import UC_X86_REG_EIP

class Required(Exception):pass
class Machine(Base):
 def __init__(self,image):
  self.success_watch=False;super().__init__(image)
  self.u.mem_write(0x7700d00,b'\xc3')
  self.u.mem_write(0x7700d10,b'\xd9\x05'+struct.pack('<I',0x7700d40)+b'\xc2\x04\x00')
  self.success_watch=True
 def event(self,kind,**fields):
  if self.case.get('stop')==kind:raise Required(kind)
  self.events.append(dict(kind=kind,**fields))
 def hook(self,u,pc,size,user):
  if self.success_watch:
   if pc==0x7700d00:self.ret(value=self.task_type);return
   if pc==0x74b170:
    assert self.arg()==0;self.event('clock',value=self.case['admin'])
    u.reg_write(UC_X86_REG_EIP,0x7700d10);return
   if pc==0x76dce0:self.event('reset',admin=self.arg())
   if pc==0x4f7930:self.event('warnings')
   if pc==0x7851c0:self.event('notify')
   if pc==0x74fec0:self.event('task_success',value=self.arg())
   if pc==0x74e970:self.event('request',status=self.r(NODE+0x90),player=self.arg());self.ret(4);return
  super().hook(u,pc,size,user)
 def setup(self,case,pc):
  self.case=case;self.events=[];self.fpu(pc);self.begin(1,list(range(16)),case['application'])
  self.watch=True;self.w(0xb36558,NODE)
  self.world=self.alloc(0x2a0);self.w(0xb36534,self.world);self.w(self.world+0xe0,case['begin'])
  self.w(0xb3cc94,case['revision']);self.b(self.sim+0x28,case['paused'])
  self.w(NODE+0x90,0xdeadbeef);self.b(NODE+0x8c,case['required'])
  self.cs(0xb3cc88,'previous');self.w(0xb3cc8c,0);self.w(0xb3cc90,0)
  self.call(0x4f7910)
  task=self.alloc(8);vt=self.alloc(12);self.w(task,vt);self.w(vt+8,0x7700d00);self.w(self.S+0x124,task)
  self.task_type=self.alloc(12);self.w(self.task_type+8,0xb3dac0 if case['save'] else 0)
  self.w(0xb3dac0+8,0)
  self.w(0x7700d40,case['admin'])
  #Actual notification copy and queue, with named descriptor-string metadata.
  descriptors=self.alloc(5*4)
  for kind in range(5):
   descriptor=self.alloc(8);self.cs(descriptor+4,'FixtureNotification');self.w(descriptors+4*kind,descriptor)
  self.w(0xb40bb0,descriptors);self.events=[];self.notification=[];self.actual.clear()
 def run(self,case,pc):
  self.setup(case,pc);stop=None
  try:self.call(0x7738b0,this=self.S)
  except Required as error:stop=str(error)
  except Exception as error:raise AssertionError((case,pc,hex(self.u.reg_read(UC_X86_REG_EIP)),self.events))from error
  for event in self.events:
   if event['kind']=='reset':assert event['admin']==case['admin'],(event,case)
  if not case['save'] and stop not in('clock','reset'):
   assert self.r(self.sim+0x18)==case['begin'] and self.r(self.sim+0x1c)==case['admin']
  return dict(input=case,precision=pc,stop=stop,events=self.events,notifications=self.notification,
   times=[self.r(self.sim+off)for off in(0x10,0x14,0x18,0x1c)],maximum=self.r(self.sim+0x24),
   paused=self.rb(self.sim+0x28),status=self.r(NODE+0x90))

def produce():
 rows=[];pins={};clears=[];tails=[]
 for image in('original','v14'):
  m=Machine(image)
  pe=base.PE0 if image=='original'else base.PE1
  for vs,r,raw,off in pe.sections:
   word=0xb3cc8c-pe.base-r
   if 0<=word<vs:
    assert raw<=word and word+8<=vs;tails.append(dict(image=image,section_rva=r,raw_size=raw,word_offset=word,virtual_size=vs))
  for pc in base.PC:
   #Use actual list append/CString nodes, then actualclear and empty-reader.
   m.fpu(pc);m.begin(1,list(range(16)),0);m.watch=True;m.case={};m.events=[]
   m.cs(0xb3cc88,'previous resource');m.w(0xb3cc8c,0);m.w(0xb3cc90,0)
   for text in('first','second'):
    value=m.alloc(4);m.cs(value,text);m.call(0x4f97b0,[value],this=0xb3cc8c,callee_pop=4)
   m.actual.clear();m.call(0x4f7910)
   assert m.rs(0xb3cc88)==[]and m.r(0xb3cc8c)==m.r(0xb3cc90)==0
   destination=m.alloc(4);m.call(0x4f7930,[destination]);assert m.rs(destination)==[]
   clears.append(dict(image=image,precision=pc,name=m.rs(0xb3cc88),head=m.r(0xb3cc8c),tail=m.r(0xb3cc90),text=m.rs(destination)))
   pins.update(m.actual)
   cases=[dict(save=save,application=app,required=req,paused=0xa5,admin=admin,begin=begin,revision=0xffffffff)
    for save,app,req,(admin,begin)in itertools.product((False,True),(0,3),(0,1,128),((0x3eaaaaab,0x41200000),(0x4b800001,0x80000000)))]
   cases += [dict(save=False,application=0,required=1,paused=0xa5,admin=0x3eaaaaab,begin=0x41200000,revision=7,stop=stop)
     for stop in('clock','reset','warnings','notify','task_success','request')]
   for case in cases:rows.append(dict(image=image,**m.run(case,pc)));pins.update(m.actual)
 for a,b in zip([r for r in rows if r['image']=='original'],[r for r in rows if r['image']=='v14']):
  assert {k:v for k,v in a.items()if k!='image'}=={k:v for k,v in b.items()if k!='image'}
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=hashlib.sha256(base.PE0.b).hexdigest(),v14_sha256=hashlib.sha256(base.PE1.b).hexdigest(),
  cases=rows,diagnostics_clear=clears,zero_tails=tails,executed_windows=base.proof_ranges(pins))
if __name__=='__main__':
 result=produce();target=HERE/'native-world-success.json'
 if '--write'in sys.argv:target.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(target.read_text())
 print('passed',len(result['cases']),'7738B0 cases,',len(result['diagnostics_clear']),'diagnostic clears')
