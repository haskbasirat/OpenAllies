from pathlib import Path
import sys,struct,json
sys.path.insert(0,str(Path('research/skirmish-20261017-svar-field-order').resolve()));import native_full_sources as f
h=f.order.T
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
p=h.P;cs=Cs(CS_ARCH_X86,CS_MODE_32)
for address in [0xb282f8,0xb282fc,0xb28300]:
 target=struct.pack('<I',address);at=0
 print('TARGET',hex(address))
 while True:
  at=p.b.find(target,at)
  if at<0:break
  va=None
  for vs,rva,size,off in p.sections:
   if off<=at<off+size:va=p.base+rva+at-off;break
  if va is not None:
   print('REF',hex(va),p.range(va-10,25).hex())
   for start in [va-2,va-1,va-6]:
    for i in list(cs.disasm(p.range(start,20),start))[:3]:print(hex(i.address),i.mnemonic,i.op_str)
  at+=4

