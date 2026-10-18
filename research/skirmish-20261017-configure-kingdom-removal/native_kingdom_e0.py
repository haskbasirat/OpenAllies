"""Full Kingdom4E6BC0/4E57C0 destructor with owned positive Commander/table storage.

Laboratory owned layouts exercise real CRT vector destruction, tactical
hash buckets, strategic allocation, Commander summoned-list release, recruitment
hash storage, CString manager transport, actual Updatable destructor and shared
Actor/Object/Thing destructors. General free and CString pool are transport
leaves; positive gameplay callbacks stop at entry. This is not native mission
initialization or Nation selection. No source asset writes occur.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-map-replacement-drain'))
from native_detach_destructor import Destruction,setup,Stop,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP

class Kingdom(Destruction):
 def hook(self,u,pc,size,user):
  if getattr(self,'kingdom_ready',False):
   if pc in [0x4ce310,0x4e29b0,0x50b7a0,0x4d1b80]:self.steps.append(hex(pc))
   if pc==0x6f7430:
    self.frees.append(self.labels.get(self.arg(),'unlabelled'));self.ret();return
  super().hook(u,pc,size,user)
 def own(self,n,label):
  p=self.alloc(n);self.labels[p]=label;return p
 def table(self,p,count,label,entries=0):
  a=self.own(count*4,label+'.buckets');self.w(p,a);self.w(p+4,count);self.w(p+8,entries);self.b(p+12,1)
  if entries:
   for i in range(entries):
    n=self.own(12,label+f'.node{i}');self.w(n,100+i);self.w(n+4,200+i);self.w(n+8,self.r(a));self.w(a,n)
 def commander(self,c):
  commander=self.own(0x1c,'commander');self.w(self.actor+0x230,commander)
  counts=c.get('summoned',[0,0,0]);array=self.own(len(counts)*4 or 4,'ability.vector');self.w(commander+0x10,array);self.w(commander+0x14,len(counts));self.w(commander+0x18,len(counts)+2)
  for i,count in enumerate(counts):
   if count is None:continue
   a=self.own(0x2c,f'ability{i}');self.w(array+i*4,a);head=0;tail=0
   for j in reversed(range(count)):
    n=self.own(8,f'summoned{i}.{j}');self.w(n,0x10007);self.w(n+4,head);head=n
    if not tail:tail=n
   self.w(a+0x24,head);self.w(a+0x28,tail)

def run(m,pc,c):
 m.kingdom_ready=False;setup(m,pc,{})
 m.labels={m.actor:'kingdom'};m.frees=[];m.steps=[]
 mapowner=m.alloc(0x50);m.w(0xb3652c,mapowner)
 m.w(m.actor+0x150,9)
 for off in [0x18,0xdc,0xe0,0x194,0x198,0x240,0x248,0x348]:m.cs(m.actor+off,'owned-lab-text')
 for off,cap in [(0x1a0,8),(0x1ac,8),(0x1b8,8),(0x1c4,8),(0x1d0,32),(0x1dc,8),(0x214,10),(0x24c,1),(0x258,8),(0x2c4,10),(0x2d0,3),(0x2dc,5),(0x2e8,20),(0x300,5),(0x318,5),(0x34c,1)]:
  p=m.own(cap*4,f'vector{off:x}');m.w(m.actor+off,p);m.w(m.actor+off+8,cap)
 m.table(m.actor+0x35c,10,'counts',c.get('counts',0))
 m.table(m.actor+0x204,57,'recruitment',c.get('recruitment',0))
 if c.get('recruitment',0):
  n=m.r(m.r(m.actor+0x204))
  while n:
   m.w(n+4,m.own(0xc,'plain.recruit'));n=m.r(n+8)
 if c.get('strategic',True):m.w(m.actor+0x330,m.own(66*4,'strategic'))
 if c.get('tactical',True):
  tactical=m.own(0xf0,'tactical');m.w(m.actor+0x344,tactical)
  for group in range(10):m.table(tactical+group*16,97 if group<5 else 13,f'tactical{group}',c.get('table_nodes',0))
 if c.get('commander',True):m.commander(c)
 # Caller has already completed F0, retired actual global GTID and detached
 # same grid. Native E0 only owns allocation/list storage from this point.
 m.kingdom_ready=True;error=None
 try:m.call(0x4e6bc0,[c.get('delete',1)],this=m.actor,callee_pop=4)
 except Stop:error=m.trace[-1]['call']
 return dict(error=error,steps=m.steps,frees=m.frees,words={hex(o):m.r(m.actor+o)for o in [0x230,0x330,0x340,0x344,0x148,0x14c,0x150]},vector_headers={hex(o):[m.r(m.actor+o+j)for j in [0,4,8]]for o in [0x1a0,0x1ac,0x1b8,0x1c4,0x1d0,0x1dc,0x214,0x24c,0x258,0x2c4,0x2d0,0x2dc,0x2e8,0x300,0x318,0x34c]}),m.actual.copy()

def main():
 cases=[{},dict(commander=False,tactical=False,strategic=False),dict(summoned=[2,None,1]),dict(summoned=[]),dict(counts=3,table_nodes=2),dict(delete=0),dict(recruitment=5)]
 rows=[];pins={}
 for image in ['original','v14']:
  m=Kingdom(image)
  for pc in PC:
   for c in cases:
    output,visited=run(m,pc,c);pins.update(visited);rows.append(dict(image=image,precision=pc,input=c,output=output))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 p=HERE/'native-kingdom-e0.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
