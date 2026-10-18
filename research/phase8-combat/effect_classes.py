"""Read-only: map ability effect kinds (4CA880 jump table 4CA9D0) to their
constructed class vtables and list each vtable's first slots (+0 dtor, +4,
+8 Apply(ability, context, scale) as called by 4C9550, +C, +10 LoadData).
Usage: effect_classes.py [SLOTS]"""
from pathlib import Path
import struct
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402

KINDS = ['SUMMON', 'MORALE_LOSS', 'MORALE_GAIN', 'HEAL', 'MAIN_DAMAGE', 'ADD_DAMAGE', 'ENCHANTMENT', 'CAPTURE',
         'STEAL_RESOURCES', 'DESCRIBE_ECONOMY']
md = Cs(CS_ARCH_X86, CS_MODE_32)


def u32(va):
    return struct.unpack('<I', h.ORIGINAL.at(va, 4))[0]


def vtable_of_case(case):
    """First `mov dword ptr [eax], imm` in the case or in the ctor it calls."""
    for insn in md.disasm(h.ORIGINAL.at(case, 0x40), case):
        if insn.mnemonic == 'mov' and insn.op_str.startswith('dword ptr [eax], 0x'):
            return int(insn.op_str.split(', ')[1], 16), None
        if insn.mnemonic == 'call' and insn.op_str != '0x6f7810':
            ctor = int(insn.op_str, 16)
            last = None
            for c in md.disasm(h.ORIGINAL.at(ctor, 0x80), ctor):
                if c.mnemonic == 'mov' and c.op_str.startswith('dword ptr [esi], 0x') or \
                   c.mnemonic == 'mov' and c.op_str.startswith('dword ptr [edi], 0x'):
                    last = int(c.op_str.split(', ')[1], 16)
                if c.mnemonic == 'ret':
                    break
            return last, ctor
    return None, None


def main():
    slots = int(sys.argv[1]) if len(sys.argv) > 1 else 6
    for kind in range(10):
        case = u32(0x4CA9D0 + 4 * kind)
        vt, ctor = vtable_of_case(case)
        entries = [u32(vt + 4 * i) for i in range(slots)] if vt else []
        print(f'{kind} {KINDS[kind]:16} case {case:08x} ctor {ctor and f"{ctor:08x}"} vtable {vt and f"{vt:08x}"} '
              + ' '.join(f'+{4*i:x}:{e:08x}' for i, e in enumerate(entries)))


if __name__ == '__main__':
    main()
