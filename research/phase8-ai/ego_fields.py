"""Ego field map recovered from the original SAIData ego loader 4B0560.

Static, read-only. Walks the original instructions of 4B0560, tracking the
TGI section handles returned by 5B4410 (enter child section) and every scalar
reader call: 5B5130 (fixed -> fstp dword), 5B4ED0 (int -> mov dword) and
5B5380 (bool -> mov byte). The wide key string pushed before each reader and
the following store into the ego object (ebp-relative) give offset -> key.
Sub-list loaders called from 4B0560 (4AF500 GoalPriority, 4AF230 ResourceNeed,
...) are listed as calls; their record layouts are separate evidence.
Writes ego-fields.json.
"""
from pathlib import Path
import json, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import original, header, sha
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_OP_REG

READERS = {0x5b5130: 'fixed', 0x5b4ed0: 'int', 0x5b5380: 'bool'}
SECTION = 0x5b4410
START, SIZE = 0x4b0560, 8889


def wide(va):
    try:
        raw = original.range(va, 512)
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
    return ''.join(out) if out else None


def main():
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = True
    code = original.range(START, SIZE)
    pushes = []
    ecx_src = None
    sections = {'esi': '', 'edi': '', 'ebx': ''}  # handle register -> path
    pending = None  # (key, type, path, call va)
    last_section = None
    fields, calls = [], []
    for ins in md.disasm(code, START):
        m, ops = ins.mnemonic, ins.operands
        if m == 'push' and ops and ops[0].type == X86_OP_IMM:
            pushes.append(ops[0].imm & 0xffffffff)
        elif m == 'mov' and len(ops) == 2 and ops[0].type == X86_OP_REG and ins.reg_name(ops[0].reg) == 'ecx' \
                and ops[1].type == X86_OP_REG:
            ecx_src = ins.reg_name(ops[1].reg)
        elif m == 'mov' and len(ops) == 2 and ops[0].type == X86_OP_REG and ops[1].type == X86_OP_REG \
                and ins.reg_name(ops[1].reg) == 'eax' and last_section is not None:
            sections[ins.reg_name(ops[0].reg)] = last_section
            last_section = None
        elif m == 'call' and ops and ops[0].type == X86_OP_IMM:
            target = ops[0].imm & 0xffffffff
            if target == SECTION:
                name = wide(pushes[-1]) if len(pushes) >= 2 else None
                parent = sections.get(ecx_src, '')
                last_section = (parent + '/' if parent else '') + (name or '?')
            elif target in READERS:
                key = wide(pushes[-1]) if pushes else None
                pending = {'key': key, 'type': READERS[target], 'section': sections.get(ecx_src, ''), 'call': hex(ins.address)}
            else:
                calls.append(hex(target))
            pushes = []
        elif pending and len(ops) >= 1 and ops[0].type == X86_OP_MEM and ins.reg_name(ops[0].mem.base) == 'ebp' \
                and m in ('fstp', 'mov'):
            fields.append({**pending, 'offset': hex(ops[0].mem.disp), 'store': f'{m} {ins.op_str}'})
            pending = None
    d = header(__file__, __doc__)
    d.update(function=hex(START), fields=fields, other_calls=sorted(set(calls)))
    p = HERE / 'ego-fields.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'fields': len(fields), 'sha256': sha(p.read_bytes())}))
    for f in fields:
        print(f['offset'], f['type'], f['section'], f['key'])


if __name__ == '__main__':
    main()
