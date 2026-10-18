"""Actual4EC010 coordinator and4CED60 Structure getter, original/v14/allPC.
The memory-return seam is explicitly TEST ONLY to expose late16D ordering;
separate stop cases retain the real4EBC70 required entry. Production reuses
the existing concrete memory owner. Recursive container, Wall and first-sighting
children stop at actual native entries, never receive successful fake effects.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.c=c;self.actor=self.alloc(0x180);self.definition=self.alloc(0x480);self.kingdom=self.alloc(0x240);self.structure=self.alloc(0x38);self.wall=self.alloc(0x20);self.child=self.alloc(0x180)
  self.u.mem_write(self.actor,b'\xa5'*0x180)
  self.w32(self.actor,0xa273f0,self.definition);self.w32(self.actor+0x54,c['word']);self.w32(self.actor+0x94,self.structure if c['structure'] else 0);self.w32(self.actor+0x90,self.wall if c['wall'] else 0)
  self.w32(self.structure+0x10,self.child if c['container'] else 0);self.w32(self.child,0xa273f0)
  self.w32(self.kingdom+0x18c,c['index']);self.w32(self.kingdom+0x190,c['nonmajor']);self.u.mem_write(self.definition+0x16d,bytes([c['announce']]))
  self.trace=[];self.stop=None;self.u.hook_add(UC_HOOK_MEM_READ,self.read,begin=self.definition+0x16d,end=self.definition+0x16d)
  if c['memory']=='return-seam':self.seams[0x4ebc70]=lambda m:m.ret(value=0)
 def read(self,u,a,p,n,v,d):self.trace.append('announce')
 def _hook(self,u,pc,n,d):
  if pc==0x4ec010 and self.ecx()==self.child:self.trace.append('recursive');self.stop='recursive'
  elif pc==0x58cb30:self.trace.append('wall');self.stop='wall'
  elif pc==0x432e40:self.trace.append('first');self.stop='first'
  elif pc==0x4ebc70:
   self.trace.append('memory')
   if self.c['memory']=='stop':self.stop='memory'
  else:
   name={0x54ec70:'shown',0x54ecb0:'seen',0x54eae0:'show',0x54ea80:'explore',0x6a71d0:'b4',0x4ced60:'container'}.get(pc)
   if name:self.trace.append(name)
  if self.stop:
   h.VISITED[pc]=bytes(u.mem_read(pc,n)).hex();assert h.P.range(pc,n)==h.V.range(pc,n);u.emu_stop();return
  super()._hook(u,pc,n,d)
 def run(self,entry):
  args=[] if entry==0x4ced60 else [0 if self.c['null'] else self.kingdom,int(self.c['explore']),int(self.c['show'])]
  try:r=self.call(entry,self.actor,args)
  except h.NullDereference:
   assert self.c['null'];self.stop='null';r=None
  except AssertionError:
   if not self.stop:raise
   r=None
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.stop
  return {'word':self.r32(self.actor+0x54),'trace':self.trace,'stop':self.stop,'container':None if r is None else bool(r)}
def base():return {'word':0,'index':3,'nonmajor':0,'null':False,'explore':True,'show':True,'announce':0,'structure':True,'container':False,'wall':False,'memory':'stop'}
def main():
 cases=[]
 for word,explore,show,memory in itertools.product([0,8,0x80000,0x80008],[False,True],[False,True],['stop','return-seam']):
  c=base();c.update(word=word,explore=explore,show=show,memory=memory);cases.append(c)
 for change in [{'announce':1,'memory':'return-seam'},{'container':True},{'wall':True},{'null':True},{'nonmajor':1},{'index':31},{'structure':False}]:
  c=base();c.update(change);cases.append(c)
 rows=[]
 for pc,c in itertools.product(h.PRECISION,cases):
  results=[Machine(pc,image,c).run(0x4ec010) for image in(h.P,h.V)];assert results[0]==results[1];rows.append({'precision':pc,'input':c,'native':results[0]})
 containers=[]
 for pc,structure,container,wall in itertools.product(h.PRECISION,[False,True],[False,True],[False,True]):
  c=base();c.update(structure=structure,container=container,wall=wall)
  results=[Machine(pc,image,c).run(0x4ced60) for image in(h.P,h.V)];assert results[0]==results[1];containers.append({'precision':pc,'input':c,'native':results[0]})
 stopped={0x58cb30,0x432e40};testseam={0x4ebc70}
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'actual2696_structure_source_sha256':H((ROOT/'research/skirmish-20261016-load-detection-init/actual-two-activation-prefixes-v2.json').read_bytes()),'rows':rows,'containers':containers,'native_machines':2*(len(rows)+len(containers)),'address_pins':[{'va':hex(a),'bytes':b,'kind':'test_memory_seam_or_required_entry' if a in testseam else 'required_entry' if a in stopped else 'executed'} for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-actor-reveal.json'
 if '--write' in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'containers':len(containers),'native_machines':doc['native_machines'],'pins':len(doc['address_pins']),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
