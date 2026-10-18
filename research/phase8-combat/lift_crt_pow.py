"""Generate crates/aa-numeric/src/crt_pow.rs from the original AA.exe bytes.

Mechanical, instruction-by-instruction translation of the MSVC CRT SSE2 pow
body 0x977A29 (entered from _CIpow 0x96D890 -> 0x977A10) into Rust. Each
basic block becomes a match arm; every SSE2 instruction maps to the IEEE
binary64/integer operation it performs under the default MXCSR (round to
nearest, no FTZ/DAZ). The coefficient tables are copied verbatim from the
read-only data at 0xAD0BA0..0xAD44C8. The generated file is a port of the
original machine code, verified by native_crt_pow.py fixtures; it is not a
reimplementation of pow.
"""
from pathlib import Path
import hashlib
import struct
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_OP_REG  # noqa: E402

START, SIZE = 0x977A29, 2861
TABLE_START, TABLE_END = 0xAD0BA0, 0xAD44C8
ERROR_SUPPORT = 0x977111

data = (GAME / 'AA.exe').read_bytes()
assert hashlib.sha256(data).hexdigest() == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
pe = struct.unpack_from('<I', data, 0x3c)[0]
count = struct.unpack_from('<H', data, pe + 6)[0]
opt = pe + 24
base = struct.unpack_from('<I', data, opt + 28)[0]
sect = opt + struct.unpack_from('<H', data, pe + 20)[0]
sections = [struct.unpack_from('<IIII', data, sect + i * 40 + 8) for i in range(count)]


def offset(va):
    for _, rva, size, off in sections:
        if rva <= va - base < rva + size:
            return off + va - base - rva
    raise ValueError(hex(va))


code = data[offset(START):offset(START) + SIZE]
md = Cs(CS_ARCH_X86, CS_MODE_32)
md.detail = True
insns = list(md.disasm(code, START))
assert sum(i.size for i in insns) == SIZE

GPR = {'eax': 0, 'ecx': 1, 'edx': 2, 'ebx': 3, 'esp': 4, 'ebp': 5, 'esi': 6, 'edi': 7}


def xmm(i, op):
    name = i.reg_name(op.reg)
    assert name.startswith('xmm'), name
    return int(name[3:])


def gpr(i, op):
    name = i.reg_name(op.reg)
    assert name in GPR, name
    return GPR[name]


def addr(i, op):
    m = op.mem
    parts = []
    if m.base:
        parts.append(f'c.r[{GPR[i.reg_name(m.base)]}]')
    if m.index:
        parts.append(f'c.r[{GPR[i.reg_name(m.index)]}].wrapping_mul({m.scale})')
    parts.append(f'{m.disp & 0xffffffff:#x}u32')
    out = parts[0]
    for p in parts[1:]:
        out = f'{out}.wrapping_add({p})'
    return out


def src32(i, op):
    if op.type == X86_OP_REG:
        return f'c.r[{gpr(i, op)}]'
    if op.type == X86_OP_IMM:
        return f'{op.imm & 0xffffffff:#x}u32'
    return f'c.rd32({addr(i, op)})'


def src64(i, op):
    """Scalar binary64 operand bits (xmm low lane or m64)."""
    if op.type == X86_OP_REG:
        return f'c.x[{xmm(i, op)}][0]'
    return f'c.rd64({addr(i, op)})'


def src128(i, op):
    if op.type == X86_OP_REG:
        return f'c.x[{xmm(i, op)}]'
    return f'c.rd128({addr(i, op)})'


targets = {START}
for i in insns:
    if i.mnemonic.startswith('j'):
        t = i.operands[0]
        assert t.type == X86_OP_IMM
        targets.add(t.imm)
        targets.add(i.address + i.size)
    if i.mnemonic == 'ret':
        targets.add(i.address + i.size)
addresses = {i.address for i in insns}
for t in targets:
    assert t in addresses or t == START + SIZE, hex(t)

COND = {'je': 'c.zf', 'jne': '!c.zf', 'jae': '!c.cf', 'jb': 'c.cf', 'ja': '!c.cf && !c.zf',
        'jbe': 'c.cf || c.zf', 'jle': 'c.zf || c.sf != c.of', 'jl': 'c.sf != c.of',
        'jge': 'c.sf == c.of', 'jg': '!c.zf && c.sf == c.of', 'js': 'c.sf', 'jns': '!c.sf'}


def translate(i):
    m, ops = i.mnemonic, i.operands
    a = ops[0] if ops else None
    b = ops[1] if len(ops) > 1 else None
    if m == 'movlpd':
        if a.type == X86_OP_REG:
            return [f'c.x[{xmm(i, a)}][0] = c.rd64({addr(i, b)});']
        return [f'c.wr64({addr(i, a)}, c.x[{xmm(i, b)}][0]);']
    if m == 'movsd':
        if a.type == X86_OP_REG and b.type == X86_OP_REG:
            return [f'c.x[{xmm(i, a)}][0] = c.x[{xmm(i, b)}][0];']
        if a.type == X86_OP_REG:
            return [f'c.x[{xmm(i, a)}] = [c.rd64({addr(i, b)}), 0];']
        return [f'c.wr64({addr(i, a)}, c.x[{xmm(i, b)}][0]);']
    if m == 'movapd':
        assert a.type == X86_OP_REG
        return [f'c.x[{xmm(i, a)}] = {src128(i, b)};']
    if m == 'movd':
        if a.type == X86_OP_REG and i.reg_name(a.reg).startswith('xmm'):
            return [f'c.x[{xmm(i, a)}] = [u64::from({src32(i, b)}), 0];']
        return [f'c.r[{gpr(i, a)}] = c.x[{xmm(i, b)}][0] as u32;']
    if m in ('andpd', 'orpd', 'xorpd', 'pxor'):
        op = {'andpd': '&', 'orpd': '|', 'xorpd': '^', 'pxor': '^'}[m]
        d = xmm(i, a)
        return [f'{{ let s = {src128(i, b)}; c.x[{d}] = [c.x[{d}][0] {op} s[0], c.x[{d}][1] {op} s[1]]; }}']
    if m in ('mulsd', 'addsd', 'subsd', 'divsd'):
        fn = m[:3]
        d = xmm(i, a)
        return [f'{{ let s = {src64(i, b)}; c.x[{d}][0] = {fn}(c.x[{d}][0], s); }}']
    if m in ('mulpd', 'addpd', 'subpd'):
        fn = m[:3]
        d = xmm(i, a)
        return [f'{{ let s = {src128(i, b)}; c.x[{d}] = [{fn}(c.x[{d}][0], s[0]), '
                f'{fn}(c.x[{d}][1], s[1])]; }}']
    if m in ('psrlq', 'psllq'):
        d = xmm(i, a)
        if b.type == X86_OP_REG:
            # Count is the whole low quadword of the source; >= 64 clears.
            fn = 'psrlq_var' if m == 'psrlq' else 'psllq_var'
            return [f'c.x[{d}] = {fn}(c.x[{d}], c.x[{xmm(i, b)}][0]);']
        assert b.type == X86_OP_IMM
        n = b.imm
        if n >= 64:
            return [f'c.x[{d}] = [0, 0];']
        op = '>>' if m == 'psrlq' else '<<'
        return [f'c.x[{d}] = [c.x[{d}][0] {op} {n}, c.x[{d}][1] {op} {n}];']
    if m == 'psubq':
        d = xmm(i, a)
        return [f'{{ let s = {src128(i, b)}; c.x[{d}] = [c.x[{d}][0].wrapping_sub(s[0]), '
                f'c.x[{d}][1].wrapping_sub(s[1])]; }}']
    if m in ('psubd', 'pcmpeqd'):
        d = xmm(i, a)
        fn = 'psubd' if m == 'psubd' else 'pcmpeqd'
        return [f'{{ let s = {src128(i, b)}; c.x[{d}] = {fn}(c.x[{d}], s); }}']
    if m == 'pmaxsw':
        d = xmm(i, a)
        return [f'{{ let s = {src128(i, b)}; c.x[{d}] = pmaxsw(c.x[{d}], s); }}']
    if m == 'pmovmskb':
        return [f'c.r[{gpr(i, a)}] = pmovmskb(c.x[{xmm(i, b)}]);']
    if m == 'pextrw':
        n = ops[2].imm
        return [f'c.r[{gpr(i, a)}] = ((c.x[{xmm(i, b)}][{n // 4}] >> {16 * (n % 4)}) & 0xffff) as u32;']
    if m == 'pinsrw':
        n = ops[2].imm
        d = xmm(i, a)
        lane, sh = n // 4, 16 * (n % 4)
        return [f'c.x[{d}][{lane}] = (c.x[{d}][{lane}] & !(0xffffu64 << {sh})) | '
                f'((u64::from({src32(i, b)}) & 0xffff) << {sh});']
    if m == 'pshufd':
        return [f'c.x[{xmm(i, a)}] = pshufd({src128(i, b)}, {ops[2].imm});']
    if m == 'unpcklpd':
        d = xmm(i, a)
        return [f'{{ let s = {src128(i, b)}; c.x[{d}] = [c.x[{d}][0], s[0]]; }}']
    if m == 'cvtdq2pd':
        return [f'{{ let s = {src128(i, b)}; c.x[{xmm(i, a)}] = [b(f64::from(s[0] as u32 as i32)), '
                f'b(f64::from((s[0] >> 32) as u32 as i32))]; }}']
    if m == 'cvtsd2si':
        return [f'c.r[{gpr(i, a)}] = cvtsd2si(f({src64(i, b)}));']
    if m == 'mov':
        if a.type == X86_OP_MEM:
            return [f'c.wr32({addr(i, a)}, {src32(i, b)});']
        return [f'c.r[{gpr(i, a)}] = {src32(i, b)};']
    if m in ('add', 'sub', 'cmp', 'and', 'or', 'xor'):
        d = gpr(i, a)
        s = src32(i, b)
        fn = {'add': 'add', 'sub': 'sub', 'cmp': 'sub', 'and': 'and', 'or': 'or', 'xor': 'xor'}[m]
        if m == 'cmp':
            return [f'{{ let s = {s}; c.{fn}(c.r[{d}], s); }}']
        return [f'{{ let s = {s}; c.r[{d}] = c.{fn}(c.r[{d}], s); }}']
    if m in ('shl', 'sar'):
        d = gpr(i, a)
        n = src32(i, b) if b.type == X86_OP_IMM else f'c.r[1]'
        return [f'{{ let n = {n}; c.r[{d}] = c.{m}(c.r[{d}], n); }}']
    if m == 'neg':
        d = gpr(i, a)
        return [f'c.r[{d}] = c.neg(c.r[{d}]);']
    if m == 'push':
        return [f'{{ let v = {src32(i, a)}; c.r[4] = c.r[4].wrapping_sub(4); c.wr32(c.r[4], v); }}']
    if m == 'pop':
        return [f'c.r[{gpr(i, a)}] = c.rd32(c.r[4]); c.r[4] = c.r[4].wrapping_add(4);']
    if m == 'fld':
        if a.type == X86_OP_MEM:
            return [f'c.st0 = Some(quiet(c.rd64({addr(i, a)})));']
        raise ValueError(i)
    if m == 'fldz':
        return ['c.st0 = Some(0);']
    if m == 'fld1':
        return ['c.st0 = Some(0x3ff0_0000_0000_0000);']
    if m == 'call':
        assert a.imm == ERROR_SUPPORT
        return ['c.libm_error();']
    raise ValueError(f'unsupported {i.mnemonic} {i.op_str}')


blocks = []
current = None
for i in insns:
    if i.address in targets:
        current = [i.address, []]
        blocks.append(current)
    m = i.mnemonic
    current[1].append(f'// {i.address:08x} {i.mnemonic} {i.op_str}')
    if m == 'jmp':
        current[1].append(f'pc = {i.operands[0].imm:#x}; continue;')
        current = None
    elif m.startswith('j'):
        cond = COND[m]
        current[1].append(f'if {cond} {{ pc = {i.operands[0].imm:#x}; continue; }}')
        current[1].append(f'pc = {i.address + i.size:#x}; continue;')
        current = None
    elif m == 'ret':
        current[1].append('return c.finish();')
        current = None
    else:
        current[1].extend(translate(i))
for blk in blocks:
    last = blk[1][-1]
    if not (last.startswith('pc =') or last.startswith('return') or last.startswith('if ')):
        nxt = None
        # fall through to the next block address
        idx = blocks.index(blk)
        nxt = blocks[idx + 1][0]
        blk[1].append(f'pc = {nxt:#x}; continue;')

table_words = struct.unpack_from('<%dI' % ((TABLE_END - TABLE_START) // 4), data, offset(TABLE_START))
table_src = ',\n'.join(', '.join(f'{w:#010x}' for w in table_words[k:k + 8]) for k in range(0, len(table_words), 8))

template = (HERE / 'crt_pow_template.rs').read_text()
body = '\n'.join(
    f'            {addr:#x} => {{\n' + '\n'.join('                ' + s for s in stmts) + '\n            }'
    for addr, stmts in blocks)
out = template.replace('/*TABLE*/', table_src).replace('/*BODY*/', body) \
    .replace('/*TABLE_LEN*/', str(len(table_words))) \
    .replace('/*SOURCE_SHA*/', hashlib.sha256(code).hexdigest()) \
    .replace('/*TABLE_SHA*/', hashlib.sha256(data[offset(TABLE_START):offset(TABLE_START) + TABLE_END - TABLE_START]).hexdigest())
target = ROOT / 'crates/aa-numeric/src/crt_pow.rs'
target.write_text(out)
subprocess.run(['rustfmt', '--edition', '2024', str(target)], check=True)
print(json_summary := f'{len(insns)} instructions, {len(blocks)} blocks, {len(table_words)} table words -> {target}')
