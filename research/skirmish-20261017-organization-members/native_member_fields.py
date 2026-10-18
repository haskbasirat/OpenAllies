"""Actual member Mind vf3C/vf40 fields, all four constructor vtables.
Controlled current24/28/flags and empty nested Regiment vector are LAB inputs.
Healthy home virtual174 stops at its actual entry; no fake relation result.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_WRITE
H=lambda b:hashlib.sha256(b).hexdigest()
CTOR=json.loads((ROOT/'research/phase7-mind/native-constructors.json').read_text())
TABLE={kind:struct.unpack('<I',bytes.fromhex(next(r for r in CTOR['cases']if r['input']['kind']==kind)['output']['initialized_hex'])[:4])[0]for kind in ['base','element','regiment','aircraft']}
class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.c=c;self.mind=self.alloc(0x300);self.actor=self.alloc(0x180);self.org=self.alloc(0xdc);self.target=self.alloc(0x180);self.w32(self.actor+0x7c,self.org);self.w32(self.org+0x2c,0);self.w32(self.mind,TABLE[c['kind']],self.actor);self.w32(self.mind+0x24,c['word24'],c['word28']);self.w32(self.target,0xa273f0);self.w32(self.target+8,c['flags']);self.trace=[];self.stop=None
  self.required=struct.unpack('<I',image.range(0xa273f0+0x174,4))[0]
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write)
 def write(self,u,a,p,n,v,d):
  if p in [self.mind+0x24,self.mind+0x28]:self.trace.append(['put',p-self.mind,v])
 def _hook(self,u,pc,n,d):
  if pc==self.required:
   self.trace.append(['required174']);self.stop='relation174';h.VISITED[pc]=bytes(u.mem_read(pc,n)).hex();assert h.P.range(pc,n)==h.V.range(pc,n);u.emu_stop();return
  super()._hook(u,pc,n,d)
 def run(self):
  slot=0x3c if self.c['operation']=='word24'else 0x40
  entry=struct.unpack('<I',h.P.range(TABLE[self.c['kind']]+slot,4))[0]
  arg=self.c['value']if slot==0x3c else(self.target if self.c['target']else 0)
  try:self.call(entry,self.mind,[arg])
  except AssertionError:
   if not self.stop:raise
  return {'trace':self.trace,'stop':self.stop,'word24':self.r32(self.mind+0x24),'word28':self.r32(self.mind+0x28)}
def main():
 rows=[];required=set()
 for kind,pc in itertools.product(TABLE,h.PRECISION):
  for op,target,flags in [('word24',False,0),('home',False,0),('home',True,1),('home',True,0),('home',True,128),('home',True,0xffffffff)]:
   c={'kind':kind,'operation':op,'target':target,'flags':flags,'value':0x87654321,'word24':0xa5a5a5a5,'word28':0x12345678}
   machines=[Machine(pc,image,c)for image in(h.P,h.V)];out=[m.run()for m in machines];assert out[0]==out[1];required.add(machines[0].required);rows.append({'precision':pc,'input':c,'native':out[0]})
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'original_v14_executions':2*len(rows),'pins':[{'va':hex(a),'bytes':b,'kind':'required_entry'if a in required else'executed'}for a,b in sorted(h.VISITED.items())]}
 path=HERE/'native-member-fields.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':doc['original_v14_executions'],'pins':len(doc['pins']),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
