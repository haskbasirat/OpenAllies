"""Native4EE560 GTID retirement and empty Actor4D55E0 destruction.

4EE560 executes real4EFEC0 against occupied/current-generation entries. Shape
descriptor production and553830 are named children; membership may stop before
retirement. Destructor cases run actual4D1B80/4EBDD0/4EEFD0 and50C1B0, with
empty owned lists/zones/memory and null SAIActor. Positive map-notify/view/SAI
children stop at their actual entry. Heap/CString managers are transport seams.
No initialized mission or positive object cleanup is claimed by these lab rows.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_removal import Removal,Stop,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPSW

class Destruction(Removal):
 def hook(self,u,pc,size,user):
  if getattr(self,'destruction_ready',False):
   if pc in [0x4cf210,0x4ea0f0]:
    name='blocking'if pc==0x4cf210 else'soft';self.event(name)
    assert self.ecx()==self.actor
    out=self.arg();self.u.mem_write(out,bytes(24));self.ret(4,out);return
   if pc==0x553830:
    assert self.ecx()==self.grid and self.arg()==self.actor
    self.event('membership');self.w(self.actor+8,self.r(self.actor+8)&~(1<<26));self.ret(12);return
   if pc==0x4efec0:
    self.event('retire');assert self.ecx()==self.registry and self.arg()==self.actor
   if pc in [0x4282a0,0x7a10f0,0x43db80]:
    self.event({0x4282a0:'map_notify',0x7a10f0:'sai_actor',0x43db80:'view_unlink'}[pc]);raise Stop()
   if pc==0x6f7430:
    self.event('free',value='actor'if self.arg()==self.actor else'owned');self.ret();return
  super().hook(u,pc,size,user)

def setup(m,pc,c):
 m.destruction_ready=False;m.begin(pc,'detach_dtor',c)
 m.cutoff=c.get('cutoff');m.grid=m.alloc(0x40);m.w(m.world+0x30,m.grid)
 m.registry=m.alloc(0x60020);m.w(0xb364c0,m.registry);occupied=m.alloc(8192)
 m.w(m.registry+0x60010,occupied);m.w(m.registry+0x60004,2);m.w(m.registry+0x6000c,7)
 other=m.alloc(0x30)
 for slot,pointer in [(5,other),(7,m.actor)]:
  m.u.mem_write(m.registry+4+slot*2,struct.pack('<H',1));m.w(m.registry+0x20004+slot*4,pointer);m.w(pointer+0x14,0x10000|slot)
 m.w(occupied,(1<<5)|(1<<7))
 m.destruction_ready=True
 return occupied

def detach(m,pc,c):
 occupied=setup(m,pc,c);error=None
 try:m.call(0x4ee560,[c['retain']],this=m.actor,callee_pop=4)
 except Stop:error=m.trace[-1]['call']
 return dict(error=error,trace=m.trace,flags=m.r(m.actor+8),raw_id=m.r(m.actor+0x14),
  live=m.r(m.registry+0x20004+7*4)==m.actor,generation=int.from_bytes(m.u.mem_read(m.registry+4+7*2,2),'little'),
  active=m.r(m.registry+0x60004),highest=m.r(m.registry+0x6000c),occupied=m.r(occupied)),m.actual.copy()

def destructor(m,pc,c):
 setup(m,pc,c);mapowner=m.alloc(0x50);m.w(0xb3652c,mapowner);m.b(mapowner+4,c['analysed']);m.b(mapowner+5,c['dynamic'])
 m.w(m.actor+0xe8,c['owner']);m.w(m.actor+0x154,1 if c.get('sai_actor')else 0)
 m.w(m.actor+0x150,9)
 for off in [0x18,0xdc,0xe0]:m.cs(m.actor+off,'lab owned text')
 m.w(m.dispatcher+4,1 if c.get('view')else 0)
 # Native pointer labels persist across4EFEC0 and into its later destructor.
 error=None
 try:
  m.call(0x50c1b0,[m.actor],this=m.dispatcher,callee_pop=4)
  m.call(0x4d55e0,[1],this=m.actor,callee_pop=4)
 except Stop:error=m.trace[-1]['call']
 assert ((m.u.reg_read(UC_X86_REG_FPSW)>>11)&7)==0
 return dict(error=error,trace=m.trace,zone_words=[m.r(m.actor+x)for x in [0x148,0x14c,0x150]],raw_id=m.r(m.actor+0x14)),m.actual.copy()

def main():
 cases=[]
 for retain,flags in itertools.product([0,1],[0,1<<26,(1<<26)|3]):
  base=dict(retain=retain,flags=flags);cases.append(('detach',base))
  for cutoff in ['blocking','soft','membership','retire']:cases.append(('detach',dict(base,cutoff=cutoff)))
 for a,d,o in itertools.product([0,1],[0,1],[0,0x10008]):cases.append(('destructor',dict(analysed=a,dynamic=d,owner=o)))
 for extra in [dict(view=True),dict(sai_actor=True)]:cases.append(('destructor',dict(analysed=0,dynamic=0,owner=0,**extra)))
 rows=[];pins={}
 for image in ['original','v14']:
  m=Destruction(image)
  for pc in PC:
   for kind,c in cases:
    output,visited=globals()[kind](m,pc,c);pins.update(visited);rows.append(dict(image=image,precision=pc,kind=kind,input=c,output=output))
 a=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original'];b=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 p=HERE/'native-detach-destructor.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
