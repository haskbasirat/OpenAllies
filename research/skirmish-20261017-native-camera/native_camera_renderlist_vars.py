"""Execute original41F9D0 registration prefix through13 RenderlistDraw Bool vars.
CString/heap and4B3470 storage are explicit transports; actual variable constructors execute.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_camera_variables as V
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
class Machine(V.L.Loader):
 def __init__(self,image,pc):
  V.L.IMAGE=V.ORIGINAL if image=='original'else V.MODIFIED;super().__init__(V.PRECISIONS[pc]);self.fast=self.alloc(0x1000);self.manager=self.alloc(0x40);self.w(self.manager+8,self.fast);self.w(0xb39938,self.manager);self.w(0xb39940,self.fast);self.rows=[];self.selected=[];self.actual={};self.seam(0x4b3470,self.register);self.u.hook_add(UC_HOOK_CODE,self.actual_hook)
 def actual_hook(self,u,pc,size,_):
  if pc not in self.seams and u.reg_read(UC_X86_REG_EIP)==pc and V.original.base<=pc<V.original.base+V.original.image_size:self.actual[pc]=size
 def register(self):
  index=self.arg(0);p=self.arg(1);name=self.text(self.r(p+0x1c));row=dict(index=index,name=name,kind=self.r(p+0x24),flags=self.r(p+0x28),word=self.r(p));var=self.alloc(0x2c);self.u.mem_write(var,bytes(self.u.mem_read(p,0x2c)));slot=index if index!=0xffffffff else len(self.rows);self.w(self.fast+slot*4,var);self.rows.append(row);self.w(self.manager+12,len(self.rows))
  if name.startswith('RenderlistDraw'):self.selected.append(row)
  if name=='RenderlistDrawUICatchAll':raise V.Done()
  self.ret(8,var)
 def run(self):
  try:self.call(0x41f9d0,self.manager)
  except V.Done:pass
  assert len(self.selected)==13
  for r in self.selected:assert r['kind']==1 and r['flags']==9 and r['word']&255==1,r
  return self.selected

def main():
 rows=[];pins={};seams=set()
 for image,pc in itertools.product(('original','v14'),V.PRECISIONS):
  m=Machine(image,pc);out=m.run();rows.append(dict(image=image,precision=pc,variables=out));pins.update(m.actual);seams|=set(m.seams)
 instructions={hex(a):V.original.range(a,n).hex() for a,n in sorted(pins.items())}
 for a,n in pins.items():assert V.original.range(a,n)==V.modified.range(a,n)
 assert all(r['variables']==rows[0]['variables'] for r in rows)
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,transports=[hex(a)for a in sorted(seams)])
 path=HERE/'native-camera-renderlist-vars.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),variables=13,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
