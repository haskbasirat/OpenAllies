"""Whole-image capstone scan for instructions referencing an absolute address.
Usage: python xrefs.py <hex addr> [...]"""
from pathlib import Path
import sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
from audit_baseline import PE
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

P = PE(ROOT.parent / 'AA.exe')


def text_range():
    # .text is the first section; scan from image base+0x1000 to the end of code.
    return 0x401000, 0x98f000


def scan(targets):
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.skipdata = True
    start, end = text_range()
    code = P.range(start, end - start)
    needles = {t: t.to_bytes(4, 'little') for t in targets}
    hits = []
    for t, nb in needles.items():
        i = code.find(nb)
        while i != -1:
            # disassemble a window before the hit to find the instruction containing it
            for back in range(1, 12):
                a = start + i - back
                ins = next(md.disasm(P.range(a, 16), a), None)
                if ins and ins.address + ins.size >= start + i + 4 and hex(t) in ins.op_str:
                    hits.append((t, ins.address, f'{ins.mnemonic} {ins.op_str}'))
                    break
            i = code.find(nb, i + 1)
    return hits


if __name__ == '__main__':
    for t, a, s in scan([int(x, 16) for x in sys.argv[1:]]):
        print(f'{t:08x} {a:08x} {s}')
