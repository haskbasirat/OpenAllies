from pathlib import Path
import capstone,hashlib,json,sys
game=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(game/'rust-rebuild/research/phase4-body'))
from original_body import IMAGE,ACCEPTED
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_32)
for pe in [IMAGE,ACCEPTED]:
 code=list(md.disasm(pe.at(0x48a8e0,0x1600),0x48a8e0))
 print('image sha256',hashlib.sha256(pe.data).hexdigest())
 for i,ins in enumerate(code):
  if ins.mnemonic.startswith('ret'):break
  if '0x60c' in ins.op_str or (ins.mnemonic=='call' and ins.op_str in ['0x480ed0','0x481020','0x481170','0x47f130','0x47f280','0x4812c0']):
   print('\n'.join(f'{x.address:08X} {x.bytes.hex()} {x.mnemonic} {x.op_str}'for x in code[max(0,i-4):i+3]))
