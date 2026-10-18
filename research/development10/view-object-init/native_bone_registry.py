"""Actual Bone486650 manager/4867B0 load/486870 append and47E270 name loads.

TGI named-field reads and ASCII-only wide-to-narrow conversion are explicit
transports. Allocators/CString transports are inherited from the native
machine. No native exception allocator, non-ASCII conversion or template
parser claim is made. Both images/allPC execute the registry/vector stores.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ECX
class Stop(Exception):pass
class Bones(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'bone_ready',False):
   if pc in [0x5b54d0,0x5b5620]:
    name=self.text(self.arg()).lower()
    assert name in ['ids','name','bone_name'],name
    value=self.current.get(name,'');cell=self.alloc(4);self.cs(cell,value)
    self.ret(4,cell);return
   if pc==0x454810:
    out=u.reg_read(UC_X86_REG_ECX);value=self.text(self.arg());assert value.isascii()
    self.cs(out,value);u.mem_write(self.r(out),value.encode('ascii')+b'\0');self.ret(4,out);return
   if pc==0x410c40:raise Stop('native duplicate IDS error cutoff')
  super().hook(u,pc,size,user)
def run(m,pc,inputs):
 m.fpu(pc);m.actual={};m.record_actual=True;m.bone_ready=False
 manager=m.alloc(0x4c);m.call(0x486650,[0x11,0],this=manager,callee_pop=8)
 assert m.r(manager)==0xa0d3b4 and m.r(manager+8)==0 and m.r(manager+0x28)==0
 m.bone_ready=True;error=None
 for row in inputs:
  m.current=row
  try:m.call(0x4867b0,[m.alloc(0x20)],this=manager,callee_pop=4)
  except Stop as e:error=str(e);break
 entries=[]
 for i in range(m.r(manager+0x28)):
  node=m.r(m.r(manager+0x24)+4*i)
  p=m.r(node+0x1c);raw=bytearray()
  while m.u.mem_read(p+len(raw),1)!=b'\0':raw.extend(m.u.mem_read(p+len(raw),1))
  entries.append(dict(ids=m.rs(node+8),bone_name=raw.decode('ascii')))
 return dict(mode=m.r(manager+8),entries=entries,error=error),m.actual
def main():
 rows=[];pins={}
 cases=[[dict(ids='position',bone_name='Scene Root'),dict(ids='base',bone_name='base'),dict(ids='selection_box',bone_name='SelectionBox')],
        [dict(ids='b',bone_name='B'),dict(ids='a',bone_name='A')],
        [dict(ids='b',bone_name='B'),dict(ids='b',bone_name='second')]]
 for image in ['original','v14']:
  m=Bones(image)
  for pc in PC:
   for inputs in cases:
    output,actual=run(m,pc,inputs);pins.update(actual);rows.append(dict(image=image,precision=pc,input=inputs,output=output))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-bone-registry.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
