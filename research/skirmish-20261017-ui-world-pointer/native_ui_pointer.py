"""Exact640640/438E10/438970/4385E0 constructor field projection,
6FE590 ray arithmetic and43C550 pointer store order. Allocation/CString/text
and OS timer transports are named seams; terrain71D840 and actual picking
are NOT completed by the project callback fixture. No whole UI/battle claim.
"""
from pathlib import Path
import sys,json,itertools,struct,random,hashlib
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine,PE0,PE1
from unicorn.x86_const import *
P,V=h.P,h.V
H=lambda x:hashlib.sha256(x).hexdigest()
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
class Constructor(BootstrapMachine):
 def __init__(self,image):
  self.active=False;self.ownpins={};self.intercepted=set();self.trace=[]
  super().__init__(image)
 def hook(self,u,pc,size,user):
  if self.active:
   if PE0.base<=pc<PE0.base+PE0.image_size:self.ownpins[pc]=size
   if pc==0x59aa70:
    self.trace.append('frequency');self.intercepted.add(pc);self.ret();return
   if pc==0x59ab60:
    self.trace.append('anchor');self.intercepted.add(pc)
    u.mem_write(u.reg_read(UC_X86_REG_ECX),struct.pack('<Q',self.anchor));self.ret();return
   if pc==0x4b9f40:self.trace.append('display')
  super().hook(u,pc,size,user)
  if self.active and u.reg_read(UC_X86_REG_EIP)!=pc and PE0.base<=pc<PE0.base+PE0.image_size:self.intercepted.add(pc)
 def run(self,c,pc):
  self.u.reg_write(UC_X86_REG_FPCW,h.CW[pc]);self.trace=[];self.anchor=c['anchor'];self.formats={'static_group_wildcard_name':c['caption']}
  state=self.alloc(0x58);self.u.mem_write(state,b'\xa5'*0x58)
  self.active=True;self.call(0x640640,this=state);self.active=False
  brush=self.r(state+0x30);group=self.r(state+0x44)
  assert self.r(state)==0xa61e40
  return dict(zero_words=[self.r(state+o)for o in [8,12,16,20,24,32,36,40,44,56,64,72,76,80,84]],
   bytes=[self.rb(state+o)for o in[0x1c,0x1d]],unwritten4=self.r(state+4),unwritten1e=int.from_bytes(self.u.mem_read(state+0x1e,2),'little'),
   brush=dict(parent=self.r(brush+4)==state,enabled=self.rb(brush+8),anchor=struct.unpack('<Q',self.u.mem_read(brush+0x10,8))[0],words=[self.r(brush+o)for o in[0x18,0x1c,0x24]],attached_parent=self.r(brush+0x28)==brush),
   group=dict(name=self.rs(group),display=self.rs(group+4),text=self.rs(group+8),count=self.r(group+0x10),capacity=self.r(group+0x14),flag18=self.rb(group+0x18),word1c=self.r(group+0x1c),flag20=self.rb(group+0x20)),trace=self.trace)
def ray(c,pc):
 m=h.Machine(b'',pc);camera=m.alloc(0x150,0xa5);out=m.alloc(12,0xa5)
 for off,w in zip(range(0x114,0x148,4),c['camera']):m.w(camera+off,w)
 m.call(0x6fe590,camera,[out,*c['xy']]);assert not m.null_accesses
 return [m.r(out+4*i)for i in range(3)]
def pointer(c,pc):
 m=h.Machine(b'',pc);world=m.alloc(0x1c0,0xa5);event=m.alloc(12);input_=m.alloc(16);ui=m.alloc(0x60);trace=[]
 for off in range(0x178,0x1c0,4):m.w(world+off,c['initial'])
 m.w(0xb36518,m.alloc(4)if c['map']else 0);m.w(0xb3653c,ui);m.w(ui+0x58,input_)
 for i,x in enumerate(c['fallback']):m.w(input_+4+4*i,x&0xffffffff)
 m.u.mem_write(event,struct.pack('<hhIHH',*c['pixel'],c['flags'],c['code'],0))
 def project(mm):
  out,point,normal=mm.arg(),mm.arg(1),mm.arg(2)
  index=(normal-(world+0x178))//24;xy=[struct.unpack('<i',mm.u.mem_read(point+4*i,4))[0]for i in range(2)]
  trace.append(dict(slot=index,point=xy))
  for i,w in enumerate([100+index,200+index,300+index]):mm.w(normal+4*i,w)
  if c.get('stop')==index:mm.outcome={'child':'project'};raise h.Throw()
  for i,w in enumerate([400+index,500+index,600+index]):mm.w(out+4*i,w)
  mm.ret(12,out)
 m.seam(0x43c4b0,project);m.call(0x43c550,world,[event if c['event']else 0]);assert not m.null_accesses
 return dict(trace=trace,words=[m.r(world+off)for off in range(0x178,0x1c0,4)],required=m.outcome is not None)
def projection(c,pc):
 m=h.Machine(b'',pc);camera=m.alloc(0x150,0xa5);current=m.alloc(4);terrain=m.alloc(4);out=m.alloc(12,0xa5);point=m.alloc(8);normal=m.alloc(12,0xa5);trace=[]
 for off,w in zip(range(0x114,0x148,4),c['camera']):m.w(camera+off,w)
 for i,w in enumerate(c['eye']):m.w(camera+0x90+4*i,w)
 for i,w in enumerate(c['pixel']):m.w(point+4*i,w&0xffffffff)
 m.w(current,camera);m.w(0xb40048,current);m.w(0xb36518,terrain);m.w(0xb396d0,c['width']);m.w(0xb396d4,c['height'])
 def ray_entry(mm):trace.append(dict(normalized=[mm.arg(1),mm.arg(2)]))
 m.probes[0x6fe590]=ray_entry
 def child(mm):
  assert mm.u.reg_read(UC_X86_REG_ECX)==terrain
  assert mm.arg()==camera+0x90 and mm.arg(5)==normal and mm.arg(6)==0
  trace.append(dict(eye=[mm.r(mm.arg()+4*i)for i in range(3)],ray=[mm.arg(i)for i in[1,2,3]]))
  for i,w in enumerate([11,22,33]):mm.w(mm.arg(4)+4*i,w)
  for i,w in enumerate([44,55,66]):mm.w(normal+4*i,w)
  mm.ret(28,c['return'])
 m.seam(0x71d840,child);m.call(0x43c4b0,0,[out,point,normal]);assert not m.null_accesses
 return dict(point=[m.r(out+4*i)for i in range(3)],normal=[m.r(normal+4*i)for i in range(3)],trace=trace)
def main():
 constructors=[];rays=[];pointers=[];projections=[];ctorpins={};intercepted=set();h.VISITED.clear()
 for pc,caption,anchor in itertools.product(h.CW,['All Units','Toutes les unités Ω'],[0,0x1234567812345678]):
  c=dict(caption=caption,anchor=anchor);pair=[]
  for image in ['original','v14']:
   m=Constructor(image);pair.append(m.run(c,pc));ctorpins.update(m.ownpins);intercepted.update(m.intercepted)
  assert pair[0]==pair[1];constructors.append(dict(precision=pc,input=c,native=pair[0]))
 rng=random.Random(0x6fe590);cases=[]
 for i in range(24):
  vals=[F(rng.uniform(-10,10))for _ in range(13)];cases.append(dict(camera=vals,xy=[F(rng.uniform(-1,2)),F(rng.uniform(-1,2))]))
 for pc,c in itertools.product(h.CW,cases):
  pair=[]
  for pe in[P,V]:h.P=pe;pair.append(ray(c,pc))
  assert pair[0]==pair[1];rays.append(dict(precision=pc,input=c,native=pair[0]))
 for pc,i,ret in itertools.product(h.CW,range(4),[0,1]):
  c=dict(camera=cases[i]['camera'],eye=[F(13.25),F(-24.5),F(99.75)],pixel=[-17+i*12345,32767-i*7654],width=1919+i,height=1079+i,**{'return':ret});pair=[]
  for pe in[P,V]:h.P=pe;pair.append(projection(c,pc))
  assert pair[0]==pair[1];projections.append(dict(precision=pc,input=c,native=pair[0]))
 for pc,event,code,flags,stop in itertools.product(h.CW,[False,True],[0x100,0x101,0x108,0x200],[0,1],[None,0,1,2]):
  if not event and(code!=0x100 or flags!=0):continue
  c=dict(event=event,pixel=[-17,32767],fallback=[0x12345678,-1234],map=True,initial=0xdeadbeef,code=code,flags=flags)
  if stop is not None:c['stop']=stop
  pair=[]
  for pe in[P,V]:h.P=pe;pair.append(pointer(c,pc))
  assert pair[0]==pair[1];pointers.append(dict(precision=pc,input=c,native=pair[0]))
 for pc in h.CW:
  c=dict(event=False,pixel=[0,0],fallback=[9,7],map=False,initial=0xdeadbeef,code=0x100,flags=1)
  pair=[]
  for pe in[P,V]:h.P=pe;pair.append(pointer(c,pc))
  assert pair[0]==pair[1];pointers.append(dict(precision=pc,input=c,native=pair[0]))
 h.P=P
 pins=[]
 for a,n in sorted(ctorpins.items()):
  raw=PE0.range(a,n);assert raw==PE1.range(a,n)
  pins.append(dict(address=hex(a),bytes=raw.hex(),classification='transport entry'if a in intercepted else'executed'))
 corepins={hex(a):raw for a,raw in sorted(h.VISITED.items())if a not in[0x43c4b0,0x71d840]}
 #43C4B0 is executed by projection rows, intercepted by pointer rows. Its
 #entry is separately identified, never counted as an executed child seam.
 projection_entry=dict(address='0x43c4b0',bytes=P.range(0x43c4b0,4).hex(),classification='executed in projection rows; transported in pointer rows')
 out=dict(schema=1,scope=__doc__,constructors=constructors,rays=rays,pointers=pointers,projections=projections,constructor_pins=pins,core_pins=corepins,projection_entry=projection_entry,executions=2*(len(constructors)+len(rays)+len(pointers)+len(projections)),producer_sha256=H(Path(__file__).read_bytes()),seams=['BootstrapMachine inherited allocation/CString/localization/string-manager transport','59AA70/59AB60 explicit OS frequency/anchor transport; not executed timer bodies','43C4B0 explicit projection callback in pointer rows only: optional-normal transport, returns point or stops','71D840 terrain intersection returns explicit point/normal and AL0/1; body excluded'])
 path=HERE/'native-ui-pointer.json'
 if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
 else:assert out==json.loads(path.read_text())
 print(json.dumps(dict(constructors=len(constructors),rays=len(rays),pointer_rows=len(pointers),projections=len(projections),executions=out['executions'],constructor_executed=sum(x['classification']=='executed'for x in pins),core_pins=len(corepins),sha256=H(path.read_bytes()))))
if __name__=='__main__':main()

