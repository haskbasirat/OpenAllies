"""AA.exe 0x858030 (TGC_File::ComputeChecksum, the map checksum of
SetSessionLoadMap 0x75EEB0 and SendUpdateMapStatus 0x770B70) under Unicorn,
over a stand-in TGC_File whose virtuals serve a byte vector (+0x34 seek,
+0x30 size, +0 read). Compares it with the rule of
crates/aa-netcode/src/file.rs map_checksum (crc = crc * 7 + byte, 32-bit) and
prints the values crates/aa-netcode/src/lan/transfer.rs holds.

  PYTHONPATH=<unicorn and keystone> python native_map_checksum.py <AA.exe>
"""
import sys, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EAX
from keystone import Ks, KS_ARCH_X86, KS_MODE_32

EXE = sys.argv[1]
data = open(EXE, "rb").read()
pe = struct.unpack_from("<I", data, 0x3C)[0]
nsec = struct.unpack_from("<H", data, pe + 6)[0]
optsz = struct.unpack_from("<H", data, pe + 20)[0]
base = struct.unpack_from("<I", data, pe + 24 + 28)[0]
off = pe + 24 + optsz


def run(content):
    mu = Uc(UC_ARCH_X86, UC_MODE_32)
    for i in range(nsec):
        name, vsz, va, rsz, raw = struct.unpack_from("<8sIIII", data, off + 40 * i)
        size = (max(vsz, rsz) + 0xFFF) & ~0xFFF
        mu.mem_map(base + va, size)
        mu.mem_write(base + va, data[raw: raw + min(rsz, size)])
    STUB, OBJ, DATA, STK = 0x10000000, 0x11000000, 0x12000000, 0x13000000
    for a in (STUB, OBJ, DATA, STK):
        mu.mem_map(a, 0x200000)
    ks = Ks(KS_ARCH_X86, KS_MODE_32)
    pos = OBJ + 0x100
    mu.mem_write(DATA, bytes(content))
    seek = ks.asm("mov dword ptr [%d], 0; ret 8" % pos)[0]
    size = ks.asm("mov eax, %d; ret" % len(content))[0]
    # read(buf, n): n bytes from DATA + position into buf, returns n
    read = ks.asm(
        "push esi; push edi; mov edi, [esp+12]; mov ecx, [esp+16]; mov esi, [%d]; add esi, %d; "
        "mov eax, ecx; add [%d], ecx; rep movsb; pop edi; pop esi; ret 8" % (pos, DATA, pos))[0]
    stubs = {}
    cur = STUB
    for k, code in (("seek", seek), ("size", size), ("read", read)):
        mu.mem_write(cur, bytes(code))
        stubs[k] = cur
        cur += 0x100
    vt = [0] * 16
    vt[0] = stubs["read"]
    vt[0x30 // 4] = stubs["size"]
    vt[0x34 // 4] = stubs["seek"]
    mu.mem_write(OBJ + 0x1000, struct.pack("<16I", *vt))
    mu.mem_write(OBJ, struct.pack("<I", OBJ + 0x1000))
    end = STUB + 0x1000
    mu.mem_write(end, b"\xF4")
    esp = STK + 0x100000
    esp -= 4
    mu.mem_write(esp, struct.pack("<I", 0))  # ComputeChecksum(0): the whole size
    esp -= 4
    mu.mem_write(esp, struct.pack("<I", end))
    mu.reg_write(UC_X86_REG_ESP, esp)
    mu.reg_write(UC_X86_REG_ECX, OBJ)
    mu.emu_start(0x858030, end, count=10_000_000)
    return mu.reg_read(UC_X86_REG_EAX)


def rule(content):
    c = 0
    for b in content:
        c = (c * 7 + b) & 0xFFFFFFFF
    return c


CASES = [b"", b"\x01", b"\x01\x02", b"\xff\xff\xff", bytes(range(256)), bytes([0xFF] * 64),
         b"Maps\\(2v2)Transfer.RMP" * 37]
bad = 0
for c in CASES:
    n = run(c)
    print(len(c), "0x%08x" % n, "same" if n == rule(c) else "DIFFERENT 0x%08x" % rule(c))
    bad += n != rule(c)
sys.exit(1 if bad else 0)
