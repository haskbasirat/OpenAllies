"""Print the 546AF0 switch case block(s) for opcodes (read-only research helper)."""
import struct, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'research/phase4-body'))
from original_body import IMAGE  # noqa
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
md = Cs(CS_ARCH_X86, CS_MODE_32)
def target(op):
    return struct.unpack('<I', IMAGE.at(0x548730 + op * 4, 4))[0]
for a in sys.argv[1:]:
    op = int(a, 0)
    t = target(op)
    print(f'=== opcode {op} -> {t:08x}')
    for ins in md.disasm(IMAGE.at(t, 600), t):
        print(f'{ins.address:08x}: {ins.mnemonic} {ins.op_str}')
        if ins.mnemonic == 'ret' or (ins.mnemonic == 'jmp' and not ins.op_str.startswith('dword')):
            break
