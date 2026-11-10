"""The C++ exception tables of AA.exe functions: for each function, the
FuncInfo its frame handler stub names (prolog push -1 / push stub; the stub
is mov eax, FuncInfo; jmp __CxxFrameHandler), its unwind states and its try
blocks with their catch handlers. Read-only; no emulation.

Used for the map transfer: a throw of FileSender::Start 0x761540 passes
0x74FE30, 0x7554B0, 0x75DFF0, 0x413DC0 (or 0x411E90), 0x766CE0, 0x74D710,
0x751B60, 0x752E60, 0x755290, 0x75CA70 and 0x765410, and none of them has a try block, so the
throw reaches the filter of the thread that runs 0x765410 (the thread
named SIM, see README.txt).

  PYTHONPATH=<capstone> python native_eh_frames.py <AA.exe> 761540 74fe30 7554b0
      75dff0 411e90 413dc0 766ce0 74d710 751b60 752e60 755290 75ca70 765410
"""
import struct
import sys

import capstone

data = open(sys.argv[1], 'rb').read()
pe = struct.unpack_from('<I', data, 0x3C)[0]
nsec = struct.unpack_from('<H', data, pe + 6)[0]
opt = struct.unpack_from('<H', data, pe + 20)[0]
base = struct.unpack_from('<I', data, pe + 24 + 28)[0]
secs = []
for i in range(nsec):
    o = pe + 24 + opt + 40 * i
    vsize, va, rsize, raw = struct.unpack_from('<IIII', data, o + 8)
    secs.append((base + va, max(vsize, rsize), raw))


def off(v):
    for va, size, raw in secs:
        if va <= v < va + size:
            return raw + (v - va)
    raise ValueError(hex(v))


md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)


def dis(v, n):
    return list(md.disasm(data[off(v):off(v) + n], v))


def funcinfo(fn):
    ins = dis(fn, 64)
    for k, x in enumerate(ins[:8]):
        if k and x.mnemonic == 'push' and x.op_str.startswith('0x') and ins[k - 1].op_str in ('-1', '0xffffffff'):
            stub = int(x.op_str, 16)
            s = dis(stub, 16)
            if s and s[0].mnemonic == 'mov' and s[0].op_str.startswith('eax, 0x'):
                return stub, int(s[0].op_str.split(', ')[1], 16)
            return stub, None
    return None, None


for a in sys.argv[2:]:
    fn = int(a, 16)
    stub, fi = funcinfo(fn)
    if not fi:
        print('%#x: no frame handler' % fn)
        continue
    magic, maxstate, unwind, ntry, trymap = struct.unpack_from('<IiIiI', data, off(fi))
    print('%#x: FuncInfo %#x magic %#x states %d try blocks %d' % (fn, fi, magic, maxstate, ntry))
    for t in range(ntry):
        lo, hi, chigh, ncatch, harr = struct.unpack_from('<iiiiI', data, off(trymap + 20 * t))
        print('  try %d: states %d..%d, %d catches' % (t, lo, hi, ncatch))
        for c in range(ncatch):
            adj, ptype, disp, addr = struct.unpack_from('<IIiI', data, off(harr + 16 * c))
            name = data[off(ptype + 8):off(ptype + 8) + 80].split(b'\0')[0].decode(errors='replace') if ptype else '...'
            print('    catch %s at %#x' % (name, addr))
