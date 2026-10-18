"""Actual shared564D80 Construction/Detection/Lair Init gate and constant leaves.
Original and v14 execute separately at all3PC.50AC60 stops at its genuine entry;
the existing native queue proof covers that child's implementation. No mock
success. Component head is read only below version22, with unsigned comparison.
"""
from pathlib import Path
import hashlib,json,sys,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ECX
H=lambda b:hashlib.sha256(b).hexdigest()
CLASSES={15:(0xa3b950,0xa3b924,0x4b2830),22:(0xa3c568,0xa3c540,0x56ec90),25:(0xa3cc6c,0xa3cc44,0x4b2830)}

class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.component=self.alloc(0x60);self.world=self.alloc(0x300)
  self.u.mem_write(self.component,b'\xa5'*0x60)
  vt,up,after=CLASSES[c['slot']];self.w32(self.component,vt);self.w32(self.component+8,up)
  self.w32(self.component+0xc,self.world if c['scheduled'] else 0)
  self.w32(0xb36534,self.world);self.w32(self.world+0x2c,c['version'])
  self.reads=[];self.writes=[];self.stop=None
  self.u.hook_add(UC_HOOK_MEM_READ,self.read,begin=self.component,end=self.component+0x5f)
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write,begin=self.component,end=self.component+0x5f)
 def read(self,u,access,address,size,value,_):self.reads.append([address-self.component,size])
 def write(self,u,access,address,size,value,_):self.writes.append([address-self.component,size,value])
 def _hook(self,u,pc,size,_):
  if pc==0x50ac60:
   h.VISITED[pc]=bytes(u.mem_read(pc,size)).hex();assert h.P.range(pc,size)==h.V.range(pc,size)
   esp=u.reg_read(UC_X86_REG_ESP)
   self.stop={'entry':'50AC60','this_offset':u.reg_read(UC_X86_REG_ECX)-self.component,'data':self.r32(esp+4)}
   assert self.stop['this_offset']==8 and self.stop['data']==0
   u.emu_stop();return
  super()._hook(u,pc,size,_)
 def run(self,c):
  before=bytes(self.u.mem_read(self.component,0x60))
  try:result=self.call(0x564d80,self.component)&255
  except AssertionError:
   if not self.stop:raise
   result=None
  pc=self.u.reg_read(UC_X86_REG_EIP);assert pc==h.STOP or pc==0x50ac60
  reads=list(self.reads);writes=list(self.writes)
  assert reads==([[12,4]] if c['version']<22 else [])
  assert writes==[] and before==bytes(self.u.mem_read(self.component,0x60)) and self.seams=={}
  vt,up,after=CLASSES[c['slot']]
  assert self.call(h.P.u32(up+8),self.component+8)&255==1
  if c['slot']!=22:self.call(after,self.component)
  return {'outcome':'required50AC60' if self.stop else 'complete','init_result':None if self.stop else bool(result),'component_reads':reads,'component_writes':writes,'queue_child':self.stop,'after_entry':hex(after),'after_executed':c['slot']!=22,'updatable_should_schedule':True}

def main():
 rows=[]
 for slot,version,scheduled in itertools.product(CLASSES,[0,21,22,27,0xffffffff],[False,True]):
  c={'slot':slot,'version':version,'scheduled':scheduled}
  for pc in h.PRECISION:
   values=[Machine(pc,image,c).run(c) for image in (h.P,h.V)];assert values[0]==values[1]
   rows.append({'precision':pc,'input':c,'native':values[0]})
 pins=[]
 for slot,(vt,up,after) in CLASSES.items():
  for base,off,expected in [(vt,8,0x564d80),(vt,0x14,after),(up,8,0x6a71d0)]:
   assert h.P.u32(base+off)==h.V.u32(base+off)==expected
   pins.append({'slot':slot,'va':hex(base+off),'bytes':h.P.range(base+off,4).hex(),'entry':hex(expected)})
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'fresh_original_v14_machines':len(rows)*2,'rows':rows,'vtable_pins':pins,'address_byte_pins':[{'va':hex(a),'bytes':b,'stopped_entry':a==0x50ac60} for a,b in sorted(h.VISITED.items())],'reused_queue_proof_sha256':H((ROOT/'research/phase6-queues/native-queues.json').read_bytes())}
 p=HERE/'native-scheduled-init.json'
 if '--write' in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(doc['address_byte_pins']),'executed':sum(not x['stopped_entry'] for x in doc['address_byte_pins']),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
