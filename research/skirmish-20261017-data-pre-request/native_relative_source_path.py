"""Actual4763C0 source-file-relative path operation, both PE/allPC.
CString allocation/assignment/printf/trimming are explicit child services.
No85A030 normalization, source-file discovery or Resource load claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX

def run(folder,value,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 def string(value):
  raw=value.encode('utf-16-le');p=m.alloc(len(raw)+20);m.w(p+4,len(raw)//2);m.w(p+12,1000);m.u.mem_write(p+16,raw+b'\0\0');return p+16
 def text(p):
  raw=bytearray()
  for i in range(4096):
   unit=bytes(m.u.mem_read(p+i*2,2))
   if unit==b'\0\0':return raw.decode('utf-16-le')
   raw+=unit
  raise AssertionError('fixture child CString unexpectedly unterminated')
 owner=m.alloc(8);argument=m.alloc(4);m.w(owner+4,string(folder));m.w(argument,string(value))
 empty=string('');helper=m.alloc(4);vt=m.alloc(16);m.w(helper,vt);m.w(vt+12,m.stub(lambda mm:mm.ret(empty-16)))
 m.seam(0x856370,lambda mm:mm.ret(helper))
 def copy(mm):
  target=mm.u.reg_read(UC_X86_REG_ECX);mm.w(target,mm.r(mm.arg(1)));mm.ret(target,4)
 m.seam(0x405b80,copy)
 def format_(mm):
  target=mm.arg(1);assert text(mm.arg(2))=='%s/%s';mm.w(target,string(text(mm.arg(3))+'/'+text(mm.arg(4))));mm.ret(target,0)
 m.seam(0x405d60,format_)
 def trim(mm):
  assert mm.arg(1)==47;target=mm.u.reg_read(UC_X86_REG_ECX);mm.w(target,string(text(mm.r(target)).lstrip('/')));mm.ret(target,4)
 m.seam(0x424c10,trim)
 m.call(0x4763c0,owner,[argument])
 return text(m.r(argument)),m.visited

def main():
 rows=[];pins={}
 for pc,folder,value in itertools.product(nm.PRECISIONS,['','Units/GBR','VFX\\Water'],['','x.dds','/x.dds','///Shared/x.dds','../Shared/x.dds','%s.dds','x\\y.nif']):
  a,pa=run(folder,value,'original',pc);b,pb=run(folder,value,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,folder=folder,value=value,output=a))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-relative-source-path.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
