"""Native Renderer24C/24B result branch7111D2..711218 after actual D3D depth-
stencil CreateSurface HRESULT. Independent required D3D child is an explicit
input, not a Vulkan/wgpu compatibility claim. Captures source diagnostic
literal and ordered native stores for no-stencil0 versus8-bit-stencil7F.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parents[1]/'research/phase8-ai'))
from native_machine import Machine,original,modified,PRECISIONS
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EAX,UC_X86_REG_ESP
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
rows=[];pins={}
for image,pc,result in itertools.product(('original','v14'),PRECISIONS,(0,1,0x80004005,0xffffffff)):
 m=Machine(pc)
 if image=='v14':
  old=(original.image_size+4095)&~4095;new=(modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(original.base+old,new-old)
  for vs,rva,raw,off in modified.sections:m.u.mem_write(modified.base+rva,modified.b[off:off+raw])
 p=m.alloc(0x300);m.w(p+0x24c,0xa5a5a5a5);m.wb(p+0x24b,0xa5);stack=m.alloc(0x1000)+0x800
 m.u.reg_write(UC_X86_REG_ESI,p);m.u.reg_write(UC_X86_REG_EAX,result);m.u.reg_write(UC_X86_REG_ESP,stack)
 m.u.emu_start(0x7111d2,0x711218)
 stencil=m.r(p+0x24c);present=m.rb(p+0x24b);sp=m.u.reg_read(UC_X86_REG_ESP);message=m.r(sp+8)
 assert stencil==(0 if result&0x80000000 else 0x7f);assert present==(0 if result&0x80000000 else 1)
 def wide(a):
  out=[]
  while m.u.mem_read(a,2)!=b'\0\0':out.append(bytes(m.u.mem_read(a,2)));a+=2
  return b''.join(out).decode('utf-16le')
 text=wide(message)
 assert text==('Using no stencil buffer'if result&0x80000000 else'Using 8-bit stencil buffer')
 rows.append(dict(image=image,precision=pc,hresult=result,mask_24c=stencil,byte_24b=present,message_address=hex(message),message=text))
for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(original.range(0x7111d2,0x711218-0x7111d2),0x7111d2):
 assert original.range(i.address,i.size)==modified.range(i.address,i.size);pins[hex(i.address)]=i.bytes.hex()
doc=dict(scope=__doc__,rows=rows,instructions=pins)
p=HERE/'native-camera-stencil-branch.json'
if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
else:assert json.loads(p.read_text())==doc
print(json.dumps(dict(rows=len(rows),pin_interval_instructions=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
