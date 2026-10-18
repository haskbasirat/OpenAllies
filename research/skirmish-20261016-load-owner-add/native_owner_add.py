"""Original4E6180 with actual list/vector/count leaves; explicit required effects."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
P=h.P;V=h.V;H=lambda b:hashlib.sha256(b).hexdigest()
class Stop(Exception):
 def __init__(self,name):self.name=name
class Machine(h.Machine):
 def __init__(self,pc):
  super().__init__(pc);self.k=self.alloc(0x400);self.a=self.alloc(0x200);self.data=self.alloc(0x600);self.cls=self.alloc(0x30);self.other=self.alloc(0x200);self.ids={self.a:4097,self.other:4098}
  self.seams[0x6f7810]=lambda m:m.ret(value=m.alloc(m.arg()))
  self.seams[0x6f7430]=lambda m:m.ret()
 def ctor(self):
  self.u.mem_write(self.k,b'\xcd'*0x400);self.ctor_stop=0x4e567e
  self.u.reg_write(UC_X86_REG_ESI,self.k);self.u.reg_write(UC_X86_REG_EBX,0);self.u.reg_write(UC_X86_REG_EDI,8);self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x4e55a7,self.ctor_stop,count=20000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.ctor_stop and self.esp()==h.STACK-16 and self.fpu_top()==0
  assert self.u.reg_read(UC_X86_REG_ESI)==self.k and self.u.reg_read(UC_X86_REG_EBX)==0
  assert self.r32(self.k+0x26c,22)==[0]*22
  assert self.r32(self.k+0x264,2)==[0,0] and self.r32(self.k+0x2f8,2)==[0,0]
  return {'class_lists':11,'class_head_tail_words':self.r32(self.k+0x26c,22),'supply':self.r32(self.k+0x264,2),'flares':self.r32(self.k+0x2f8,2),'vectors':{hex(o):{'count':self.r32(self.k+o+4),'capacity':self.r32(self.k+o+8),'allocated':self.r32(self.k+o)!=0}for o in [0x2c4,0x2d0,0x2dc,0x2e8]},'stop':hex(self.ctor_stop),'esp_delta':self.esp()-h.STACK}
 def linked(self,offset):
  vals=[];p=self.r32(self.k+offset)
  while p:
   assert len(vals)<100;vals.append(self.ids[self.r32(p)]);p=self.r32(p+4)
  return vals
 def vector(self,offset):
  p,n,cap=self.r32(self.k+offset,3)
  return {'values':[self.ids[x]for x in self.r32(p,n)] if n else [],'count':n,'capacity':cap}
 def run(self,c):
  gt=self.alloc(0x60000);self.w32(0xb364c0,gt)
  self.w32(self.a,0xa273f0,self.data,c.get('flags',0));self.w32(self.a+0x14,4097)
  self.w32(self.data+0x18,self.cls);self.w32(self.cls+0xc,c.get('kind',2));self.w32(self.cls+0x1c,3);self.w32(self.data+0x174,c.get('source_flags',0));self.w32(self.a+0x100,c.get('states',1<<21)&0xffffffff,c.get('states',1<<21)>>32)
  self.w32(self.k+0x190,c.get('word190',1));self.w32(self.k+0x32c,self.alloc(0x10)if c.get('statistics')else 0)
  self.w32(self.k+0x1f8,self.other if c.get('capital')else 0)
  for offset,cap in [(0x2c4,10),(0x2d0,3),(0x2dc,5),(0x2e8,20),(0x318,5)]:
   cap=c.get('capacity',cap);self.w32(self.k+offset,self.alloc(cap*4)if cap else 0,0,cap)
  if c.get('prior_org'):
   self.w32(self.r32(self.k+0x2e8),self.other);self.w32(self.k+0x2ec,1)
  for key,off in [('economy',0x84),('structure',0x94),('settlement',0x98),('hq',0xd8),('organization',0x7c),('supply',0x8c),('properties',0x78),('mobile',0xac)]:
   if c.get(key):
    p=self.alloc(0x80);self.w32(self.a+off,p)
    if key=='structure':self.w32(p+0x10,c.get('structure_handle',0))
  if 'dying' in c:
   body=self.alloc(0x50);self.w32(self.a+0x60,body);self.w32(body+0x1c,c['dying'])
  if c.get('props'):
   prop=self.alloc(0x20);self.w32(prop+8,17);self.w32(prop+0xc,self.alloc(0x20));self.w32(self.r32(self.k+0x318),prop);self.w32(self.k+0x31c,1)
  for address,name in [(0x5704c0,'economy'),(0x4d1350,'organization'),(0x58f010,'statistics'),(0x4f6760,'destroy'),(0x4cef10,'property'),(0x45eca0,'research')]:
   def stop(m,name=name):raise Stop(name)
   self.seams[address]=stop
  result='complete';service=None
  try:
   for _ in range(c.get('repeat',1)):
    self.call(0x4e6180,self.k,[self.a]);assert self.final_esp==h.STACK+8 and self.fpu_top()==0
  except Stop as e:result='required_service';service=e.name
  return {'result':result,'service':service,'class3':self.linked(0x284),'flags':self.r32(self.a+8),'counted':self.r32(self.k+0x2f4),'supply':self.linked(0x264),'capital':self.ids.get(self.r32(self.k+0x1f8)),'vectors':{hex(o):self.vector(o)for o in [0x2c4,0x2d0,0x2dc,0x2e8]}}
def main():
 cases=[{}, {'repeat':2},{'economy':True},{'structure':True},{'structure':True,'structure_handle':4098},{'settlement':True},{'hq':True},{'hq':True,'prior_org':True},{'organization':True},{'supply':True},{'supply':True,'repeat':2},{'kind':29},{'kind':29,'capital':True},{'properties':True,'word190':0},{'properties':True,'word190':0,'props':True},{'structure':True,'word190':0}]
 for cap in [0,1,3]:
  cases.append({'hq':True,'capacity':cap,'repeat':4})
 for dying in [0,0x80000000,0x3f800000,0xbf800000]:
  for flags in [0,128,65536]:cases.append({'states':0,'dying':dying,'source_flags':flags})
 for flags in [0,128,65536]:cases.append({'states':0,'source_flags':flags})
 cases +=[{'states':0,'source_flags':128,'statistics':True},{'mobile':True,'source_flags':128}]
 rows=[];ctors=[]
 for img,label in [(P,'original'),(V,'v14')]:
  h.P=img
  for pc in h.PRECISION:
   ctors.append({'image':label,'precision':pc,'output':Machine(pc).ctor()})
   for c in cases:rows.append({'image':label,'precision':pc,'input':c,'output':Machine(pc).run(c)})
 h.P=P
 for a,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(a,len(bytes.fromhex(raw)))==V.range(a,len(bytes.fromhex(raw)))
 for i in range(len(cases)):assert all(rows[i]['output']==rows[len(cases)*j+i]['output']for j in range(6))
 out={'schema':1,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'constructors':ctors,'cases':rows,'case_count':len(rows),'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())],'scope':'4E55A7..4E567E actual constructor block including11x79E130+96D18D and native vector capacities, not fullKingdom constructor. Full4E6180 with actual4CED60 structure leaf,4E1540 finite counted predicate,4E4470 count/stats guard,4E3C70 no-city return,766DE0/4849F0 vector allocation,4E1F00 supply. Heap allocation/delete explicitseams; actual reached economy/organization/statistics/destroy/property/research bodies are required stops. All fixture inputs synthetic native memory domains; actual owned source binding tested separately. No fullbattleclaim.'}
 p=HERE/'native-owner-add.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'cases':len(rows),'constructors':len(ctors),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
