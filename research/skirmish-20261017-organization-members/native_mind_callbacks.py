"""Actual Mind+114/118 leaves for57CBA0, vtable dispatch from constructor proof.
Active move virtual24 stops at its REAL entry, not a successful child seam.
Return EAX is ignored by the native organization caller and is not modeled.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda b:hashlib.sha256(b).hexdigest()
CTOR=json.loads((ROOT/'research/phase7-mind/native-constructors.json').read_text())
TABLE={kind:struct.unpack('<I',bytes.fromhex(next(r for r in CTOR['cases'] if r['input']['kind']==kind)['output']['initialized_hex'])[:4])[0]for kind in ['base','element','regiment','aircraft']}
class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.c=c;self.mind=self.alloc(0x300);self.u.mem_write(self.mind,b'\xa5'*0x300);self.w32(self.mind,TABLE[c['kind']]);self.w32(self.mind+0x50,c['word50']);self.w32(self.mind+0xb8,c['wordb8']);self.trace=[];self.stop=None;self.child=None;self.status=None
  if c['kind'] in ['regiment','element']:
   offset=0x68 if c['kind']=='regiment' else 0x7c
   self.status=self.mind+offset+0xc;self.w32(self.status,c['status'])
   vtable=0xa33804 if c['kind']=='regiment' else 0xa348bc
   # Element UAI move vtable must come from actual constructor's offset7C.
   cr=next(r for r in CTOR['cases']if r['input']['kind']==c['kind'])
   vtable=struct.unpack_from('<I',bytes.fromhex(cr['output']['initialized_hex']),offset)[0]
   self.w32(self.mind+offset,vtable);self.child=struct.unpack('<I',image.range(vtable+0x24,4))[0]
  self.u.hook_add(UC_HOOK_MEM_READ,self.read);self.u.hook_add(UC_HOOK_MEM_WRITE,self.write)
 def read(self,u,a,p,n,v,d):
  if p==self.status:self.trace.append(['status',self.r32(p)])
 def write(self,u,a,p,n,v,d):
  if p in [self.mind+0x50,self.mind+0xb8]:self.trace.append(['put',p-self.mind,v])
 def _hook(self,u,pc,n,d):
  if self.c['operation']=='speed' and pc==self.child:
   self.trace.append(['move',self.arg(),self.arg(1)]);self.stop='move';h.VISITED[pc]=bytes(u.mem_read(pc,n)).hex();assert h.P.range(pc,n)==h.V.range(pc,n);u.emu_stop();return
  super()._hook(u,pc,n,d)
 def run(self):
  offset=0x118 if self.c['operation']=='speed'else 0x114
  entry=struct.unpack('<I',h.P.range(TABLE[self.c['kind']]+offset,4))[0]
  args=[self.c['old'],self.c['new']]if self.c['operation']=='speed'else[]
  try:self.call(entry,self.mind,args)
  except AssertionError:
   if not self.stop:raise
  return {'trace':self.trace,'stop':self.stop,'word50':self.r32(self.mind+0x50),'wordb8':self.r32(self.mind+0xb8)}
def main():
 rows=[]
 for kind,operation,status,pc in itertools.product(TABLE,['speed','leader'],[0,1,2,3,0x80000000,0xffffffff],h.PRECISION):
  c={'kind':kind,'operation':operation,'status':status,'word50':0x12345678,'wordb8':0xa5a5a5a5,'old':0x40400000,'new':0x40000000}
  out=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert out[0]==out[1];rows.append({'precision':pc,'input':c,'native':out[0]})
 required={int(p['va'],16)for p in []}
 for kind in ['regiment','element']:
  c=next(r['input']for r in rows if r['input']['kind']==kind);required.add(Machine('Double53',h.P,c).child)
 receipt=[]
 for kind,vt in TABLE.items():
  for slot in [0x114,0x118]:
   raw=h.P.range(vt+slot,4);assert raw==h.V.range(vt+slot,4);receipt.append({'kind':kind,'vtable':hex(vt),'slot':hex(slot),'bytes':raw.hex(),'entry':hex(struct.unpack('<I',raw)[0])})
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'ctor_fixture_sha256':H((ROOT/'research/phase7-mind/native-constructors.json').read_bytes()),'rows':rows,'original_v14_executions':2*len(rows),'dispatch':receipt,'pins':[{'va':hex(a),'bytes':b,'kind':'required_move_entry'if a in required else'executed'}for a,b in sorted(h.VISITED.items())]}
 path=HERE/'native-mind-callbacks.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':doc['original_v14_executions'],'pins':len(doc['pins']),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
