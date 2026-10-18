from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import PE0
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
decoder=Cs(CS_ARCH_X86,CS_MODE_32)
for address,size in [(0x47e260,0xe0),(0x86f150,0xd0),(0x869420,0x90),(0x870c10,0x90),(0x4516a0,0x80),(0x870720,0xe0),(0x5a5410,0x240),(0x878630,0x160),(0x878790,0x240),(0x5a99c0,0x80),(0x878f30,0x140)]:
 print(hex(address))
 for x in decoder.disasm(PE0.range(address,size),address):print(f'{x.address:08x} {x.bytes.hex()} {x.mnemonic} {x.op_str}')
print('Nodevt',[(hex(i),hex(int.from_bytes(PE0.range(0xab6950+i,4),'little')))for i in range(0,0x48,4)])
print('Bonevt',[(hex(i),hex(int.from_bytes(PE0.range(0xa0d3b4+i,4),'little')))for i in range(0,0x28,4)])
print('Nodeupdatevt',[(hex(i),hex(int.from_bytes(PE0.range(0xab6950+i,4),'little')))for i in range(0x48,0x90,4)])
