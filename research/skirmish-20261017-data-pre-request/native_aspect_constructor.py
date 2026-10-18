"""Ordinary Aspect46A510 constructor relevant field stores, with explicit
CString allocation/copy services and NONNULL actual-owner input transport.
No Object/ActorData source Loader, Texture or Model decoder completion.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX

def run(image,pc,poison):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 aspect=m.alloc(0x6c);m.u.mem_write(aspect,bytes([poison])*0x6c)
 owner=m.alloc(0xb4);text=m.alloc(0x20);name=m.alloc(4);m.w(name,text+0x10);m.w(owner+0xb0,text+0x10)
 allocator=m.alloc(4);vt=m.alloc(0x10);m.w(allocator,vt)
 def new(mm):mm.ret(mm.alloc(0x20),0)
 m.w(vt+0xc,m.stub(new));m.seam(0x856370,lambda mm:mm.ret(allocator,0))
 m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1),0))
 def copy(mm):
  mm.w(mm.u.reg_read(UC_X86_REG_ECX),mm.r(mm.arg(1)));mm.ret(0,4)
 m.seam(0x405b80,copy)
 result=m.call(0x46a510,aspect,[owner,5,name])
 out={f'{o:x}':m.r(aspect+o) for o in [8,0x10,0x18,0x24,0x28,0x2c,0x30,0x34,0x38,0x3c,0x40,0x44,0x48,0x4c,0x50,0x58,0x5c,0x60,0x64]}
 out.update({f'{o:x}':m.rb(aspect+o) for o in [0x14,0x1c,0x54,0x68]})
 out['8']='actual_owner';out['return_same']=result==aspect
 return out,m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for poison in [0,0xa5,0xff]:
   a,pa=run('original',pc,poison);b,pb=run('v14',pc,poison);assert a==b
   rows.append(dict(precision=pc,poison=poison,output=a));pins.update(pa);pins.update(pb)
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins)}
 path=HERE/'native-aspect-constructor.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
