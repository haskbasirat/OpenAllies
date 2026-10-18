"""Visual431A90 ctor/431BC0 sampling and whole431550 bilinear query.
Original/v14 execute separately at all x87 PCs. Initialization uses a named
4F0E50 finite sample child seam; its inputs/order/stores are observed. Whole
query has no arithmetic/game-code seams, only synthetic field storage and
an x87 return-observation wrapper. Allocator transport is explicit.
"""
from pathlib import Path
import sys,json,struct,itertools,random
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'map-resource-mode'));import native_tail_constructor as base
h=base.h
from unicorn.x86_const import UC_X86_REG_EIP
from keystone import Ks,KS_ARCH_X86,KS_MODE_32
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def number(v):return struct.unpack('<f',struct.pack('<I',v))[0]
class Machine(h.Machine):
 def __init__(self,pc):
  super().__init__(b'',pc);self.view=self.alloc(0x60,0xa5);self.events=[];self.stage=None
 def fields(self):return dict(step=self.r(self.view+0x10),inverse=self.r(self.view+0x14),columns=self.r(self.view+0x18),rows=self.r(self.view+0x1c),limits=[self.r(self.view+0x20),self.r(self.view+0x24)],heights=[self.r(self.r(self.view+4)+i*4)for i in range(self.r(self.view+8))])
 def construct(self):self.call(0x431a90,self.view,[])
 def initialize(self,c):
  self.construct();world=self.alloc(0x60);height=self.alloc(0x60);self.w(0xb36534,world);self.w(world+0x34,height)
  self.w(height+0xc,c['base'][0]);self.w(height+0x10,c['base'][1]);self.w(height+0x18,c['cell'])
  for o,v in zip([0x48,0x4c],c['limits']):self.w(height+o,v)
  table=self.alloc(4);quad=self.alloc(4);self.w(0xb39940,table);self.w(table,quad);self.w(quad,c['quad'])
  def sample(m):
   i=len(self.events)
   if c['stop']==i:self.stage='sample_error';self.outcome='explicit_child_stop';raise h.Throw()
   xy=m.arg(0);assert m.arg(1)==0x122
   self.events.append([self.r(xy),self.r(xy+4)])
   value=bits((i%7-3)*.125)
   code=self.next_stub;self.next_stub+=32;data=code+16
   self.w(data,value);self.u.mem_write(code,b'\xd9\x05'+struct.pack('<I',data)+b'\xc2\x08\x00')
   self.fld_stubs.update(range(code,code+9));self.u.reg_write(UC_X86_REG_EIP,code)
  self.seam(0x4f0e50,sample);self.call(0x431bc0,self.view,[])
  assert not self.null_accesses,self.null_accesses
  return dict(fields=self.fields(),events=self.events,outcome=self.stage or'complete')
 def query(self,c):
  heights=self.alloc(len(c['heights'])*4)
  for i,v in enumerate(c['heights']):self.w(heights+i*4,v)
  self.w(self.view+4,heights);self.w(self.view+8,len(c['heights']));self.w(self.view+0x10,c['step']);self.w(self.view+0x14,c['inverse']);self.w(self.view+0x18,c['columns']);self.w(self.view+0x1c,c['rows'])
  xy=self.alloc(8);self.w(xy,c['xy'][0]);self.w(xy+4,c['xy'][1]);output=self.alloc(32);wrapper=self.alloc(64)
  code,_=Ks(KS_ARCH_X86,KS_MODE_32).asm(f'push dword ptr [esp+4];call 0x431550;fstp tbyte ptr [{output}];fld tbyte ptr [{output}];fstp dword ptr [{output+16}];ret 4',wrapper)
  self.u.mem_write(wrapper,bytes(code));self.call(wrapper,self.view,[xy]);assert not self.null_accesses
  return dict(extended=str(int.from_bytes(self.u.mem_read(output,10),'little')),height=self.r(output+16))
def main():
 h.VISITED.clear();ctors=[];initial=[];queries=[]
 for image,pe in [('original',base.ORIGINAL),('v14',base.V14)]:
  h.P=pe
  for pc in h.CW:
   m=Machine(pc);m.construct();a=m.r(m.view+0x28)
   ctors.append(dict(image=image,precision=pc,fields=m.fields(),aux_count=m.r(m.view+0x2c),aux_capacity=m.r(m.view+0x30),aux=[[m.r(a+i*16+j*4)for j in range(4)]for i in range(10)]))
 for pc,dimensions,cell,quad,stop in itertools.product(h.CW,[[3,3],[5,4]],[bits(1),bits(2),bits(3.7)],[1,2,3],[None,0,1]):
  c=dict(base=dimensions,cell=cell,quad=quad,limits=[bits(-5),bits(11)],stop=stop)
  h.P=base.ORIGINAL;a=Machine(pc).initialize(c);h.P=base.V14;b=Machine(pc).initialize(c);assert a==b
  initial.append(dict(precision=pc,input=c,output=a))
 randoms=random.Random(431550);points=[[-1,-1],[0,0],[.3,.7],[1,2],[2,1],[100,100],[-100,100],[100,-100]]+[[randoms.uniform(-1,9),randoms.uniform(-1,9)]for _ in range(12)]
 for pc,step,xy in itertools.product(h.CW,[1.,2.,.3],points):
  c=dict(columns=4,rows=3,step=bits(step),inverse=bits(1/step),heights=[bits(randoms.uniform(-10,10))for _ in range(12)],xy=list(map(bits,xy)))
  h.P=base.ORIGINAL;a=Machine(pc).query(c);h.P=base.V14;b=Machine(pc).query(c);assert a==b
  queries.append(dict(precision=pc,input=c,output=a))
 h.P=base.ORIGINAL
 excluded={0x6f7810,0x6f7430,0x4f0e50}
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded and (0x431550<=a<0x4316c0 or 0x431910<=a<0x431d40 or a==0x79e130 or a==0x79e132 or a==0x79e136)}
 for a,b in pins.items():assert base.ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==base.V14.range(int(a,16),len(bytes.fromhex(b)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),constructors=ctors,initialize=initial,queries=queries,pins=pins)
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-view-heightfield.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(constructors=len(ctors),initial_pairs=len(initial),query_pairs=len(queries),pins=len(pins),sha256=h.H(data))))
if __name__=='__main__':main()
