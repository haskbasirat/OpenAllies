from pathlib import Path
import sys,struct
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
pattern=struct.pack('<I',0xb43b40);start=0;c=Cs(CS_ARCH_X86,CS_MODE_32)
while True:
 o=h.P.b.find(pattern,start)
 if o<0:break
 start=o+1
 try:va=next(h.P.base+r+o-off for vs,r,raw,off in h.P.sections if off<=o<off+raw)
 except:continue
 if 0x9b0000<va<0x9ef000:
  print('xref',hex(va))
  for i in c.disasm(h.P.range(va-1,20),va-1):print(hex(i.address),i.mnemonic,i.op_str)
