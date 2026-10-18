"""Actual42B6E0 ten constructor kinds, vf24/vf2C and shared-loader prefix.
Pool allocation85C680 and CString allocation are explicit successful services.
Real vf14 runs only to its first42C1B0 call (required stop); the subtype body
after common LoadData is unproved/unbound. False vf24/vf2C leaves authorize
resource-request projection only, never runtime FX command execution.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm

class ReachedBase(Exception):pass

def run(kind,image,pc,poison):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 allocations=[]
 def alloc(mm):
  p=mm.alloc(0x100);mm.u.mem_write(p,bytes([poison])*0x100);allocations.append(p);mm.ret(p)
 m.seam(0x85c680,alloc)
 helper=m.alloc(4);vt=m.alloc(0x10);m.w(helper,vt);m.w(vt+0xc,m.stub(lambda mm:mm.ret(mm.alloc(0x20))))
 m.seam(0x856370,lambda mm:mm.ret(helper))
 p=m.call(0x42b6e0,0,[kind],pop=0);assert p==allocations[0]
 table=m.r(p);methods={f'{o:x}':m.r(table+o)for o in [4,0x14,0x20,0x24,0x2c]}
 for off in [4,0x14,0x20,0x24,0x2c]:assert nm.original.range(table+off,4)==nm.modified.range(table+off,4)
 output=dict(kind=kind,vtable=f'{table:08x}',methods={k:f'{v:08x}'for k,v in methods.items()},
  vf24=m.call(methods['24'],p,[])&255,vf2c=m.call(methods['2c'],p,[])&255,
  words={f'{o:x}':m.r(p+o)for o in [8,0xc,0x10,0x14,0x18,0x1c,0x20,0x24,0x28,0x2c,0x30,0x34]},bytes={'4':m.rb(p+4),'38':m.rb(p+0x38)})
 source=m.alloc(4);context=m.alloc(4)
 def base(mm):
  assert mm.ecx()==p and mm.arg(1)==source and mm.arg(2)==context
  raise ReachedBase()
 m.seam(0x42c1b0,base)
 reached=False
 try:m.call(methods['14'],p,[source,context])
 except ReachedBase:reached=True
 output['loader_common_first']=reached
 assert reached
 return output,m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for kind in range(10):
   for poison in [0,0xa5,0xff]:
    a,pa=run(kind,'original',pc,poison);b,pb=run(kind,'v14',pc,poison);assert a==b
    assert(a['vf24'],a['vf2c'])==((1,0)if kind==0 else(0,1)if kind==3 else(0,0))
    rows.append(dict(precision=pc,poison=poison,output=a));pins.update(pa);pins.update(pb)
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 caller=0x42ba17;raw=nm.original.range(caller,0x10);assert raw==nm.modified.range(caller,len(raw))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),
  'static_caller_vf14':{'va':hex(caller),'bytes':raw.hex()},
  'required_native_stops':['0x42c1b0'],
  'successful_allocation_services':['0x85c680','0x856370']};path=HERE/'native-request-only-fxc.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
