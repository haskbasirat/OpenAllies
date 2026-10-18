"""Actual428A30 recursive root/immediate/chain short circuit, executing428360.
FXC virtual24 is a constructor-kind service; actual Data24 and FXC64 are
bounded explicit fields. Source permanent/duration production has its prior
separate native proof. No type source/renderer/runtime effects are invented.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
class Required(Exception):pass
KINDS=['stop','play','create','create','stop','create','play','create']
IMMEDIATE=[[2],[4],[],[],[6],[],[],[]];CHAIN=[[3],[5],[],[],[],[],[7],[]]
def cases():
 for target,duration in itertools.product([-1,2,3,5,7],[False,True]):
  for stop in [None,2,7]:yield dict(target=target,duration=duration,stop=stop)
 yield dict(target=-1,duration=False,stop=None,empty=True)
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 trace=[];nodes=[m.alloc(0x70)for _ in KINDS];indices={p:i for i,p in enumerate(nodes)}
 def pair(ids):
  head=0;tail=0
  for i in reversed(ids):
   link=m.array([nodes[i],head]);head=link
   if not tail:tail=link
  return [head,tail]
 for i,p in enumerate(nodes):
  vt=m.alloc(0x28);m.w(p,vt)
  def kind(mm,i=i):trace.append(['kind',i]);mm.ret(int(KINDS[i]=='create'))
  m.w(vt+0x24,m.stub(kind));m.w(vt+0x20,0x428360)
  source=m.alloc(0x28);m.w(p+0x3c,source);m.w(source+0x24,1 if c['target']==i else 3);m.wb(p+0x64,c['duration'])
  for off,ls in [(8,IMMEDIATE[i]),(0x10,CHAIN[i])]:
   x=pair(ls);m.w(p+off,x[0]);m.w(p+off+4,x[1])
 def active(mm):
  i=indices[mm.ecx()];trace.append(['active',i])
  if c['stop']==i:raise Required(str(i))
 m.observe(0x428360,active)
 roots=m.array(pair([]if c.get('empty')else[0,1]));status='ok';result=None
 try:result=bool(m.call(0x428a30,roots,[])&255)
 except Required as e:status='required_'+str(e)
 return dict(status=status,result=result,trace=trace),m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins)};path=HERE/'native-animation-permanence.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
