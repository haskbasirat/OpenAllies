"""Actual551F90/551FD0 ->54F400/54F4B0 ->actual class+B0/+94/+98.
4EBC70 memory is either a labelled coordinator test seam or a required stop;
its effects are NOT certified by the returning seam. Each case runs both
original and v14 at all3PC, retaining tile/actor/bit30 failure prefixes.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ECX
H=lambda b:hashlib.sha256(b).hexdigest()
VT={2:0xa2a2a0,3:0xa273f0,4:0xa299d8,6:0xa28a00,7:0xa28b88,9:0xa28e90}
class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.c=c;self.events=[];self.required=None
  self.map=self.alloc(0x60);self.tiles=self.alloc(0x60);self.nodes=[self.alloc(0x10)for _ in range(3)]
  self.actors=[self.alloc(0x200)for _ in range(3)];self.identities=self.alloc(0x40010)
  self.w32(0xb364c0,self.identities);self.w32(self.map+0x1c,2,1,0,self.tiles)
  assert h.P.range(0xa11740,4)==h.V.range(0xa11740,4)==bytes.fromhex('10000000')
  for i,a in enumerate(self.actors):
   self.w32(a,VT[c['class']]);self.w32(a+8,c['flags'] if i!=1 else 0);self.w32(a+0x54,0x10203040+i)
   self.w32(a+0x50,0x0003000a)
   self.w32(self.nodes[i],a,self.nodes[i+1]if i<2 else 0)
  self.w32(self.tiles+8,self.nodes[0]);self.w32(self.tiles+0x14,0x12345678)
  self.w32(self.tiles+0x30+0x14,0xabcdef01)
  self.u.mem_write(self.identities+4+10*2,struct.pack('<H',3 if c['associated'] else 2))
  self.w32(self.identities+0x20004+10*4,self.actors[2])
  self.seams[0x4ebc70]=lambda m:m.memory()
 def memory(self):
  a=self.u.reg_read(UC_X86_REG_ECX);i=self.actors.index(a)
  mode='visible'if self.r32(a+0x54)&0xffff0000==0xffff0000 else 'explored'
  self.events.append({'actor':i,'mode':mode,'flags':self.r32(a+8),'word':self.r32(a+0x54)})
  if self.c['stop']==mode:self.required='4EBC70:'+mode;self.u.emu_stop();return
  if self.c['mutate'] and len(self.events)==1:
   self.w32(a+8,self.r32(a+8)^0x10)
   self.w32(self.nodes[0]+4,self.nodes[2])
   self.w32(self.map+0x1c,1)
  self.ret()
 def run(self):
  try:self.call(0x551fd0 if self.c['visible']else 0x551f90,self.map)
  except AssertionError:
   if not self.required:raise
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.required
  return {'outcome':self.required or'complete','tiles':[self.r32(self.tiles+i*0x30+0x14)for i in range(2)],'actors':[{'flags':self.r32(a+8),'word':self.r32(a+0x54)}for a in self.actors],'events':self.events,'width':self.r32(self.map+0x1c),'next_first':2 if self.r32(self.nodes[0]+4)==self.nodes[2]else 1}
def main():
 tables=[]
 for cls,vt in VT.items():
  slots={}
  for offset,want in [(0xb0,0x6a71d0),(0x94,0x4ebfb0),(0x98,0x4ebfd0)]:
   p=h.P.range(vt+offset,4);assert p==h.V.range(vt+offset,4)and struct.unpack('<I',p)[0]==want
   slots[hex(offset)]={'entry':hex(want),'bytes':p.hex()}
  tables.append({'class':cls,'vtable':hex(vt),'slots':slots})
 rows=[]
 for cls in VT:
  for flags in [0,1<<11,(1<<11)|(1<<23),1<<20,(1<<20)|(1<<23),(1<<20)|(1<<30)]:
   for associated in [False,True]:
    for visible in [False,True]:
     for stop,mutate in [('none',False),('none',True),('explored',False),('visible',False)]:
      c={'class':cls,'flags':flags,'associated':associated,'visible':visible,'stop':stop,'mutate':mutate}
      for pc in h.PRECISION:
       values=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert values[0]==values[1]
       rows.append({'pc':pc,'input':c,'native':values[0]})
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'visible_shift':{'va':'0xa11740','bytes':'10000000'},'vtables':tables,'rows':rows,'native_machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':b,'kind':'test_memory_seam_or_required_entry'if a==0x4ebc70 else'executed'}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-store-fog.json'
 if'--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':doc['native_machines'],'pins':len(doc['pins']),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
