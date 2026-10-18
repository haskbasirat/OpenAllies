"""Actual76C7C0 BC traversal and765E00 lazy globalWorld/rawPlayer28 getter.

Positive765E10 clears are named laboratory children with optional entry
cutoffs. This proves caller order and retained prefixes, not real VWorld/player
replacement. NoWorld rows poison Player28 reads; an actual positive World is
supplied explicitly in the other rows. Both images/all three x87 modes.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_READ
class Stop(Exception):pass
class Scan(Machine):
 def __init__(self,image):
  self.scan_ready=False;super().__init__(image);self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,user):
  if self.scan_ready and not self.world_present and address in self.raw_words:raise AssertionError('noWorld read Player28')
 def hook(self,u,pc,size,user):
  if self.scan_ready and pc==0x765e10:
   assert self.ecx()in self.player_indices and self.arg()==0
   index=self.player_indices[self.ecx()];self.events.append(index)
   if self.cutoff==len(self.events)-1:raise Stop()
   self.w(self.ecx()+0x28,0);self.ret(4);return
  super().hook(u,pc,size,user)
def run(m,pc,c):
 m.scan_ready=False;m.fpu(pc);m.actual={};m.record_actual=True;m.events=[];m.cutoff=c.get('cutoff')
 session=m.alloc(0x200);kingdom=m.alloc(0x400);other=m.alloc(0x400)
 m.world_present=c['world'];m.w(0xb36534,m.alloc(0x300)if m.world_present else 0)
 players=[];m.player_indices={};m.raw_words=set();head=0
 for index,value in enumerate(c['links']):
  player=m.alloc(0x60);players.append(player);m.player_indices[player]=index;m.raw_words.add(player+0x28)
  m.w(player+0x28,kingdom if value=='removed'else other if value=='other'else 0xdeadbeef if value=='poison'else 0)
 for player in reversed(players):
  node=m.alloc(8);m.w(node,player);m.w(node+4,head);head=node
 m.w(session+0xbc,head);m.scan_ready=True;error=None
 try:m.call(0x76c7c0,[kingdom],this=session,callee_pop=4)
 except Stop:error='clear required'
 def label(value):return 'removed'if value==kingdom else'other'if value==other else'poison'if value==0xdeadbeef else None
 output=dict(error=error,clears=m.events,links=[label(m.r(p+0x28))for p in players])
 return output,m.actual.copy()
def main():
 cases=[dict(world=False,links=['poison','poison']),dict(world=True,links=[]),dict(world=True,links=[None]),dict(world=True,links=['other']),dict(world=True,links=[None,'other',None]),dict(world=True,links=['removed','removed','other']),dict(world=True,links=['removed',None,'removed'])]
 cases.extend(dict(world=True,links=['removed',None,'removed'],cutoff=n)for n in [0,1])
 rows=[];pins={}
 for image in ['original','v14']:
  m=Scan(image)
  for pc in PC:
   for case in cases:
    output,visited=run(m,pc,case);rows.append(dict(image=image,precision=pc,input=case,output=output));pins.update(visited)
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 path=HERE/'native-session-scan.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
