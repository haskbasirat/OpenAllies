"""Actual twelve native property constructors (default wrappers) and camera mutations.

Runs real allocation-success constructor bodies; typed Rust ownership replaces
native reference counts/vtable addresses. Partial Fog dependency cutoffs are
pre-instruction observations, before each reached source global load.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_camera_reset as R
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
CTORS=[0x87d060,0x8898a0,0x889c70,0x87d500,0x88a7d0,0x889ef0,0x88a110,0x889550,0x86d690,0x87b7d0,0x86e7b0,0x88a3c0]
GLOBALS=[0xb43f3c,0xb442f8,0xb44304,0xb43f48,0xb44334,0xb44310,0xb4431c,0xb442ec,0xb43dfc,0xb43f1c,0xb43e08,0xb44328]
SIZES=[0x24,0x24,0x34,0x64,0x24,0x24,0x24,0x40,0x48,0x2c,0x24,0x28]
FOG=[0x3f800000,0x3f000000,0xc0000000]
class Cut(Exception):pass

def run(image,pc,seed,cut,hw):
 m=R.Machine(image,pc,hw);m.actual={};writes=[];allocations=[]
 def alloc(x):
  size=x.arg(1);p=x.alloc(size);x.u.mem_write(p,b'\xa5'*size);allocations.append((p,size));x.ret(p)
 m.seam(0x6f7810,alloc);m.seam(0x6f7430,lambda x:x.ret());m.w(0xb323c0,seed)
 for a,b in zip((0xb43e68,0xb43e6c,0xb43e70),FOG):m.w(a,b)
 def written(u,access,a,size,value,_):writes.append((a,size))
 m.u.hook_add(UC_HOOK_MEM_WRITE,written)
 if cut is not None:m.observe([0x889ca5,0x889cad,0x889cb6][cut],lambda x:(_ for _ in ()).throw(Cut()))
 output=[]
 for slot,address in enumerate(CTORS):
  if cut is not None and slot!=2:continue
  stopped=False
  try:m.call(address)
  except Cut:stopped=True
  p=next(p for p,n in reversed(allocations)if n==SIZES[slot])if stopped else m.r(GLOBALS[slot]);known=[0]* (SIZES[slot]//4)
  for a,size in writes:
   for b in range(max(a,p),min(a+size,p+SIZES[slot])):known[(b-p)//4]|=0xff<<(8*((b-p)&3))
  words=[m.r(p+4*i)for i in range(SIZES[slot]//4)]
  output.append(dict(slot=slot,words=words,known=known,stopped=stopped))
  if cut is not None:assert stopped
 return dict(properties=output,counter=m.r(0xb323c0)),m.actual

def main():
 rows=[];pins={};hw=R.Hardware()
 try:
  for image,pc in itertools.product(('original','v14'),R.PRECISIONS):
   for seed in(1,0xfffffffe,0xffffffff):
    out,visited=run(image,pc,seed,None,hw);rows.append(dict(image=image,precision=pc,seed=seed,cut=None,output=out));pins.update(visited)
   for cut in(0,1,2):
    out,visited=run(image,pc,1,cut,hw);rows.append(dict(image=image,precision=pc,seed=1,cut=cut,output=out));pins.update(visited)
 finally:hw.close()
 for a,b in pins.items():assert R.original.range(a,len(bytes.fromhex(b))).hex()==R.modified.range(a,len(bytes.fromhex(b))).hex()==b
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),fog_words=FOG,rows=rows,instructions={hex(a):b for a,b in sorted(pins.items())})
 path=HERE/'native-render-properties.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),property_constructors=18*12+18,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
