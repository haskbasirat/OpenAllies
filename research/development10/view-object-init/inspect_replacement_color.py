from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import PE0
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from capstone.x86_const import X86_OP_IMM
md=Cs(CS_ARCH_X86,CS_MODE_32);md.detail=True
ins=list(md.disasm(PE0.range(0x401bd0,0x3000),0x401bd0))
for i,x in enumerate(ins):
 if x.mnemonic!='push' or not x.operands or x.operands[0].type!=X86_OP_IMM:continue
 va=x.operands[0].imm
 if PE0.base<=va<PE0.base+PE0.image_size:
  if PE0.range(va,48).startswith('ReplacementColorDefault'.encode('utf-16le')):
   for a in ins[max(0,i-25):i+28]:print(f'{a.address:08x} {a.bytes.hex()} {a.mnemonic} {a.op_str}')
