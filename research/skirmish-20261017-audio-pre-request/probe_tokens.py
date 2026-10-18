from pathlib import Path
import sys
R=Path.cwd();sys.path.insert(0,str(R/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(R/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn.x86_const import *
full.order.T.P=h.P;m=h.Machine();m.thiscall(0x4013e0,0xb43b40,[])
slot=m.alloc(4);m.w(slot,m.cstring('audio'));audio=m.alloc(0x88);m.thiscall(0x46b400,audio,[slot]);m.w(audio+0x18,0)
ctx=m.alloc(16);m.w(ctx+4,m.cstring('Data/Audio'));m.w(0xb36530,ctx)
trace=[]
def sound(m,u):
 p=m.r(u.reg_read(UC_X86_REG_ESP)+4);trace.append(m.wstr(m.r(p)));res=m.alloc(0x44);m.ret(res)
m.stubs[0x59b4b0]=sound
for text in ['', 'A.wav,B.wav', '  A.wav,, B.wav , ', 'A.wav\tB.wav','/Sounds/General/x.wav']:
 m.w(audio+0x38,0);value=m.alloc(4);m.w(value,m.cstring(text));trace.clear()
 try:
  m.thiscall(0x46bb60,audio,[audio+0x34,value]);print(repr(text),trace,'count',m.r(audio+0x38))
 except Exception as e:print(type(e).__name__,e);break
