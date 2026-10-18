"""Read-only disassembly helper for the original AA.exe (never modified).

python natdis.py <hex-va> [count]     linear disassembly from VA
python natdis.py fn <hex-va>          whole function using functions.tsv body size
"""
from pathlib import Path
import csv, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]; GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from audit_baseline import PE
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
PE0 = PE(GAME / 'AA.exe')
CS = Cs(CS_ARCH_X86, CS_MODE_32)
_FUNCS = None
def functions():
    global _FUNCS
    if _FUNCS is None:
        _FUNCS = {int(r['address'], 16): int(r['bodyBytes']) for r in csv.DictReader((GAME / 'reconstruction/research/native-aa/functions.tsv').open(), delimiter='\t')}
    return _FUNCS
def insns(va, count=None, size=None):
    if size is None: size = 0x2000
    code = PE0.range(va, size)
    out = []
    for i in CS.disasm(code, va):
        out.append(i)
        if count and len(out) >= count: break
    return out
def function(va):
    size = functions()[va]
    return insns(va, size=size)
def u32(va): return struct.unpack('<I', PE0.range(va, 4))[0]
def f32(va): return struct.unpack('<f', PE0.range(va, 4))[0]
def f64(va): return struct.unpack('<d', PE0.range(va, 8))[0]
def wstr(va, limit=400):
    b = PE0.range(va, limit * 2); out = []
    for i in range(0, len(b), 2):
        c = b[i] | b[i + 1] << 8
        if c == 0: break
        out.append(chr(c))
    return ''.join(out)
if __name__ == '__main__':
    if sys.argv[1] == 'fn':
        for i in function(int(sys.argv[2], 16)): print(f'{i.address:08x}: {i.mnemonic} {i.op_str}')
    else:
        n = int(sys.argv[2]) if len(sys.argv) > 2 else 40
        for i in insns(int(sys.argv[1], 16), n): print(f'{i.address:08x}: {i.mnemonic} {i.op_str}')
