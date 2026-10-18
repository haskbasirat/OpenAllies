"""Whole native76BA90 human-opposition and765E00 current-World getter.
Typed laboratory SessionPlayer/Kingdom/Team objects are explicit inputs;
no native map/bootstrap/AI activation or unbound pointer success is claimed.
Getter-required stops are Rust source-binding seams, not native AV/SEH.
"""
from pathlib import Path
import hashlib,itertools,json,random,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/resource-work-depot'))
from native_work_depot import Machine as Base,base
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_EIP
class Required(Exception):pass
class Machine(Base):
 def __init__(self,image):self.opposition=False;super().__init__(image);self.opposition=True
 def hook(self,u,pc,size,user):
  if self.opposition and pc==0x765e00:
   if self.case.get('stop_getter')==len(self.getters)+1:raise Required('getter')
   self.getters.append(self.player_ids[u.reg_read(UC_X86_REG_ECX)])
  super().hook(u,pc,size,user)
 def run(self,case,precision):
  self.opposition=False;self.fpu(precision);self.begin(1,list(range(16)),0);self.watch=True
  self.case=case;self.getters=[];self.player_ids={};self.w(0xb36534,self.alloc(4)if case['world']else 0)
  teams=[0]+[self.alloc(4)for _ in range(3)];kingdoms=[]
  for k in case['kingdoms']:
   p=self.alloc(0x240);self.w(p+0x19c,k['result']);self.w(p+0x1ec,teams[k['team']]);kingdoms.append(p)
  tail=0
  for i,p in reversed(list(enumerate(case['players']))):
   actor=self.alloc(0x60);self.player_ids[actor]=i;self.b(actor+0xc,p['ai']);self.w(actor+0x28,0 if p['kingdom']is None else kingdoms[p['kingdom']])
   item=self.alloc(8);self.w(item,actor);self.w(item+4,tail);tail=item
  self.w(self.S+0xbc,tail);self.actual.clear();self.opposition=True;stop=None;value=None
  try:value=bool(self.call(0x76ba90,this=self.S)&255)
  except Required as e:stop=str(e)
  self.opposition=False
  return dict(input=case,precision=precision,stop=stop,result=value,getters=self.getters)
def produce():
 r=random.Random(0x76ba90);cases=[]
 for i in range(120):
  kingdoms=[dict(result=r.randrange(3),team=r.randrange(4))for _ in range(4)]
  players=[dict(ai=bool(r.randrange(2)),kingdom=r.choice([None,0,1,2,3]))for _ in range(i%6)]
  cases.append(dict(world=bool(i%7),kingdoms=kingdoms,players=players))
 for team_a,team_b in itertools.product(range(3),repeat=2):
  cases.append(dict(world=True,kingdoms=[dict(result=0,team=team_a),dict(result=0,team=team_b)],players=[dict(ai=False,kingdom=0),dict(ai=False,kingdom=1)]))
 for stop in(1,2,3,4,5,7,8):
  cases.append(dict(world=True,kingdoms=[dict(result=0,team=0),dict(result=0,team=0)],players=[dict(ai=False,kingdom=0),dict(ai=False,kingdom=1)],stop_getter=stop))
 rows=[];pins={}
 for image in('original','v14'):
  m=Machine(image)
  for precision,case in itertools.product(base.PC,cases):rows.append(dict(image=image,**m.run(case,precision)));pins.update(m.actual)
 a=[r for r in rows if r['image']=='original'];b=[r for r in rows if r['image']=='v14']
 for x,y in zip(a,b):assert{k:v for k,v in x.items()if k!='image'}=={k:v for k,v in y.items()if k!='image'}
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),cases=rows,executed_windows=base.proof_ranges(pins))
if __name__=='__main__':
 result=produce();p=HERE/'native-human-opposition.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(p.read_text())
 print('passed',len(result['cases']),'rows',len(result['executed_windows']),'windows')
