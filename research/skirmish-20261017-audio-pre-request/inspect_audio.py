from pathlib import Path
import sys,struct
R=Path.cwd();sys.path.insert(0,str(R/'research/phase8-ai'))
from native_machine import original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
C=Cs(CS_ARCH_X86,CS_MODE_32)
for value in [0xb3d76c,0xa0a318]:
 raw=struct.pack('<I',value);p=0;hits=[]
 while True:
  p=original.b.find(raw,p)
  if p<0:break
  for _,rva,size,off in original.sections:
   if off<=p<off+size:hits.append(original.base+rva+p-off);break
  p+=1
 print('REF',hex(value),[hex(x)for x in hits])
 for hit in hits[:12]:
  print('\n',hex(hit))
  for i in C.disasm(original.range(hit-12,45),hit-12):print(hex(i.address),i.mnemonic,i.op_str)
for a,n in [(0x9d0900,0x500),(0x59b4b0,0x60)]:
 print('\nBODY',hex(a))
 for i in C.disasm(original.range(a,n),a):print(hex(i.address),i.mnemonic,i.op_str)
