"""Page-0 guard for Unicorn harnesses (docs/NATIVE-PORT-RULES.md, emulator
caveats): with FS based at 0, a null read of [0..3] would silently return the
SEH chain pointer. Any page-0 access faults unless the instruction carries the
FS segment override (pattern of research/phase8-ai/native_machine.py)."""
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP

PREFIXES = (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3)


def _fs_prefixed(u, pc):
    for byte in bytes(u.mem_read(pc, 4)):
        if byte == 0x64:
            return True
        if byte not in PREFIXES:
            return False
    return False


def install(machine):
    machine.null_accesses = []

    def hook(u, access, address, size, value, _):
        pc = u.reg_read(UC_X86_REG_EIP)
        if address + size > 4 or not _fs_prefixed(u, pc):
            machine.null_accesses.append([hex(pc), address, size])
            u.emu_stop()

    machine.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, hook, begin=0, end=0xfff)
