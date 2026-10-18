"""Native775BF0 full ordered parent and actual notification construction/
77D900 S reset.76DD90,76EE10,76BA90,7747F0,76D820 and4F8AE0 are explicitly
named required child transports, not native startup or battle success. Native
770130 executes its nonnull managerBC0 gate. Required stops model unavailable
Rust leaves, not C++ SEH. Child proofs/source binding remain separately required.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/resource-work-depot'))
from native_work_depot import Machine as Base,base
from unicorn.x86_const import UC_X86_REG_ECX
class Required(Exception):pass
class Machine(Base):
 def __init__(self,image):self.tasks=False;super().__init__(image);self.tasks=True
 def event(self,kind,**fields):
  if self.case.get('stop')==kind:raise Required(kind)
  self.events.append(dict(kind=kind,**fields))
 def hook(self,u,pc,size,user):
  if self.tasks:
   if pc==0x404f60:
    ret=self.r(u.reg_read(30))
    if ret in(0x775c2a,0x775ccb,0x775d14):
     self.event({0x775c2a:'diagnostic',0x775ccb:'begin',0x775d14:'opposition_diagnostic'}[ret],**({'value':bool(self.rb(self.S+0x16d))}if ret==0x775d14 else{}));self.ret();return
   if pc==0x7851c0:self.event('notify')
   if pc==0x76dd90:self.event('unfreeze');assert self.arg()==0;self.ret(4);return
   if pc==0x76ee10:self.event('restart');self.ret();return
   if pc==0x76ba90:self.event('opposition');self.ret(value=int(self.case['opposition']));return
   if pc==0x770130:self.event('game_spy') #realBC0 executes
   if pc==0x7747f0:self.event('setup');self.ret();return
   if pc==0x76d820:self.event('svars');assert self.arg()==0xb39aa8;self.ret();return
   if pc==0x4f8ae0:self.event('world',label=self.text(self.arg()));assert self.arg(1)==self.world;self.ret();return
   if pc==0x77d900 and u.reg_read(UC_X86_REG_ECX)==self.sync_s:self.event('reset_s',index=self.arg(),hash=self.arg(1))
  super().hook(u,pc,size,user)
 def run(self,case,precision):
  self.tasks=False;self.fpu(precision);self.begin(1,list(range(16)),case['application']);self.watch=True
  self.case=case;self.events=[];self.w(self.S+0xe4,case['state'])
  for off,v in [(0x16a,case['flag_a']),(0x16b,case['flag_b']),(0x16d,True),(0x16e,True)]:self.b(self.S+off,v)
  self.world=self.alloc(0x10);self.w(0xb36534,self.world)
  spy=self.alloc(0xc0);self.b(spy+0xbc,0);self.w(0xb3654c,spy)
  self.sync_s=self.alloc(0x20);self.call(0x77ccb0,[2],this=self.sync_s,callee_pop=4);self.w(0xb36560,self.sync_s)
  descriptors=self.alloc(40)
  for kind in range(10):
   descriptor=self.alloc(8);self.cs(descriptor+4,'FixtureNotification');self.w(descriptors+4*kind,descriptor)
  self.w(0xb40bb0,descriptors);self.notification=[];self.actual.clear();self.events=[];self.tasks=True;stop=None
  try:self.call(0x775bf0,this=self.S)
  except Required as error:stop=str(error)
  self.tasks=False
  return dict(input=case,precision=precision,stop=stop,events=self.events,notifications=self.notification,
   flags={f'{off:x}':bool(self.rb(self.S+off))for off in(0x16a,0x16b,0x16d,0x16e)},
   s=dict(enabled=bool(self.rb(self.sync_s)),dump=bool(self.rb(self.sync_s+1)),suspend=self.r(self.sync_s+4),hash=self.r(self.sync_s+8),begin=self.r(self.sync_s+0x10),end=self.r(self.sync_s+0x14)))
def produce():
 rows=[];pins={}
 cases=[dict(state=s,flag_a=a,flag_b=b,opposition=h,application=app)
  for s,a,b,h,app in itertools.product((0,1,2,4,5),(False,True),(False,True),(False,True),(0,3))]
 cases +=[dict(state=2,flag_a=True,flag_b=False,opposition=True,application=0,stop=stop)
  for stop in('notify','unfreeze','begin','opposition','opposition_diagnostic','game_spy','setup','svars','world','reset_s')]
 for image in('original','v14'):
  m=Machine(image)
  for precision,case in itertools.product(base.PC,cases):rows.append(dict(image=image,**m.run(case,precision)));pins.update(m.actual)
 a=[r for r in rows if r['image']=='original'];b=[r for r in rows if r['image']=='v14']
 for x,y in zip(a,b):assert{k:v for k,v in x.items()if k!='image'}=={k:v for k,v in y.items()if k!='image'}
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),cases=rows,executed_windows=base.proof_ranges(pins))
if __name__=='__main__':
 result=produce();p=HERE/'native-all-tasks-complete.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(p.read_text())
 print('passed',len(result['cases']),'rows',len(result['executed_windows']),'windows')
