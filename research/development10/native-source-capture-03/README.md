# Actual Windows v14 source-load numeric receipt

**Successful bounded capture:** the real v14 Windows executable reached source
startup4BD800, mean computation4BBF90, combat-value computation4BB590 and the
4BD800 return. The return contains **all 1,028 standard ActorData entries** in
native linked-list order, preserving every IDS, flags+174 and cache words+1E4,
+30C and+36C. This is source-data startup, **not a battle or rewritten client**.

`capture.json` SHA-256:
`dcd61117e17730cd02ce62d27452b98867e3b0675f8e7a2a008f0d63f35f7df6`.

| Actual boundary | Thread | x87 CW | x87 SW | MXCSR | CRT SSE2 |
| --- | --- | --- | --- | --- | --- |
| Source4BD800 entry | 15112 | 027F | 0120 | 1FBD | 1 |
| Means4BBF90 entry | 15112 | 027F | 0020 | 1FBD | 1 |
| Values4BB590 entry | 15112 | 027F | 0020 | 1FBD | 1 |
| Source4BD800 return | 15112 | 027F | 4020 | 1FBD | 1 |

The legacy FloatSave and ExtendedRegisters/FXSAVE CW/SW agree at **every**
boundary. Raw716-byte WOW64 contexts, legacy floating save areas and512-byte
FXSAVE areas are retained. CW027F selects53-bit x87 precision and round-to-nearest;
the x87 precision-exception bit is set in all four observed status words. CRT
SSE2 global B4B340 is1. No inference from emulator defaults or a UI thread was
used to assign this source-load context. This observation is for the exact
wrapper/configuration below; other application/thread/driver contexts are not
implicitly covered.

Manager means+74 through+90 are recorded at all boundaries. The mean words at
values entry and return match each other. Lazy assertion/CSV/ranged guards,
cached settings pointer/value pairs and source stream object bytes are retained.
The source manager receiver at entry was checked against the actual B39F58
global before acceptance. No cached input was substituted for a live pointer.

## Exact process and inputs

The sole created debuggee was PID48136 at
`research/development10/native-source-capture-03/runtime/AA.exe`. It was never
an attachment to an existing game. Breakpoints were process-memory-only.

| File | SHA-256 |
| --- | --- |
| AA.exe | 3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1 |
| D3D9.dll | 6a0ca214784be04b7c8b547105aa9d79acf4dc26c0b6f8702b437ddca54058b2 |
| dgVoodoo.conf | 241951ccb0361e76c5a7c7afe4f481122a7b3ff5d428e606913aae89631291f6 |

Ten original archive hashes were measured before and after execution and remain
unchanged. `runtime-files.json` independently checks all **54 copied source
files** after capture: every source/copy byte hash matches, no copy is the same
filesystem file as its source, and every copied RWD has its read-only attribute.
This full54-file post-capture comparison is distinct from the archive hashes
actually pinned both before and after native execution.

The native85A9D0 SHGetFolderPath result was intercepted at85AA1B only after a
successful HRESULT. Its actual CSIDL argument at ESP+218 and returned UTF16
buffer at ESP+8 were used. The replacement path was read back and verified.
Personal5 redirection was observed **before source entry**, plus35/28/38/43;
all destinations are fresh personal/common/other folders under this capture.
The script fails closed at source entry if no verified Personal5 redirect exists.
No original profile was imported. Attempt01 did not establish this condition;
its older scope claim is explicitly disqualified in its FAILED-ATTEMPT.txt.

## Debugger correctness and cleanup

Windows delivered32-bit traps as WX86 breakpoint4000001F and single-step4000001E;
the script handles those plus ordinary80000003/80000004. The independently
reviewed ABI is WOW64_CONTEXT716 bytes and x64 DEBUG_EVENT176 bytes.

All context **gets** use ALL1003F for evidence. All context **sets** use only
CONTROL10001 to move EIP and the single-step flag. Floating/SSE state is never
written by this debugger. Other owned threads are manually suspended only
during each temporarily unpatched single-step, then their exact increments are
resumed; the termination receipt shows all tracked counts at zero.

The original failure is written before cleanup if anything raises. Cleanup
targets only the handle whose actual image path equals the freshly created
executable, requests termination before continuing the stopped return event,
drains pending debug events including EXIT_PROCESS, then waits on an owned
duplicate handle. This run observed EXIT_PROCESS, wait result0, exit code0 and
**no cleanup errors**. The author command completed with exit0 and was reaped.
A subsequent process inventory found no AA.exe process.

The original attempt02 exception is unrecoverable because its old finally block
masked it; it is documented as unknown. Its exact old script is preserved.
Attempt03 retained the legacy/FXSAVE equality assertions, and both views matched.
No assumption about attempt02's unknown cause is needed for this successful run.

Microsoft's primary API documentation supports the debugger lifecycle used:

- [Debugging Events](https://learn.microsoft.com/en-us/windows/win32/debug/debugging-events):
  debug events suspend the process; continuing EXIT_PROCESS is required before
  kernel shutdown completes, and a duplicated process handle supports waiting.
- [ContinueDebugEvent](https://learn.microsoft.com/en-us/windows/win32/api/debugapi/nf-debugapi-continuedebugevent):
  continue each pending event on the debugger's creating thread; distinguish
  handled debugger traps from application exceptions.
- [TerminateProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess):
  remote termination is asynchronous, so successful return alone is not reaping.
- [Wow64GetThreadContext](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-wow64getthreadcontext)
  and [WOW64_CONTEXT](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-wow64_context):
  flags select the returned context and the thread must be stopped.

## Retained validation missteps

An initial post-capture manifest helper used a parent path one level too high.
Its zero-file result is preserved as `runtime-files.invalid-path.json` and is
**invalid evidence**. The corrected helper now asserts that the source directory
exists and that its inventory is nonempty; its54-file result is the accepted
`runtime-files.json`. A preceding one-line shell quoting error wrote no files.
Neither affected the already completed native capture or original assets.

Numeric comparison against the Rust/native-emulated source completion belongs
to the separately produced comparison report. This receipt establishes actual
observed inputs/state/results; it does not itself declare cross-implementation
numeric equality, a runnable battle, multiplayer, or full-game parity.
