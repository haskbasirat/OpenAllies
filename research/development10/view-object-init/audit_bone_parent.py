from pathlib import Path
import sys,json,hashlib
p=Path('research/development10/view-object-init');sys.path.insert(0,str(p.parent/'creator-labels'))
from native_creator_labels import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32);ins=list(c.disasm(PE0.range(0x48a8e0,0x1c00),0x48a8e0));matches=[]
for i,x in enumerate(ins):
 if '0x580' in x.op_str:
  for y in ins[max(0,i-12):i+2]:
   assert PE0.range(y.address,y.size)==PE1.range(y.address,y.size)
   matches.append(dict(va=hex(y.address),bytes=y.bytes.hex(),instruction=y.mnemonic+' '+y.op_str))
print(json.dumps(dict(scope='Static both-PE GameData48A8E0 parent callsite: actual Bone manager allocation/486650 mode0 then field580 publication. Does not execute whole GameData constructor or certify arbitrary aliases.',pins=matches),indent=2))
