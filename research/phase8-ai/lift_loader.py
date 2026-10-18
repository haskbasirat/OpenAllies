"""Lift the straight-line getter/store sequences of an original SAIData
loader into Rust statements (read-only helper; the generated text is pasted
into crates/aa-mind/src/sai/ego*.rs and the irregular blocks it marks are
ported by hand).

usage: lift_loader.py <start-hex> [this-register]

Recognised patterns (each emits one statement with its native address):
  push <wide name>; mov ecx, R; call 5B5130; fstp dword [B + off]   -> f32 field
  ... call 5B5130; fmul [9F4978]; sub esp,8; fadd [9FBDB8]; fstp qword;
      call 96D5E0; add esp,8; call 96CC4C; ... mov [B + off], eax     -> ms field
  push <wide name>; mov ecx, R; call 5B4ED0; ... mov [B + off], eax  -> i32 field
  push <wide name>; mov ecx, R; call 5B5380; ... mov byte [B + off], al -> bool
  push 1; push <wide name>; mov ecx, R; call 5B4410; mov R2, eax     -> section
A store through the this-register B is emitted as an offset; any other base
register is emitted as `<reg> + off`. Anything else is an IRREGULAR marker.
"""
from pathlib import Path
import sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import original
from loader_listing import wide, size_of
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_OP_REG

GETTERS = {0x5b5130: 'fixed', 0x5b4ed0: 'int', 0x5b5380: 'bool', 0x5b4410: 'section'}


def is_store(t):
    if t.mnemonic not in ('fstp', 'mov') or not t.operands or t.operands[0].type != X86_OP_MEM:
        return False
    base = t.operands[0].mem.base
    if base == 0 or t.reg_name(base) in ('esp', 'ecx'):
        return False
    if t.mnemonic == 'fstp':
        return 'dword' in t.op_str
    return len(t.operands) > 1 and t.operands[1].type == X86_OP_REG and t.reg_name(t.operands[1].reg) in ('eax', 'al')


def main():
    start = int(sys.argv[1], 16)
    this = sys.argv[2] if len(sys.argv) > 2 else 'ebp'
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = True
    insns = list(md.disasm(original.range(start, size_of(start)), start))
    regs = {}
    out = []
    i = 0
    pending = []
    last_call = None
    while i < len(insns):
        ins = insns[i]
        m, ops = ins.mnemonic, ins.operands
        if m == 'push' and ops[0].type == X86_OP_IMM:
            pending.append(ops[0].imm & 0xffffffff)
            i += 1
            continue
        if m == 'mov' and ops[0].type == X86_OP_REG and ins.reg_name(ops[0].reg) == 'ecx' \
                and ops[1].type == X86_OP_REG:
            last_call = ins.reg_name(ops[1].reg)
            i += 1
            continue
        if m == 'call' and ops[0].type == X86_OP_IMM and ops[0].imm in GETTERS:
            kind = GETTERS[ops[0].imm]
            node = regs.get(last_call, 'node_%s' % last_call)
            names = [wide(v) for v in pending]
            name = next((n for n in reversed(names) if n), None)
            j = i + 1
            if kind == 'section':
                nxt = insns[j]
                dst = ins.reg_name(nxt.operands[0].reg) if nxt.mnemonic == 'mov' and \
                    nxt.operands[1].type == X86_OP_REG and nxt.reg_name(nxt.operands[1].reg) == 'eax' else 'eax'
                var = 's_' + (name or 'x').lower()
                out.append('    // %08x' % ins.address)
                out.append('    let %s = required(d, %s, "%s")?;' % (var, node, name))
                regs[dst] = var
                i = j + (1 if dst != 'eax' else 0)
                pending = []
                last_call = None
                continue
            k = j
            ms = False
            store = None
            while k < len(insns) and k < j + 12:
                t = insns[k]
                if t.mnemonic == 'fmul' and 'dword ptr [0x9f4978]' in t.op_str:
                    ms = True
                if is_store(t) and not (t.mnemonic == 'fstp' and ms):
                    store = t
                    break
                if t.mnemonic == 'call' and t.operands[0].type == X86_OP_IMM and t.operands[0].imm in GETTERS:
                    break
                k += 1
            out.append('    // %08x' % ins.address)
            if store is None:
                out.append('    // IRREGULAR: %s %s result consumed by: %s' % (kind, name,
                           '; '.join('%s %s' % (x.mnemonic, x.op_str) for x in insns[j:j + 4])))
                i = j
                pending = []
                last_call = None
                continue
            off = store.operands[0].mem.disp
            base = store.reg_name(store.operands[0].mem.base)
            at = '%#x' % off if base == this else '%s + %#x' % (base, off)
            if kind == 'fixed' and ms:
                out.append('    e.set_i32(%s, milliseconds(&d.fixed(%s, "%s")?, p)?);' % (at, node, name))
            elif kind == 'fixed':
                out.append('    e.set_u32(%s, f32_of(&d.fixed(%s, "%s")?)?);' % (at, node, name))
            elif kind == 'int':
                out.append('    e.set_i32(%s, d.int(%s, "%s")?);' % (at, node, name))
            else:
                out.append('    e.set_u8(%s, u8::from(d.boolean(%s, "%s")?));' % (at, node, name))
            pending = [x.operands[0].imm & 0xffffffff for x in insns[j:k]
                       if x.mnemonic == 'push' and x.operands[0].type == X86_OP_IMM]
            last_call = None
            for x in insns[j:k]:
                if x.mnemonic == 'mov' and x.operands[0].type == X86_OP_REG and \
                        x.reg_name(x.operands[0].reg) == 'ecx' and x.operands[1].type == X86_OP_REG:
                    last_call = x.reg_name(x.operands[1].reg)
                elif not (x.mnemonic == 'push' and x.operands[0].type == X86_OP_IMM) and \
                        not (x.mnemonic == 'mov' and x.reg_name(x.operands[0].reg) == 'ecx') and \
                        x.mnemonic not in ('fmul', 'fadd', 'sub', 'add', 'call') and \
                        not (x.mnemonic == 'fstp' and 'qword' in x.op_str):
                    out.append('    // IRREGULAR (between) %08x  %s %s' % (x.address, x.mnemonic, x.op_str))
            i = k + 1
            continue
        out.append('    // IRREGULAR %08x  %s %s' % (ins.address, m, ins.op_str))
        if m == 'mov' and len(ops) == 2 and ops[0].type == X86_OP_REG and ops[1].type == X86_OP_REG and \
                ins.reg_name(ops[1].reg) in regs:
            regs[ins.reg_name(ops[0].reg)] = regs[ins.reg_name(ops[1].reg)]
        elif m in ('mov', 'lea', 'xor', 'or', 'inc', 'dec', 'add', 'pop') and ops and ops[0].type == X86_OP_REG:
            regs.pop(ins.reg_name(ops[0].reg), None)
        i += 1
    print('\n'.join(out))


if __name__ == '__main__':
    main()
