"""Disassemble original AA.exe ranges (read-only research helper).
usage: python dis.py ADDR [ADDR...] (hex); stops at first ret unless --n N instructions."""
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'research/phase4-body'))
from original_body import IMAGE  # noqa
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
md = Cs(CS_ARCH_X86, CS_MODE_32)
args = sys.argv[1:]
n = 400
if '--n' in args:
    i = args.index('--n'); n = int(args[i + 1]); del args[i:i + 2]
full = '--full' in args
args = [a for a in args if a != '--full']
for a in args:
    addr = int(a, 16)
    raw = IMAGE.at(addr, n * 8)
    print(f'=== {addr:08x}')
    count = 0
    for ins in md.disasm(raw, addr):
        print(f'{ins.address:08x}: {ins.mnemonic} {ins.op_str}')
        count += 1
        if count >= n or (not full and ins.mnemonic == 'ret'):
            break
