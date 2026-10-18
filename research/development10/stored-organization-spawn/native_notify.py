"""Whole578040 guard/callback ordering, with required stats/SDK child seams.

Every scalar read retained by the runtime field kernel is observed in order.
4E14E0,58E870,58EDD0 and7AD8E0 are explicit children, not completed owners.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
OLD=h.P;V14=h.V
class Cutoff(Exception):pass
class M(h.Machine):
 def run(self,c):
  self.trace=[];org=self.alloc(0xdc);actor=self.alloc(0x200);kingdom=self.alloc(0x350)
  member=self.alloc(0x40);other=self.alloc(0x40);vt=self.alloc(0x110);stats=self.alloc(16)
  sai=self.alloc(0x20);controller=self.alloc(16)
  self.w32(org+4,actor);self.w32(org+0x44,member if c['primary'] else other)
  self.w32(actor,vt);self.w32(vt+0x108,h.STUBS+0x100);self.w32(vt+0x20,h.STUBS+0x200)
  self.w32(actor+0xd8,0x12345678 if c['resource'] else 0)
  self.w32(kingdom+0x32c,stats if c['stats'] else 0)
  self.w32(0xb36528,controller if c['sdk'] else 0)
  self.w32(actor+0x154,sai if c['sai'] else 0);self.w32(sai+12,controller)
  reads={actor+0xd8:'resource',org+0x44:'primary',kingdom+0x32c:'stats',0xb36528:'sdk',actor+0x154:'sai'}
  def read(u,access,address,size,value,_):
   if address in reads:self.trace.append(reads[address])
  self.u.hook_add(UC_HOOK_MEM_READ,read)
  def stop(label):
   if c['fail']==label:raise Cutoff()
  def owner(m):m.trace.append('allegiance');m.ret(value=kingdom if c['owner'] else 0)
  def state(m):assert m.arg(0)==1;m.trace.append('state1');m.ret(4,int(c['state']))
  def refresh(m):assert m.ecx()==kingdom;m.trace.append('refresh');stop('refresh');m.ret()
  def score(m):
   index=m.arg(0);assert m.ecx()==stats and m.arg(1)==0x3f800000
   m.trace.append(['score',index]);stop('score'+str(index));m.ret(8)
  def classified(m):assert m.arg(0)==actor;m.trace.append('classify');stop('classify');m.ret(value=int(c['classified']))
  def notify(m):assert m.ecx()==controller and m.arg(0)==actor;m.trace.append('notify');stop('notify');m.ret(4)
  self.seams[h.STUBS+0x100]=owner;self.seams[h.STUBS+0x200]=state
  self.seams[0x4e14e0]=refresh;self.seams[0x58e870]=score;self.seams[0x58edd0]=classified;self.seams[0x7ad8e0]=notify
  error=None
  try:self.call(0x578040,org,[member])
  except Cutoff:error=c['fail']
  return dict(trace=self.trace,error=error)
def main():
 h.VISITED.clear();rows=[];cases=[]
 sdk=[(False,False),(True,False),(True,True)]
 for d,s in sdk:cases.append(dict(owner=False,resource=False,primary=False,stats=False,state=False,classified=False,sdk=d,sai=s,fail=None))
 for r,p,(t,a,k),(d,s) in itertools.product([False,True],[False,True],[(False,False,False),(True,False,False),(True,True,False),(True,True,True)],sdk):
  cases.append(dict(owner=True,resource=r,primary=p,stats=t,state=a,classified=k,sdk=d,sai=s,fail=None))
 for fail in ['refresh','score8','classify','score11','notify']:
  cases.append(dict(owner=True,resource=True,primary=True,stats=True,state=True,classified=True,sdk=True,sai=True,fail=fail))
 for pc,c in itertools.product(h.PRECISION,cases):
  h.P=OLD;a=M(pc).run(c);h.P=V14;b=M(pc).run(c);assert a==b,(pc,c,a,b)
  rows.append(dict(pc=pc,input=c,output=a))
 h.P=OLD;seams=[0x4e14e0,0x58e870,0x58edd0,0x7ad8e0]
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in seams}
 doc=dict(schema=1,scope=__doc__,rows=rows,pins=pins,excluded_entries=[hex(a)for a in seams],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 raw=(json.dumps(doc,indent=2)+'\n').encode();path=HERE/'native-notify.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),executed_pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
