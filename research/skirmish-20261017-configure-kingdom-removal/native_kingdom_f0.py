"""Full4E4130 prefix through real79F590/79EEC0,50B250,4F1430,4F09C0.

Stops at actual4D7580 entry: shared Actor uninit/destructor are not supplied
as success. Explicit class-list children and positive scheduled head stop at
entry. Session BC is an explicit empty laboratory roster; the separate54-row
session fixture proves nonempty scan/positive clear call ordering. Heap free
is a transport leaf. Inputs are laboratory layouts, including an actual1677
index/count subset, not an initialized native game capture.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
class Stop(Exception):pass
class Kingdom(Machine):
 def __init__(self,image):self.ready=False;super().__init__(image)
 def hook(self,u,pc,size,user):
  if self.ready:
   if pc in [0x79f590,0x4f09c0,0x76c7c0]:self.trace.append(hex(pc))
   if pc==0x4d7580:
    self.trace.append('actor_uninit');assert self.ecx()==self.actor and self.arg()==0;raise Stop()
   if pc==0x4f5490:self.trace.append('class_actor');raise Stop()
   if pc==0x50b250 and self.scheduled:self.trace.append('scheduled');raise Stop()
   if pc==0x6f7430:self.ret();return
  super().hook(u,pc,size,user)
def run(m,pc,c):
 m.ready=False;m.fpu(pc);m.actual={};m.record_actual=True;m.trace=[]
 m.actor=m.alloc(0x400);world=m.alloc(0x400);sai=m.alloc(0xa0);session=m.alloc(0x200)
 m.w(0xb36534,world);m.w(0xb36528,sai);m.w(0xb36550,session)
 #Same fresh manager allocation, actual count0 branch; no AI-off inference.
 m.w(sai+0x64,0);m.b(sai+0x20,0);m.w(session+0xbc,0)
 arrays=[m.alloc(8*4),m.alloc(8*4)];m.w(world+0x148,arrays[0]);m.w(world+0x154,arrays[1]);m.w(world+0x144,c.get('counter',7))
 for off,key,array in [(0x188,'all',arrays[0]),(0x18c,'major',arrays[1])]:
  index=c.get(key,1);m.w(m.actor+off,index&0xffffffff)
  if index>=0:m.w(array+index*4,m.actor)
 m.w(m.actor+0x190,c.get('minor',0))
 colors=m.alloc(8*4);m.w(world+0x160,colors)
 color_index=c.get('color',0)
 if color_index is not None:
  color=m.alloc(0x20);m.w(color+0xc,color_index);m.w(m.actor+0x1e8,color);m.w(colors+color_index*4,1)
 team=m.alloc(0x80);m.w(team+0x68,2);m.w(m.actor+0x1ec,team if c.get('team',True)else 0)
 members=c.get('members',['other','self','self']);nodes=[];head=0
 for value in reversed(members):
  node=m.alloc(8);m.w(node,m.actor if value=='self' else 123);m.w(node+4,head);head=node;nodes.append(node)
 m.w(team+0x44,head);m.w(team+0x48,nodes[0] if nodes else 0)
 m.scheduled=c.get('scheduled',False);m.w(m.actor+0x184,1 if m.scheduled else 0)
 if c.get('class_actor'):m.w(m.actor+0x26c,m.alloc(8))
 error=None;m.ready=True
 try:m.call(0x4e4130,[0],this=m.actor,callee_pop=4)
 except Stop:error=m.trace[-1]
 values=[];node=m.r(team+0x44)
 while node:values.append('self'if m.r(node)==m.actor else'other');node=m.r(node+4)
 output=dict(error=error,trace=m.trace,counter=m.r(world+0x144),indices=[m.r(m.actor+o)for o in [0x188,0x18c]],
  all_slots=['self' if m.r(arrays[0]+i*4)==m.actor else 0 for i in range(8)],major_slots=['self' if m.r(arrays[1]+i*4)==m.actor else 0 for i in range(8)],
  colors=[m.r(colors+i*4)for i in range(8)],team_present=m.r(m.actor+0x1ec)!=0,team_count=m.r(team+0x68),team_values=values,instances_count=m.r(m.actor+0x31c))
 return output,m.actual.copy()
def main():
 cases=[{},dict(all=-1),dict(major=-1),dict(minor=1),dict(team=False),dict(members=[]),dict(members=['other']),dict(color=None),dict(counter=0xffffffff),dict(scheduled=True),dict(class_actor=True)]
 rows=[];pins={}
 for image in ['original','v14']:
  m=Kingdom(image)
  for pc in PC:
   for case in cases:
    output,visited=run(m,pc,case);pins.update(visited);rows.append(dict(image=image,precision=pc,input=case,output=output))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 p=HERE/'native-kingdom-f0.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
