"""Null-page guard for the Unicorn harnesses (docs/NATIVE-PORT-RULES.md,
emulator caveats): the FS base is 0, so the MSVC SEH link fs:[0] lives at
linear 0..3 and page 0 must stay mapped. Without a guard, a native null
pointer read of [0..3] silently returns the SEH chain pointer and any other
page-0 access returns zeros. Only FS-prefixed accesses inside [0, 4) are
legal; every other page-0 access stops the run and is reported as a native
null-pointer dereference (the original process would fault there).
"""
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE, UC_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP

PREFIXES = (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3)

class NullDereference(RuntimeError):
    pass

def fs_prefixed(u, pc):
    """True when the instruction at pc carries the FS segment override (0x64)."""
    for byte in bytes(u.mem_read(pc, 4)):
        if byte == 0x64:
            return True
        if byte not in PREFIXES:
            return False
    return False

def install(machine, stop=True):
    """Hook page 0 of `machine.u`. With `stop`, the run stops at the first
    illegal access and `machine.call` raises NullDereference; without it the
    accesses are only recorded in `machine.null_accesses`."""
    machine.null_accesses = []
    def hook(u, access, address, size, value, user):
        pc = u.reg_read(UC_X86_REG_EIP)
        if address + size <= 4 and fs_prefixed(u, pc):
            return
        machine.null_accesses.append({'pc': hex(pc), 'address': address, 'size': size, 'write': access == UC_MEM_WRITE})
        if stop:
            u.emu_stop()
    machine.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, hook, begin=0, end=0xfff)
    if stop:
        original = machine.call
        def call(*args, **kwargs):
            machine.null_accesses.clear()
            try:
                return original(*args, **kwargs)
            finally:
                if machine.null_accesses:
                    raise NullDereference(f'native null-pointer access {machine.null_accesses[:4]}')
        machine.call = call
