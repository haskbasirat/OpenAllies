"""Actual46A650 BannerAspect constructor,469A90 lazy size-copy gate and
469CC0 native PC-specific FSQRT stores. CString allocation/reference transport
are explicit services, BannerData model/skin fields and normal ground-size
are bounded inputs. No fabricated Object/View or model-loaded payload.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX

def run(image,pc,poison,size):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,n,off in nm.modified.sections:
   if n:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+n])
 row=m.alloc(0x6c);m.u.mem_write(row,bytes([poison])*0x6c);data=m.alloc(0x30)
 header=m.alloc(0x30);m.w(header+4,2);m.w(header+12,100);m.u.mem_write(header+16,b'a\0b\0\0\0')
 text=header+16;ids=m.array([text]);m.w(data+0x18,text);m.w(data+0x1c,text);skin=m.alloc(4);m.w(data+0x20,skin)
 helper=m.alloc(4);vt=m.alloc(16);m.w(helper,vt);m.w(vt+12,m.stub(lambda mm:mm.ret(header)))
 m.seam(0x856370,lambda mm:mm.ret(helper));m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1)))
 def copy(mm):mm.w(mm.ecx(),mm.r(mm.arg(1)));mm.ret(0,pop=4)
 m.seam(0x405b80,copy)
 returned=m.call(0x46a650,row,[data,5,ids]);assert returned==row
 initial={f'{o:x}':m.r(row+o)for o in [8,0x10,0x18,0x24,0x28,0x2c,0x34,0x38,0x3c,0x40,0x44,0x48,0x4c,0x50,0x58,0x5c,0x60,0x64]}
 initial.update({f'{o:x}':m.rb(row+o)for o in [0x14,0x1c,0x54,0x68]});initial['30_is_banner_skin']=m.r(row+0x30)==skin
 normal=m.alloc(0x6c);m.w(normal+0x58,size[0]);m.w(normal+0x5c,size[1]);gate=m.call(0x469a90,normal,[])&255
 if gate:m.call(0x469cc0,row,[m.array(size)])
 output=dict(initial=initial,gate=gate,size=[m.r(row+0x58),m.r(row+0x5c)],radius=m.r(row+0x60))
 enums=[]
 def registration(mm):
  text=mm.arg(2);raw=bytearray()
  while bytes(mm.u.mem_read(text,2))!=b'\0\0':raw+=bytes(mm.u.mem_read(text,2));text+=2
  enums.append([mm.arg(1),raw.decode('utf-16le'),mm.arg(3)]);mm.ret(pop=12)
 m.seam(0x46d1b0,registration);m.call(0x46d2c0,m.alloc(64),[]);output['enum']=enums
 #Source-only Aspect destruction must not release/dereference these Model
 #or skin resources. Run the actual destructor (empty selected animation
 #allocation) and forbid reads of resource fields; heap frees are services.
 from unicorn import UC_HOOK_MEM_READ
 from unicorn.x86_const import UC_X86_REG_EIP
 resources=m.alloc(0x40);m.w(row+0x28,resources);m.w(row+0x30,resources+0x20);freed=[]
 def forbidden(u,access,address,size,value,unused):raise AssertionError(('unexpected Resource destructor read',hex(address),hex(u.reg_read(UC_X86_REG_EIP))))
 m.u.hook_add(UC_HOOK_MEM_READ,forbidden,begin=resources,end=resources+0x3f)
 m.seam(0x6f7430,lambda mm:(freed.append(mm.arg(1)),mm.ret()))
 m.call(0x46a770,row,[]);assert resources not in freed and resources+0x20 not in freed
 output['destructor_resource_reads']=0;output['destructor_resource_frees']=0
 return output,m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for poison in [0,0xa5,0xff]:
   for size in [[0,0],[0x80000000,0],[nm.f32(3),nm.f32(4)],[nm.f32(-1),nm.f32(-2)],[nm.f32(-1),nm.f32(2)],[nm.f32(1e-10),nm.f32(2e-10)]]:
    a,pa=run('original',pc,poison,size);b,pb=run('v14',pc,poison,size);assert a==b;rows.append(dict(precision=pc,poison=poison,size=size,output=a));pins.update(pa);pins.update(pb)
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins)};p=HERE/'native-banner-aspect.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
