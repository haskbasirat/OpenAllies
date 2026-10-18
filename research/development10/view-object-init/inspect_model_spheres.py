from pathlib import Path
import sys
p=Path('research/development10/view-object-init');sys.path.insert(0,str(p.parent/'creator-labels'))
from native_creator_labels import PE0
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,s in [(0x89fad0,0x130),(0x89fbf0,0xd0),(0x5a9610,0x220),(0x866eb0,0xc0),(0x868020,0x60)]:
 print(hex(a))
 for x in c.disasm(PE0.range(a,s),a):print(f'{x.address:08X} {x.bytes.hex()} {x.mnemonic} {x.op_str}')
