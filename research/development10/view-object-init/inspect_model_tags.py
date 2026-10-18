from pathlib import Path
import sys
p=Path('research/development10/view-object-init');sys.path.insert(0,str(p.parent/'creator-labels'))
from native_creator_labels import PE0
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,s in [(0x87a430,0x120),(0x87a000,0x70),(0x879ad0,0x25)]:
 for x in c.disasm(PE0.range(a,s),a):print(f'{x.address:08X} {x.bytes.hex()} {x.mnemonic} {x.op_str}')
for a in [0xab7670,0xab763c,0xab7648,0xab7654,0xab7660,0xa88720]:print(hex(a),repr(PE0.range(a,40)))
