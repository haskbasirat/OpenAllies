"""Read-only original/v14 instruction windows for concrete saved Init owners."""
from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/python-packages'))
from audit_baseline import PE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
original=PE(ROOT.parent/'AA.exe');v14=PE(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
cs=Cs(CS_ARCH_X86,CS_MODE_32)
def show(start,length):
 raw=original.range(start,length);assert raw==v14.range(start,length)
 return [f'{i.address:08X} {i.bytes.hex()} {i.mnemonic} {i.op_str}' for i in cs.disasm(raw,start)]
def main():
 if len(sys.argv)>1:
  start=int(sys.argv[1],16);length=int(sys.argv[2],16) if len(sys.argv)>2 else 0x100
  print('\n'.join(show(start,length)));return
 # Locate LEA world-vector arguments hidden by decompiler thiscall recovery.
 for _,r,size,off in original.sections:
  if original.base+r!=0x401000:continue
  raw=original.b[off:off+size]
  for displacement in (0x160,0x164,0x3f0,0x3f4):
   data=displacement.to_bytes(4,'little');at=0
   while (at:=raw.find(data,at))>=0:
    if at>=2 and raw[at-2]==0x8d and 0x80<=raw[at-1]<=0xbf and raw[at-1]&7!=4:
     va=original.base+r+at-2
     if 0x4e0000<=va<0x510000:print('\n'.join(show(va,0x28)))
    at+=4
if __name__=='__main__':main()
