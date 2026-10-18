from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
m=h.Machine()
from unicorn.x86_const import UC_X86_REG_ESP
def increment(m,u):
 p=m.r(u.reg_read(UC_X86_REG_ESP)+4);v=(m.r(p)+1)&0xffffffff;m.w(p,v);m.ret(v,pop=4)
m.stubs[0x301f750]=increment;m.w(0x9f01e8,0x301f750)
m.thiscall(0x4013e0,0xb43b40,[])
from unicorn.x86_const import UC_X86_REG_EIP
name=m.alloc(4);m.w(name,m.cstring('Textures/UI/Circle.tga'));r=m.alloc(0x100);
try:print(m.thiscall(0x5a7db0,r,[name]))
except Exception as e:print(type(e).__name__,str(e),'eip',hex(m.u.reg_read(UC_X86_REG_EIP)));raise
print({hex(i):hex(m.r(r+i))for i in range(0,0x68,4)})
t=m.r(r+0x20);print('task',{hex(i):hex(m.r(t+i))for i in range(0,0x20,4)})
s=m.r(r+0x80);print('model',{hex(i):hex(m.r(r+i))for i in range(0x3c,0x90,4)});print('slots',{hex(i):hex(m.r(s+i))for i in range(0,8,4)})


m.cdecl(0x9d0900,[])
for i in range(30):
 ptr=m.r(0xb282f8+i*8);cl=m.r(0xb282fc+i*8)
 if not cl:break
 print('ext',m.raw_wstr(ptr),hex(cl))
