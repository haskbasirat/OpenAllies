"""Whole7747F0 -> real773480 CString order construction.76CA70 is a named
request transport recording the actual object fields/player argument, not a
network/Host/gameplay success. Native count clears only after all transports.
Required transport stops are not C++ SEH and do not certify native unwinding.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/resource-work-depot'))
from native_work_depot import Machine as Base,base
from unicorn.x86_const import UC_X86_REG_ECX
class Required(Exception):pass
class Machine(Base):
 def __init__(self,image):self.flush=False;super().__init__(image);self.flush=True
 def hook(self,u,pc,size,user):
  if self.flush and pc==0x76ca70:
   if self.case.get('stop_emit')==len(self.emits)+1:raise Required('emit')
   order=u.reg_read(UC_X86_REG_ECX)
   self.emits.append(dict(texts=[self.rs(order+off)for off in(4,8,12)],flag=self.rb(order+16),player_argument=self.arg()))
   self.ret(4);return
  super().hook(u,pc,size,user)
 def run(self,case,precision):
  self.flush=False;self.fpu(precision);self.begin(1,list(range(16)),0);self.watch=True;self.case=case;self.emits=[]
  entries=case['entries'];data=self.alloc(12*max(1,len(entries)))
  for i,entry in enumerate(entries):
   for j,text in enumerate(entry):self.cs(data+i*12+j*4,text)
  self.w(self.S+4,data);self.w(self.S+8,len(entries));self.w(self.S+12,max(4,len(entries)))
  self.actual.clear();self.flush=True;stop=None
  try:self.call(0x7747f0,this=self.S)
  except Required as e:stop=str(e)
  self.flush=False
  return dict(input=case,precision=precision,stop=stop,emits=self.emits,count=self.r(self.S+8))
def produce():
 rows=[];pins={};entries=[['Kingdom'+str(i),'AI\u03bb'+str(i),'ai_easy']for i in range(7)]
 cases=[dict(entries=entries[:n])for n in(0,1,2,7)]+[dict(entries=entries,stop_emit=n)for n in(1,3,7)]
 for image in('original','v14'):
  m=Machine(image)
  for precision,case in itertools.product(base.PC,cases):rows.append(dict(image=image,**m.run(case,precision)));pins.update(m.actual)
 a=[r for r in rows if r['image']=='original'];b=[r for r in rows if r['image']=='v14']
 for x,y in zip(a,b):assert{k:v for k,v in x.items()if k!='image'}=={k:v for k,v in y.items()if k!='image'}
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),cases=rows,executed_windows=base.proof_ranges(pins))
if __name__=='__main__':
 result=produce();p=HERE/'native-setup-ai-flush.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(p.read_text())
 print('passed',len(result['cases']),'rows',len(result['executed_windows']),'windows')
