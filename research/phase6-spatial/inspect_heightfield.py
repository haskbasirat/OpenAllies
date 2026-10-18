"""Inspect original heightfield construction/serialization without process mutation."""
from pathlib import Path
import sys, struct
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
p=Pe(ROOT.parent/'AA.exe', ORIGINAL_SHA)
q=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe', ACCEPTED_SHA)
vtable=struct.unpack('<6I',p.at(0xa3a870,24))
print('vtable', [hex(v) for v in vtable])
ranges=[(0x5519a0,414),(0x4f2915,90),(vtable[0],160),(vtable[1],300)]
dis=Cs(CS_ARCH_X86,CS_MODE_32)
for address,size in ranges:
    raw=p.at(address,size)
    assert raw==q.at(address,size)
    print('\n',hex(address))
    for i in dis.disasm(raw,address):print(f'{i.address:08x} {i.mnemonic} {i.op_str}')
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools'))
from map_extract import read_chunks
source=ROOT.parent/'reconstruction/assets/original/Data/Maps/Campaign/axis01_crete.rmp'
chunks=read_chunks(source)
for key in ['HEIGHT_MAP','MOUNTAIN_HEIGHT_MAP']:
    raw=chunks[key][0]
    print(key,len(raw),[hex(x) for x in struct.unpack_from('<6I',raw)],struct.unpack_from('<6f',raw))
