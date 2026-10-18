"""Linear disassembly helper for the saved-component research (Capstone over
the original AA.exe image). Usage: python disasm.py <va> [<end-va>|+<bytes>]
Stops at the first int3 run after a ret when no end is given."""
from pathlib import Path
import sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
from audit_baseline import PE
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

P = PE(ROOT.parent / 'AA.exe')


def listing(start, end=None):
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    size = (end - start) if end else 0x4000
    code = P.range(start, size)
    out = []
    last_ret = False
    for ins in md.disasm(code, start):
        if end is None and last_ret and ins.mnemonic == 'int3':
            break
        out.append(f'{ins.address:08x} {ins.mnemonic:7} {ins.op_str}')
        last_ret = ins.mnemonic == 'ret'
        if end is None and ins.mnemonic in ('ret',) and False:
            break
    return out


if __name__ == '__main__':
    start = int(sys.argv[1], 16)
    end = None
    if len(sys.argv) > 2:
        a = sys.argv[2]
        end = start + int(a[1:], 16) if a.startswith('+') else int(a, 16)
    print('\n'.join(listing(start, end)))
