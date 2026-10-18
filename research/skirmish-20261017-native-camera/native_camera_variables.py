"""Execute actual401BD0 AVar registration prefix through CameraFOV23.

All actual Float401710/boundedFloat401A10 constructors execute. CString/heap
and Settings4B3470 registration are named transports. Registration copies
the same constructed Var into the stable fast table so later bounded defaults
read actual prior registrations. No profile/source setter is claimed here.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
sys.path.insert(0,str(ROOT/'research/phase8-ai'));from native_machine import original,modified,PRECISIONS
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
class Done(Exception):pass
ORIGINAL=L.IMAGE;MODIFIED=L.ACCEPTED
class Machine(L.Loader):
 def __init__(self,image,pc):
  L.IMAGE=ORIGINAL if image=='original'else MODIFIED
  super().__init__(PRECISIONS[pc]);self.fast=self.alloc(0x400);self.manager=self.alloc(0x40);self.w(self.manager+8,self.fast);self.w(0xb39990,self.fast)
  self.rows=[];self.actual={};self.seam(0x4b3470,self.register)
  self.u.hook_add(UC_HOOK_CODE,self.actual_hook)
 def actual_hook(self,u,pc,size,_):
  if pc not in self.seams and u.reg_read(UC_X86_REG_EIP)==pc and original.base<=pc<original.base+original.image_size:self.actual[pc]=size
 def text(self,p):
  out=[]
  while (c:=int.from_bytes(self.u.mem_read(p+2*len(out),2),'little')):out.append(c)
  return ''.join(map(chr,out))
 def register(self):
  index=self.arg(0);p=self.arg(1)
  assert index==len(self.rows),(index,len(self.rows))
  row=dict(index=index,name=self.text(self.r(p+0x1c)),kind=self.r(p+0x24),flags=self.r(p+0x28),word=self.r(p),bounds=[self.r(p+0x14),self.r(p+0x18)])
  self.rows.append(row);var=self.alloc(0x2c);self.u.mem_write(var,bytes(self.u.mem_read(p,0x2c)));self.w(self.fast+4*index,var)
  if index==23:raise Done()
  self.ret(8,var)
 def run(self):
  try:self.call(0x401bd0,self.manager)
  except Done:pass
  assert len(self.rows)==24
  return self.rows[7:24]
def main():
 rows=[];visited={};seams=set();by_image={}
 for image,pc in itertools.product(('original','v14'),PRECISIONS):
  m=Machine(image,pc);output=m.run();visited.update(m.actual);seams|=set(m.seams);rows.append(dict(image=image,precision=pc,variables=output));by_image.setdefault(image,output);assert by_image[image]==output
 pins=[];differences=[]
 for va,size in sorted(visited.items()):
  if va in seams or not(0x401000<=va<0x9ef000):continue
  a=original.range(va,size);b=modified.range(va,size)
  if a!=b:differences.append(dict(va=hex(va),original=a.hex(),v14=b.hex()))
  pins.append(dict(va=hex(va),original=a.hex(),v14=b.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=pins,target_instruction_differences=differences,seams_not_executed=[hex(v)for v in sorted(seams)])
 path=HERE/'native-camera-variables.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),variables_per_row=17,pins=len(pins),target_instruction_differences=len(differences),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
