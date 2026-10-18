"""Condensed call listing of an original loader (read-only helper).

usage: loader_listing.py <start-hex>
Prints every call with the wide-string immediates pushed before it, the ecx
source and the instruction that consumes the result, plus branches, so the
straight-line structure of the SAIData loaders can be read quickly.
"""
from pathlib import Path
import sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import original, GAME
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from capstone.x86 import X86_OP_IMM


def wide(va):
    try:
        raw = original.range(va, 256)
    except ValueError:
        return None
    out = []
    for i in range(0, len(raw) - 1, 2):
        v = raw[i] | raw[i + 1] << 8
        if v == 0:
            break
        if v < 0x20 or v > 0x7e:
            return None
        out.append(chr(v))
    return ''.join(out) if len(out) > 1 else None


def size_of(start):
    for line in open(GAME / 'reconstruction/research/native-aa/functions.tsv', encoding='utf-8'):
        f = line.split('\t')
        if f[0] == '%08x' % start:
            return int(f[3])
    raise SystemExit('unknown function')


def main():
    start = int(sys.argv[1], 16)
    size = size_of(start)
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = True
    insns = list(md.disasm(original.range(start, size), start))
    pending = []
    for k, ins in enumerate(insns):
        if ins.mnemonic == 'push' and ins.operands[0].type == X86_OP_IMM:
            s = wide(ins.operands[0].imm & 0xffffffff)
            pending.append(repr(s) if s else hex(ins.operands[0].imm & 0xffffffff))
        if ins.mnemonic == 'call':
            nxt = insns[k + 1] if k + 1 < len(insns) else None
            print('%08x call %-12s args=%s  then: %s %s' % (ins.address, ins.op_str, pending,
                                                          nxt.mnemonic if nxt else '', nxt.op_str if nxt else ''))
            pending = []
        elif ins.mnemonic.startswith('j') and ins.mnemonic != 'jmp':
            print('%08x %s %s' % (ins.address, ins.mnemonic, ins.op_str))
        elif ins.mnemonic == 'jmp':
            print('%08x jmp %s' % (ins.address, ins.op_str))


if __name__ == '__main__':
    main()
