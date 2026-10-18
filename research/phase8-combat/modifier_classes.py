"""Read-only: map modifier types (4FE840 jump table 4FEA6C) to the classes
they construct (vtable, constructor) and list each vtable's first slots
(+0 LoadData(node) as called by 4C4030, ... +20 attribute getter)."""
from pathlib import Path
import struct
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402

TYPES = ['FLAG', 'ADD', 'MULTIPLY', 'ENCHANTMENT_IMMUNITY', 'ABILITY_EFFECT', 'RESIST_DAMAGE', 'PERIODIC_DAMAGE',
         'PERIODIC_HEAL', 'REVERSE_ABILITY', 'RESOURCE_PRODUCTION']
md = Cs(CS_ARCH_X86, CS_MODE_32)


def u32(va):
    return struct.unpack('<I', h.ORIGINAL.at(va, 4))[0]


def scan(start, depth=0):
    """Return (vtable, size, ctor) for the case: allocation size, vtable store."""
    size = vt = ctor = None
    for insn in md.disasm(h.ORIGINAL.at(start, 0x60), start):
        if insn.mnemonic == 'push' and insn.op_str.startswith('0x') and size is None:
            size = int(insn.op_str, 16)
        if insn.mnemonic == 'mov' and ', 0xa' in insn.op_str and insn.op_str.startswith('dword ptr [e'):
            vt = int(insn.op_str.split(', ')[1], 16)
        if insn.mnemonic == 'call' and insn.op_str not in ('0x6f7810',) and depth == 0:
            ctor = int(insn.op_str, 16)
            sub_vt, _, _ = scan(ctor, 1)
            vt = vt or sub_vt
            if sub_vt:
                vt = sub_vt
        if insn.mnemonic in ('jmp', 'ret') and (vt or size):
            break
    return vt, size, ctor


def main():
    slots = int(sys.argv[1]) if len(sys.argv) > 1 else 10
    for kind in range(10):
        case = u32(0x4FEA6C + 4 * kind)
        vt, size, ctor = scan(case)
        entries = [u32(vt + 4 * i) for i in range(slots)] if vt else []
        print(f'{kind} {TYPES[kind]:20} case {case:08x} size {size and hex(size)} ctor {ctor and f"{ctor:08x}"} '
              f'vtable {vt and f"{vt:08x}"} ' + ' '.join(f'+{4*i:x}:{e:08x}' for i, e in enumerate(entries)))


if __name__ == '__main__':
    main()
