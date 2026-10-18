"""Original/v14 Loading and UI post-VWorld ordered branches.

UI task enqueue, full VWorld Init/camera/world publication and native status
string transport are explicit required-child seams, not successful real queues
or gameplay acceptance. Dependency stops are pre-instruction/pre-seam cutoffs,
not native SEH exceptions. No original/installed game file is changed.
"""
from pathlib import Path
import sys, json, hashlib, itertools
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine as Base,original,modified,PRECISIONS
from unicorn.x86_const import UC_X86_REG_EIP

class Cutoff(Exception):pass
class Machine(Base):
 def __init__(self,image,precision):
  super().__init__(precision);self.image=image
  if image=='v14':
   old=(original.image_size+0xfff)&~0xfff;new=(modified.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(original.base+old,new-old)
   for vs,rva,raw,off in modified.sections:self.u.mem_write(modified.base+rva,modified.b[off:off+raw])
  self.trace=[];self.cut=None;self.actual={}
 def _hook(self,u,pc,size,data):
  if pc==self.cut:raise Cutoff(hex(pc))
  super()._hook(u,pc,size,data)
  # Base observers run before CPU execution, and observer cutoffs are excluded.
  if pc not in self.seams and u.reg_read(UC_X86_REG_EIP)==pc and original.base<=pc<original.base+original.image_size:
   self.actual[pc]=bytes(u.mem_read(pc,size)).hex()
 def event(self,label,value=None):
  self.trace.append(label if value is None else [label,value])
 def stop_call(self,label,pop=0,value=0):
  def seam(m):m.event(label);m.ret(value,pop)
  return seam

def fields(m,p):return dict(changed=m.rb(p+8),return_state=m.r(p+0xc),kind=m.r(p+0x10),flags=[m.rb(p+x)for x in range(0x14,0x18)])
def fresh(m):
 p=m.alloc(0x18);m.u.mem_write(p,b'\xa5'*0x18);m.call(0x682de0,p);return p
def output(m,p,error):return dict(fields=fields(m,p),trace=m.trace,error=error)
def invoke(m,pc,p,args=()):
 try:m.call(pc,p,args);return None
 except Cutoff:return 'required child cutoff'
def run(m,case):
 m.seams={};m.observers={};m.trace=[];m.cut=None;m.actual={}
 p=fresh(m);m.trace=[]
 # Lab prefills explicitly exercise native branches, not constructor claims.
 op=case['op'];error=None
 if op=='constructor':pass
 elif op=='bind':
  session=m.alloc(0x100);m.w(0xb36550,session);m.wb(session+0xf4,case['multi'])
  if case['fail']:m.cut=0x682e36
  error=invoke(m,0x682e10,p,(case['return_state'],case['kind']))
 elif op=='menu':
  m.wb(p+0x15,case['old']);error=invoke(m,0x682db0,p,(case['argument'],))
 elif op=='event':
  for i,v in enumerate(case['flags']):m.wb(p+0x14+i,v)
  n=m.alloc(0x3c);m.w(n,case['kind'])
  def cstring(x):
   x.event('loading_status_string');a=x.alloc(32);x.w(a+12,3);x.w(x.ecx(),a+16);x.ret(x.ecx(),4)
  m.seam(0x401b00,cstring)
  def status(x):x.event('loading_status');x.ret(0,12)
  m.seam(0x773ce0,status)
  m.seam(0x414620,lambda x:(x.event(['base',case['kind']]),x.ret(0,4)))
  if case['fail']=='status':
   def fail_status(x):x.event('loading_status');raise Cutoff()
   m.seam(0x773ce0,fail_status)
  if case['fail']=='base':
   def fail_base(x):x.event(['base',case['kind']]);raise Cutoff()
   m.seam(0x414620,fail_base)
  error=invoke(m,0x682fd0,p,(n,))
 elif op=='tick':
  for i,v in enumerate(case['flags']):m.wb(p+0x14+i,v)
  m.w(p+0xc,case['return_state']);world=m.alloc(0xb0);session=m.alloc(0x100)
  m.w(0xb36538,world);m.w(0xb36550,session);m.wb(world+0xad,case['view']);m.wb(session+0xf4,case['multi'])
  for pc in [0x682f1f,0x682f90]:m.observe(pc,lambda x:x.event('view',x.rb(world+0xad)!=0))
  m.observe(0x682f2f,lambda x:x.event('multiplayer',x.rb(session+0xf4)!=0))
  def cstring(x):
   a=x.alloc(32);x.w(a+12,3);x.w(x.ecx(),a+16);x.ret(x.ecx(),4)
  m.seam(0x401b00,cstring)
  def status(x):
   x.event('waiting');
   if case['fail']=='waiting':raise Cutoff()
   if case['after_wait'] is not None:x.wb(world+0xad,case['after_wait'])
   x.ret(0,12)
  m.seam(0x773ce0,status)
  def complete(x):
   x.event('complete',x.arg(1)!=0)
   if case['fail']=='complete':raise Cutoff()
   x.ret(0,4)
  m.seam(0x76aca0,complete)
  def state(x):
   x.event('set_state',x.arg(1));assert x.arg(2)==0 and x.arg(3)==0
   if case['fail']=='state':raise Cutoff()
   x.ret(0,12)
  m.seam(0x40ce10,state)
  error=invoke(m,0x682ee0,p)
 elif op=='vworld_status':
  node=m.alloc(0x98);m.w(node+0x90,case['old']);m.w(0xb36558,node)
  def diag(x):
   kind='session_diagnostic'if x.arg(3)==0x5dc else 'admin_diagnostic';x.event(kind)
   if case['fail']==kind:raise Cutoff()
   x.ret()
  m.seam(0x404f60,diag)
  def emit(x):
   x.event('emit',x.r(x.ecx()+4))
   if case['fail']=='emit':raise Cutoff()
   x.ret(0,4)
  m.seam(0x74e970,emit)
  error=invoke(m,0x76aca0,0,(case['success'],))
  return dict(status=m.r(node+0x90),trace=m.trace,error=error),m.actual
 elif op=='ui_init':
  ui=m.alloc(0x1e8);world=m.alloc(0x180);session=m.alloc(0x180);manager=m.alloc(0x40);camera=m.alloc(4)
  lock=m.alloc(12);table=m.alloc(12);m.w(lock+8,table)
  m.w(table+4,m.stub(lambda x:(x.event('lock'),x.ret())))
  def unlock(x):
   x.event('unlock')
   if case['fail']=='unlock':raise Cutoff()
   x.ret()
  m.w(table+8,m.stub(unlock));m.w(0xb36524,lock)
  m.w(0xb36534,world);m.w(0xb36550,session);m.w(0xb36540,manager);m.w(0xb40048,camera);m.w(camera,0x12341234)
  m.w(ui+0x1e0,case['old']);m.wb(ui+0x1e4,0xa5);m.w(ui+0xd0,world)
  m.seam(0x59aba0,lambda x:x.ret(256))
  def required(label,pop=0):
   def effect(x):
    x.event(label)
    if case['fail']==label:raise Cutoff()
    x.ret(0,pop)
   return effect
  m.seam(0x404f60,required('diagnostic'))
  m.queue=list(case['kingdoms'])
  def kingdom(x):
   v=m.queue.pop(0);x.event('kingdom',v)
   if case['fail']=='kingdom'+str(2-len(m.queue)):raise Cutoff()
   x.ret(v)
  m.seam(0x76c2b0,kingdom)
  m.seam(0x700e30,required('camera_reset'))
  def init(x):
   assert x.ecx()==world;x.event('view_init',x.arg(1))
   if case['fail']=='view_init':raise Cutoff()
   x.ret(1,4)
  m.seam(0x43f0a0,init)
  def zero(x):
   assert x.arg(1)==0 and x.r(x.arg(2))==0 and x.r(x.arg(2)+4)==0
   x.event('camera_zero')
   if case['fail']=='camera_zero':raise Cutoff()
   x.ret(0,8)
  m.seam(0x700500,zero)
  m.seam(0x4f0750,required('world_onview',4));m.task=0
  def alloc(x):
   assert x.arg(1)==0x1c;m.task=x.alloc(0x1c);x.u.mem_write(m.task,b'\xa5'*0x1c);x.ret(m.task)
  m.seam(0x6f7810,alloc)
  def enqueue(x):
   assert x.ecx()==manager and x.arg(1)==m.task;x.event('enqueue')
   if case['fail']=='enqueue':raise Cutoff()
   x.ret(0,4)
  m.seam(0x5aed10,enqueue)
  error=invoke(m,0x5c3000,ui,(case['revision'],))
  return dict(has_kingdom=m.rb(ui+0x1e4),kingdom=m.r(ui+0x1e0),
   task_fields=None if not m.task else[m.r(m.task+off)for off in range(4,0x1c,4)],task_vtable=None if not m.task else m.r(m.task),trace=m.trace,error=error),m.actual
 else:raise AssertionError(op)
 return output(m,p,error),m.actual

def cases():
 yield dict(op='constructor')
 for state,kind,multi,fail in itertools.product([0,5,6,7,14,0xffffffff],[0,1,0xffffffff],[0,1,0xa5],[False,True]):
  yield dict(op='bind',return_state=state,kind=kind,multi=multi,fail=fail)
 for old,arg in itertools.product([0,1,0xa5],[0,1,0x100,0x101,0xffffffff]):yield dict(op='menu',old=old,argument=arg)
 for kind,flags,fail in itertools.product([0,1,9,99],[[0,0,0,0],[0xa1,0xa2,0xa3,0xa4]],[None,'status','base']):
  yield dict(op='event',kind=kind,flags=flags,fail=fail)
 for flags,view,multi in itertools.product(itertools.product([0,1],repeat=4),[0,1],[0,1]):
  yield dict(op='tick',flags=list(flags),return_state=12,view=view,multi=multi,after_wait=None,fail=None)
 for fail,after in itertools.product([None,'waiting','complete','state'],[0,1]):
  yield dict(op='tick',flags=[1,1,1,1],return_state=0xffffffff,view=1,multi=1,after_wait=after,fail=fail)
 for success,old,fail in itertools.product([0,1,0xa5],[0,2,9],[None,'session_diagnostic','admin_diagnostic','emit']):
  yield dict(op='vworld_status',success=success,old=old,fail=fail)
 for kingdoms,fail in itertools.product([[0,99],[7,0],[7,9]],[None,'diagnostic','kingdom1','kingdom2','camera_reset','view_init','camera_zero','world_onview','enqueue','unlock']):
  yield dict(op='ui_init',old=0x55667788,kingdoms=kingdoms,revision=1,fail=fail)

def main():
 rows=[];pins={};inputs=list(cases())
 for image in ['original','v14']:
  for precision in PRECISIONS:
   m=Machine(image,precision)
   for case in inputs:
    out,actual=run(m,case);pins.update(actual);rows.append(dict(image=image,precision=precision,input=case,output=out))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 for va,raw in pins.items():assert bytes.fromhex(raw)==original.range(va,len(bytes.fromhex(raw)))==modified.range(va,len(bytes.fromhex(raw)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())])
 path=HERE/'native-loading.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),inputs=len(inputs),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
