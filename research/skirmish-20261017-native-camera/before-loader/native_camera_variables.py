"""Execute actual401BD0 AVar registration prefix through CameraFOV23.

All actual Float401710/boundedFloat401A10 constructors execute. CString/heap
and Settings4B3470 registration are named transports. Registration copies
the same constructed Var into the stable fast table so later bounded defaults
read actual prior registrations. No profile/source setter is claimed here.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-view-init'))
import native_uvar_registration as base
from unicorn.x86_const import UC_X86_REG_EIP
class Done(Exception):pass
class Machine(base.Machine):
 def __init__(self,image,pc):
  super().__init__(image,pc);self.fast=self.alloc(0x400);self.manager=self.alloc(0x40);self.w(self.manager+8,self.fast);self.w(0xb39990,self.fast)
 def register(self):
  index=self.arg(0);p=self.arg(1)
  assert index==len(self.rows),(index,len(self.rows))
  row=dict(index=index,name=self.rs(p+0x1c),kind=self.r(p+0x24),flags=self.r(p+0x28),word=self.r(p),bounds=[self.r(p+0x14),self.r(p+0x18)])
  self.rows.append(row);var=self.alloc(0x2c);self.u.mem_write(var,bytes(self.u.mem_read(p,0x2c)));self.w(self.fast+4*index,var)
  if index==23:self.intercepted.add(0x4b3470);raise Done()
  self.ret(8,var)
 def run(self):
  try:self.call(0x401bd0,[],this=self.manager,callee_pop=0)
  except Done:pass
  assert len(self.rows)==24
  return self.rows[7:24]
def main():
 rows=[];visited={};seams=set();by_image={}
 for image,pc in itertools.product(('original','v14'),base.h.PRECISION):
  m=Machine(image,pc);output=m.run();visited.update(m.visited);seams|=m.intercepted|{0x4b3470};rows.append(dict(image=image,precision=pc,variables=output));by_image.setdefault(image,output);assert by_image[image]==output
 pins=[];differences=[]
 for va,size in sorted(visited.items()):
  if va in seams or not(0x401000<=va<0x9ef000):continue
  a=base.h.P.range(va,size);b=base.h.V.range(va,size)
  if a!=b:differences.append(dict(va=hex(va),original=a.hex(),v14=b.hex()))
  pins.append(dict(va=hex(va),original=a.hex(),v14=b.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=pins,target_instruction_differences=differences,seams_not_executed=[hex(v)for v in sorted(seams)])
 path=HERE/'native-camera-variables.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),variables_per_row=17,pins=len(pins),target_instruction_differences=len(differences),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
