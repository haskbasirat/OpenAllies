"""Whole49C760 ObjectData constructor relevant request-field baseline.
4BBE70, shape/list child constructors and vector iterator execute; CString
allocation/reference transport and heap allocation are explicit services.
Current SVar shape value is a bounded field input, not a startup-source claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm

def run(image,pc,poison):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 data=m.alloc(0x180);m.u.mem_write(data,bytes([poison])*0x180)
 header=m.alloc(32);m.w(header+12,100);m.u.mem_write(header+16,b'a\0\0\0');ids=m.array([header+16])
 helper=m.alloc(4);vt=m.alloc(16);m.w(helper,vt);m.w(vt+12,m.stub(lambda mm:mm.ret(header)))
 m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1)));m.seam(0x856370,lambda mm:mm.ret(helper))
 m.seam(0x6f7810,lambda mm:mm.ret(mm.alloc(mm.arg(1))))
 variables=m.alloc(0x34);cell=m.alloc(4);m.w(0xb39ab0,variables);m.w(variables+0x30,cell);m.w(cell,0x3e4ccccd)
 result=m.call(0x49c760,data,[ids,m.alloc(4)])
 out={f'{o:x}':m.r(data+o)for o in [0x1c,0x20,0xec,0xf0,0x104,0x110,0x118,0x134,0x138]}
 out['fc']=m.rb(data+0xfc);out['return_same']=result==data
 out['fx_pairs']=[m.r(data+0xbc+4*i)for i in range(12)]
 return out,m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for poison in [0,0xa5,0xff]:
   a,pa=run('original',pc,poison);b,pb=run('v14',pc,poison);assert a==b
   rows.append(dict(precision=pc,poison=poison,output=a));pins.update(pa);pins.update(pb)
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins)};path=HERE/'native-object-request-constructor.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
