from pathlib import Path
import sys,struct
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
from audit_baseline import PE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
pe=PE(ROOT.parent/'AA.exe')
cs=Cs(CS_ARCH_X86,CS_MODE_32)
address=int(sys.argv[1],16);size=int(sys.argv[2],16)
print('\n'.join(f'{i.address:08x} {i.mnemonic} {i.op_str}'for i in cs.disasm(pe.range(address,size),address)))
