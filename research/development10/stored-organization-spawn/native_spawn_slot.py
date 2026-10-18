"""Native57AD30 including raw51 and native ActorParams constructor fields.

4FBF40 is an explicit creation child/cutoff, not a successful World service.
CString manager/nil storage and virtual108 are explicit transport services.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
OLD=h.P;V14=h.V
class Cutoff(Exception):pass
class M(h.Machine):
 def run(self,c):
  self.trace=[];manager=self.alloc(16);nil=self.alloc(32);mvt=self.alloc(16)
  self.w32(manager,mvt);self.w32(mvt+12,h.STUBS+0x100);self.w32(nil,manager,0,0,1000)
  self.seams[0x856370]=lambda m:m.ret(value=manager)
  self.seams[h.STUBS+0x100]=lambda m:m.ret(value=nil)
  world=self.alloc(0x100);self.w32(0xb36534,world)
  org=self.alloc(0x100);owner=self.alloc(0x140);vt=self.alloc(0x110);layout=self.alloc(0x40)
  groups=self.alloc(8);defs=self.alloc(8);members=self.alloc(8)
  self.w32(owner,vt);self.w32(vt+0x108,h.STUBS+0x200)
  self.w32(org+4,owner);self.w32(org+0x18,layout);self.w32(org+0x1c,defs);self.w32(org+0x28,members)
  self.w32(layout+0x28,groups);self.w32(groups,1,0);self.w32(defs,0x7001,0x7000)
  self.w32(members+4*c['slot'],0x1234 if c['occupied'] else 0)
  def allegiance(m):
   m.trace.append('allegiance')
   if c['fail']=='allegiance':raise Cutoff()
   m.ret(value=c['owner'])
  self.seams[h.STUBS+0x200]=allegiance
  def create(m):
   p=m.arg(0);words=list(struct.unpack('<21I',m.u.mem_read(p,0x54)))
   assert words[7]==nil+16
   #Name is the actual empty CString; native pointer/vtable identities and
   #unwritten allocation-padding byte53 are excluded from numeric fields.
   words[0]=0;words[7]=0;words[20]&=0x00ffffff
   m.trace.append(dict(create=dict(words=words,slot=m.arg(2),world=m.ecx()==world,owner=m.arg(1)==owner)))
   if c['fail']=='create':raise Cutoff()
   m.ret(12,0x1200 if c['created'] else 0)
  self.seams[0x4fbf40]=create
  error=None
  try:returned=self.call(0x57ad30,org,[c['slot'],c['flag']])&255
  except Cutoff:error=c['fail'];returned=None
  return dict(returned=returned,trace=self.trace,error=error)
def main():
 h.VISITED.clear();rows=[]
 cases=[dict(slot=s,occupied=o,flag=f,owner=k,created=r,fail=e)
        for s,o,f,k,r,e in itertools.product([0,1],[False,True],[0,1,128,255,257],[0,1676],[False,True],[None,'allegiance','create'])]
 for pc,c in itertools.product(h.PRECISION,cases):
  h.P=OLD;a=M(pc).run(c);h.P=V14;b=M(pc).run(c);assert a==b,(pc,c,a,b)
  rows.append(dict(pc=pc,input=c,output=a))
 h.P=OLD
 excluded=[0x856370,0x4fbf40]
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,pins=pins,excluded_entries=[hex(a)for a in excluded],padding='ActorParams53 is unwritten and excluded; native vtable and CString allocation addresses are not portable identities.')
 raw=(json.dumps(doc,indent=2)+'\n').encode();path=HERE/'native-spawn-slot.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),executed_pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
