"""Sampling profiler for a running Windows x64 process, using only the
system dbghelp.dll (no installed tools, no admin rights).

Each sample briefly suspends every thread, walks its stack with StackWalk64,
and resumes it. Symbols come from the program's PDB next to its .exe.

    python sample_stacks.py <pid> [--seconds 30] [--interval 0.05] [--out report.txt]

The report lists, per busy thread, the hottest leaf functions and the
hottest inclusive functions (each function counted once per sampled stack).
"""
import argparse, collections, ctypes, ctypes.wintypes as wt, sys, time

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
dbg = ctypes.WinDLL("dbghelp", use_last_error=True)

PROCESS_ALL = 0x0400 | 0x0010  # QUERY_INFORMATION | VM_READ
THREAD_ACCESS = 0x0002 | 0x0008 | 0x0040  # SUSPEND_RESUME | GET_CONTEXT | QUERY_INFORMATION
TH32CS_SNAPTHREAD = 0x4
CONTEXT_FULL = 0x10000B
MACHINE_AMD64 = 0x8664

class THREADENTRY32(ctypes.Structure):
    _fields_ = [("dwSize", wt.DWORD), ("cntUsage", wt.DWORD), ("th32ThreadID", wt.DWORD),
                ("th32OwnerProcessID", wt.DWORD), ("tpBasePri", wt.LONG),
                ("tpDeltaPri", wt.LONG), ("dwFlags", wt.DWORD)]

class ADDRESS64(ctypes.Structure):
    _fields_ = [("Offset", ctypes.c_uint64), ("Segment", ctypes.c_uint16), ("Mode", ctypes.c_uint32)]

class KDHELP64(ctypes.Structure):
    _fields_ = [("Thread", ctypes.c_uint64), ("ThCallbackStack", wt.DWORD),
                ("ThCallbackBStore", wt.DWORD), ("NextCallback", wt.DWORD),
                ("FramePointer", wt.DWORD), ("KiCallUserMode", ctypes.c_uint64),
                ("KeUserCallbackDispatcher", ctypes.c_uint64), ("SystemRangeStart", ctypes.c_uint64),
                ("KiUserExceptionDispatcher", ctypes.c_uint64), ("StackBase", ctypes.c_uint64),
                ("StackLimit", ctypes.c_uint64), ("BuildVersion", wt.DWORD),
                ("RetpolineStubFunctionTableSize", wt.DWORD),
                ("RetpolineStubFunctionTable", ctypes.c_uint64), ("RetpolineStubOffset", wt.DWORD),
                ("RetpolineStubSize", wt.DWORD), ("Reserved0", ctypes.c_uint64 * 2)]

class STACKFRAME64(ctypes.Structure):
    _fields_ = [("AddrPC", ADDRESS64), ("AddrReturn", ADDRESS64), ("AddrFrame", ADDRESS64),
                ("AddrStack", ADDRESS64), ("AddrBStore", ADDRESS64),
                ("FuncTableEntry", ctypes.c_void_p), ("Params", ctypes.c_uint64 * 4),
                ("Far", wt.BOOL), ("Virtual", wt.BOOL), ("Reserved", ctypes.c_uint64 * 3),
                ("KdHelp", KDHELP64)]

class SYMBOL_INFOW(ctypes.Structure):
    _fields_ = [("SizeOfStruct", wt.ULONG), ("TypeIndex", wt.ULONG), ("Reserved", ctypes.c_uint64 * 2),
                ("Index", wt.ULONG), ("Size", wt.ULONG), ("ModBase", ctypes.c_uint64),
                ("Flags", wt.ULONG), ("Value", ctypes.c_uint64), ("Address", ctypes.c_uint64),
                ("Register", wt.ULONG), ("Scope", wt.ULONG), ("Tag", wt.ULONG),
                ("NameLen", wt.ULONG), ("MaxNameLen", wt.ULONG), ("Name", ctypes.c_wchar * 1024)]

FTA = ctypes.WINFUNCTYPE(ctypes.c_void_p, wt.HANDLE, ctypes.c_uint64)
GMB = ctypes.WINFUNCTYPE(ctypes.c_uint64, wt.HANDLE, ctypes.c_uint64)
dbg.SymFunctionTableAccess64.restype = ctypes.c_void_p
dbg.SymFunctionTableAccess64.argtypes = [wt.HANDLE, ctypes.c_uint64]
dbg.SymGetModuleBase64.restype = ctypes.c_uint64
dbg.SymGetModuleBase64.argtypes = [wt.HANDLE, ctypes.c_uint64]
dbg.StackWalk64.argtypes = [wt.DWORD, wt.HANDLE, wt.HANDLE, ctypes.POINTER(STACKFRAME64),
                            ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
                            ctypes.c_void_p]
dbg.SymFromAddrW.argtypes = [wt.HANDLE, ctypes.c_uint64, ctypes.POINTER(ctypes.c_uint64),
                             ctypes.POINTER(SYMBOL_INFOW)]
dbg.SymInitializeW.argtypes = [wt.HANDLE, wt.LPCWSTR, wt.BOOL]
k32.OpenProcess.restype = wt.HANDLE
k32.OpenThread.restype = wt.HANDLE
k32.CreateToolhelp32Snapshot.restype = wt.HANDLE
k32.GetThreadContext.argtypes = [wt.HANDLE, ctypes.c_void_p]
k32.GetThreadTimes.argtypes = [wt.HANDLE] + [ctypes.POINTER(wt.FILETIME)] * 4

def threads(pid):
    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
    entry = THREADENTRY32(); entry.dwSize = ctypes.sizeof(entry)
    out = []
    ok = k32.Thread32First(snap, ctypes.byref(entry))
    while ok:
        if entry.th32OwnerProcessID == pid:
            out.append(entry.th32ThreadID)
        ok = k32.Thread32Next(snap, ctypes.byref(entry))
    k32.CloseHandle(snap)
    return out

def cpu_time(handle):
    t = [wt.FILETIME() for _ in range(4)]
    if not k32.GetThreadTimes(handle, *[ctypes.byref(x) for x in t]):
        return 0
    f = lambda ft: (ft.dwHighDateTime << 32) | ft.dwLowDateTime
    return f(t[2]) + f(t[3])

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("pid", type=int)
    ap.add_argument("--seconds", type=float, default=30)
    ap.add_argument("--interval", type=float, default=0.05)
    ap.add_argument("--depth", type=int, default=64)
    ap.add_argument("--out")
    ap.add_argument("--sympath", help="folder holding the program's .pdb (default: system search)")
    ap.add_argument("--stacks", action="store_true",
                    help="print every thread's most common full stack, including idle threads")
    a = ap.parse_args()
    proc = k32.OpenProcess(PROCESS_ALL, False, a.pid)
    if not proc:
        sys.exit(f"OpenProcess failed: {ctypes.get_last_error()}")
    dbg.SymSetOptions(0x2 | 0x4)  # UNDNAME | DEFERRED_LOADS
    if not dbg.SymInitializeW(proc, a.sympath, True):
        sys.exit(f"SymInitialize failed: {ctypes.get_last_error()}")
    fta, gmb = FTA(dbg.SymFunctionTableAccess64), GMB(dbg.SymGetModuleBase64)
    ctx_buf = ctypes.create_string_buffer(1232 + 16)
    base = (ctypes.addressof(ctx_buf) + 15) & ~15
    names = {}
    def name(addr):
        if addr in names:
            return names[addr]
        info = SYMBOL_INFOW(); info.SizeOfStruct = 88  # sizeof(SYMBOL_INFOW) with Name[1]
        info.MaxNameLen = 1024
        disp = ctypes.c_uint64()
        n = info.Name if dbg.SymFromAddrW(proc, addr, ctypes.byref(disp), ctypes.byref(info)) else f"0x{addr:x}"
        names[addr] = n
        return n
    handles, start_cpu = {}, {}
    leaf = collections.defaultdict(collections.Counter)
    incl = collections.defaultdict(collections.Counter)
    samples = collections.Counter()
    full = collections.defaultdict(collections.Counter)
    end = time.time() + a.seconds
    while time.time() < end:
        for tid in threads(a.pid):
            h = handles.get(tid)
            if h is None:
                h = k32.OpenThread(THREAD_ACCESS, False, tid)
                if not h:
                    continue
                handles[tid] = h; start_cpu[tid] = cpu_time(h)
            if k32.SuspendThread(h) == 0xFFFFFFFF:
                continue
            try:
                ctypes.memset(base, 0, 1232)
                ctypes.c_uint32.from_address(base + 0x30).value = CONTEXT_FULL
                if not k32.GetThreadContext(h, ctypes.c_void_p(base)):
                    continue
                frame = STACKFRAME64()
                frame.AddrPC.Offset = ctypes.c_uint64.from_address(base + 0xF8).value
                frame.AddrStack.Offset = ctypes.c_uint64.from_address(base + 0x98).value
                frame.AddrFrame.Offset = ctypes.c_uint64.from_address(base + 0xA0).value
                for x in (frame.AddrPC, frame.AddrStack, frame.AddrFrame):
                    x.Mode = 3  # AddrModeFlat
                stack = []
                for _ in range(a.depth):
                    if not dbg.StackWalk64(MACHINE_AMD64, proc, h, ctypes.byref(frame), ctypes.c_void_p(base),
                                           None, ctypes.cast(fta, ctypes.c_void_p),
                                           ctypes.cast(gmb, ctypes.c_void_p), None):
                        break
                    if frame.AddrPC.Offset == 0:
                        break
                    stack.append(frame.AddrPC.Offset)
            finally:
                k32.ResumeThread(h)
            if not stack:
                continue
            samples[tid] += 1
            fns = [name(x) for x in stack]
            if a.stacks:
                full[tid][tuple(fns)] += 1
            leaf[tid][fns[0]] += 1
            for f in set(fns):
                incl[tid][f] += 1
        time.sleep(a.interval)
    lines = []
    busy = sorted(handles, key=lambda t: cpu_time(handles[t]) - start_cpu[t], reverse=True)
    for tid in busy[:4]:
        cpu = (cpu_time(handles[tid]) - start_cpu[tid]) / 1e7
        if cpu < 0.5:
            continue
        lines.append(f"=== thread {tid}: {cpu:.1f}s CPU during {a.seconds:.0f}s, {samples[tid]} samples")
        lines.append("-- leaf")
        for f, n in leaf[tid].most_common(25):
            lines.append(f"{100*n/samples[tid]:5.1f}%  {f}")
        lines.append("-- inclusive")
        for f, n in incl[tid].most_common(45):
            lines.append(f"{100*n/samples[tid]:5.1f}%  {f}")
    if a.stacks:
        for tid in sorted(full):
            stack, n = full[tid].most_common(1)[0]
            lines.append(f"=== thread {tid} most common stack ({n}/{samples[tid]} samples)")
            lines.extend("    " + f for f in stack)
    text = "\n".join(lines)
    if a.out:
        open(a.out, "w", encoding="utf-8").write(text)
    print(text)

if __name__ == "__main__":
    main()
