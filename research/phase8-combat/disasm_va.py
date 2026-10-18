"""Read-only linear disassembly of AA.exe for functions missing from the Ghidra
inventory (virtual-only entries). Stops after the last ret not followed by a
jump target inside the window. Usage: dis.py ADDR [MAXBYTES] [--strings]."""
from pathlib import Path
import struct
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402

md = Cs(CS_ARCH_X86, CS_MODE_32)


def wide(va):
    try:
        raw = h.ORIGINAL.at(va, 160)
    except ValueError:
        return None
    text = raw.decode('utf-16le', 'replace').split('\x00')[0]
    if len(text) >= 2 and all(32 <= ord(ch) < 127 for ch in text):
        return text
    return None


def main():
    va = int(sys.argv[1], 16)
    size = int(sys.argv[2], 16) if len(sys.argv) > 2 and not sys.argv[2].startswith('-') else 0x400
    insns = list(md.disasm(h.ORIGINAL.at(va, size), va))
    targets = set()
    end = None
    for i in insns:
        if i.mnemonic.startswith('j') and i.op_str.startswith('0x'):
            targets.add(int(i.op_str, 16))
    for i in insns:
        if i.mnemonic.startswith('ret') and not any(t > i.address for t in targets if t < va + size):
            end = i.address
            break
    for i in insns:
        note = ''
        for tok in i.op_str.replace(',', ' ').replace(']', ' ').replace('[', ' ').split():
            if tok.startswith('0x') and len(tok) >= 7:
                s = wide(int(tok, 16))
                if s:
                    note = f'   ; L"{s}"'
        print(f'{i.address:08x}  {i.mnemonic} {i.op_str}{note}')
        if end is not None and i.address >= end:
            break


if __name__ == '__main__':
    main()
