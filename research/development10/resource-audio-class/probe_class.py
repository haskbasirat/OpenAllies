from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn.x86_const import *
full.order.T.P=h.P;m=h.Machine()
m.stubs[0x96cab0]=lambda m,u:m.ret()
def ascii_name(m,u):
 p=m.r(u.reg_read(UC_X86_REG_ESP)+4);text=bytearray()
 while m.u.mem_read(p,1)!=b'\0':text.extend(m.u.mem_read(p,1));p+=1
 assert text==b'KKC_AudioResource'
 out=u.reg_read(UC_X86_REG_ECX);m.w(out,m.cstring(text.decode('ascii')));m.ret(out,pop=4)
m.stubs[0x4094f0]=ascii_name
m.w(0xb38e08,0);m.w(0xb38e0c,0);m.w(0xb38e10,2);m.w(0xb38e14,m.alloc(31*4));m.w(0xb38e18,31);m.w(0xb38e1c,0)
try:m.cdecl(0x9d02f0,[])
except Exception:
 print('FAIL',hex(m.u.reg_read(UC_X86_REG_EIP)),m.null_accesses);raise
print('descriptor',m.r(0xb3d76c),m.wstr(m.r(0xb3d770)),hex(m.r(0xb3d774)),hex(m.r(0xb3d77c)))
