"""Native752FC0 ->74DA50 ->750940/750990 ->776560 ->74DAA0.
The real ordering-player+4 lookup, linked traversal, raw-DWORD history reset
and per-peer/current-view stores execute.775BF0/773AE0 are explicit required
completion leaves; their successful lab response is not application readiness.
No packet-history alias or actual network peers are claimed by lab objects.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/resource-work-depot'))
from native_work_depot import Machine as Base,base
from native_bootstrap import NODE
from unicorn.x86_const import UC_X86_REG_EIP
class Required(Exception):pass
class Machine(Base):
 def __init__(self,image):
  self.protocol=False;super().__init__(image);self.protocol=True
 def event(self,kind,**fields):
  if self.case.get('stop')==kind:raise Required(kind)
  self.events.append(dict(kind=kind,**fields))
 def hook(self,u,pc,size,user):
  if self.protocol:
   if pc in (0x775bf0,0x773ae0):
    self.event('complete',success=pc==0x775bf0);self.ret();return
   if pc==0x750940:self.event('clear_history')
   if pc==0x750990:self.event('retire_history',value=self.arg())
   if pc==0x74daa0:self.event('clear_tasks')
  super().hook(u,pc,size,user)
 def run(self,case,precision):
  self.case=case;self.events=[];self.fpu(precision);self.begin(1,list(range(16)),0);self.watch=True
  peers=[self.alloc(0x88)for _ in case['peers']]
  for p,s in zip(peers,case['peers']):self.w(p+0x34,s)
  prior=0
  for p in reversed(peers):
   item=self.alloc(8);self.w(item,p);self.w(item+4,prior);prior=item
  self.w(NODE+0x2ac,prior);self.w(NODE+0x2b0,0);self.b(NODE+0x8c,0xa5)
  target=peers[case['target']] if peers else self.alloc(0x88)
  self.w(target+0x34,0xbad0);player=self.alloc(8);self.w(player+4,target);self.w(NODE+0x284,player)
  begin,end=case['range'];self.w(NODE+0x238,begin&0xffffffff);self.w(NODE+0x23c,end&0xffffffff)
  self.w(NODE+0x240,7);cells=self.alloc(28);self.w(NODE+0x244,cells)
  for i in range(7):self.w(cells+4*i,0x12340000+i)
  self.actual.clear();self.events=[];stop=None
  try:self.call(0x752fc0,[case['status']],this=NODE,callee_pop=4)
  except Required as e:stop=str(e)
  return dict(input=case,precision=precision,stop=stop,events=self.events,
   peers=[self.r(p+0x34)for p in peers],target=self.r(target+0x34),required=self.rb(NODE+0x8c),
   history=[self.r(NODE+0x238),self.r(NODE+0x23c)],cells=[self.r(cells+4*i)for i in range(7)])
def produce():
 rows=[];pins={}
 cases=[dict(peers=list(peers),target=target,status=status,range=[-3,4])
  for peers in ([],[0],[3],[4],[0,3],[3,2],[4,3],[7,3,4],[3,3,3])
  for target in range(max(len(peers),1))for status in(0,1,2,3,4,7)]
 cases += [dict(peers=[3,3],target=0,status=3,range=[-3,4],stop=stop)
  for stop in('clear_history','retire_history','complete','clear_tasks')]
 for image in('original','v14'):
  m=Machine(image)
  for precision,case in itertools.product(base.PC,cases):rows.append(dict(image=image,**m.run(case,precision)));pins.update(m.actual)
 a=[r for r in rows if r['image']=='original'];b=[r for r in rows if r['image']=='v14']
 assert len(a)==len(b)
 for x,y in zip(a,b):assert {k:v for k,v in x.items()if k!='image'}=={k:v for k,v in y.items()if k!='image'}
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=hashlib.sha256(base.PE0.b).hexdigest(),v14_sha256=hashlib.sha256(base.PE1.b).hexdigest(),
  cases=rows,executed_windows=base.proof_ranges(pins))
if __name__=='__main__':
 result=produce();p=HERE/'native-task-protocol.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(p.read_text())
 print('passed',len(result['cases']),'rows',len(result['executed_windows']),'windows')
