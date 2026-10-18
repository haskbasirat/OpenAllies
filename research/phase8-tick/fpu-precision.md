# x87 precision of the authoritative simulation (AA.exe, static evidence)

Scope: which x87 control word (CW) is in effect when the original game runs
its frame/tick code, from AA.exe alone (SHA256 5d2548a2...0d26). Tools:
`fpu_scan.py` -> `fpu-sites.json` (every FLDCW/FNINIT/FLDENV/FRSTOR/FXRSTOR/
LDMXCSR/FNCLEX, every `_controlfp`/`_control87` call, every
IDirect3D9::CreateDevice call, with direct-call reachability), pseudocode and
disassembly. No original process was run.

## Verdict

The working policy "PC24 for the authoritative simulation" is **refuted by
static evidence**, with one OS-level assumption left to confirm dynamically:

* Direct3D 9 is created **without** `D3DCREATE_FPU_PRESERVE`, so Direct3D
  sets single precision (PC24, round-to-nearest) - but only on the thread that
  calls `CreateDevice`, which is the **main/UI thread**.
* The whole tick (`TGC_NetNode::Update` 0x765410 -> 0x75CA70 -> world tick
  0x4F9970 / admin tick 0x755290) runs on a **separate TGC_Thread** created
  after the device, whose CRT start routine performs **no** floating-point
  initialisation. Its CW is therefore the Windows initial thread state, which
  is 0x027F (PC53, round-to-nearest, all exceptions masked) on x86 Windows.
* Nothing reachable from the tick changes the CW persistently; the only CW
  changes reachable from it are CRT math routines that restore the caller's CW.

Recommended policy: `Precision::Double53` for the authoritative simulation
(`World::clock_precision`, `Queues::precision`, `Driver::precision`), and
`Precision::Single24` for presentation code ported from the main thread
(render interpolation, UI). Precision stays an explicit input everywhere until
the dynamic check below confirms it. The oracles cover PC24, PC53 and PC64;
the interleaving case `host_third_rate` already differs between PC24 and
PC53/PC64 within 40 frames, so this choice is observable.

## 1. CreateDevice call sites and BehaviorFlags

All three IDirect3D9::CreateDevice calls (IDirect3D9 pointer 0xB443F4,
vtable +0x40) are in `NiDX9Renderer::Create` 0x891890: 0x891B45, 0x891B8A
(fewer back buffers retry) and 0x891F0E. BehaviorFlags = renderer +0x294
(`piVar4[0xA5]`):

* 0x88B1A0 sets the base by device type: 0x50, 0x40, 0x80, 0x20 (vertex
  processing / pure device) - never 0x2.
* 0x89198C..0x891998: caller flag 0x40 adds 0x2 (`D3DCREATE_FPU_PRESERVE`); caller flag
  0x20 adds 0x4 (`D3DCREATE_MULTITHREADED`).
* The only caller, `KKC_Renderer::ChangeResolution` 0x710AC0, builds its
  flags from 0x10 (UVars+0xD4 clear), 0x20 (UVar RendererMultithreaded), 0x4,
  0x8, or the fallback constant 6 - **never 0x40**.

So `D3DCREATE_FPU_PRESERVE` is never requested. (Direct3D 9 documents that
without it the runtime switches the calling thread to single precision.)

## 2. Threads

| Thread | Created by | Runs | FP init |
|---|---|---|---|
| main | process start (WinMain -> `KKC_Application::Init` 0x419FB5) | init, renderer, UI, message loop | CRT `_cinit` -> `__fpmath(1)` 0x96BDB7 -> `__setdefaultprecision` 0x972C6B: `_controlfp(_PC_53, _MCW_PC)`; then `CreateDevice` (0x7101A0 at 0x41A200 -> 0x710AC0 -> 0x891890) switches it to PC24 |
| net / simulation | `KKC_Application::Init` 0x41ABC5..0x41ABEE: TGC_Thread at app+0x150, callback NetNode+0x40, `0x85E7B0` -> `0x85E430` -> `_beginthreadex` 0x9713EF (suspended), priority 0x85DB00(2), resumed by vfunc +0x14 | `TGC_Thread::Run` 0x85ED90 calls the callback [thread+0xA4] on every wait timeout. The callback is the thunk built in `TGC_NetNode` ctor 0x765AB0 (`mov eax,this; xchg [esp],eax; push eax; jmp 0x765410`), i.e. `TGC_NetNode::Update` | `_threadstartex` 0x97135A calls `_FPmtinit` [0xB3509C] = 0x7D723C, a bare `RET`: no CRT FP init. CW = OS initial thread state |
| GameSpy | same pattern, callback [0xB3654C]+0x9C (0x41AD5B) | online services | as above |

The renderer (`CreateDevice`, 0x41A200) is created before the net thread is
started (0x41ABDC). Device re-creation goes through
`KKC_Application::ChangeResolution` 0x40E260 from the options UI
(0x67C390/0x67CD00), i.e. the UI thread, never the net thread.

Inside 0x765410, after 0x75CA70, the strategic AI `KKC_SAI::Think` 0x7A0080
runs on the same net thread with a wall-clock budget (UVars
SAITimeBetweenThinks/SAIMaxAccumulateThinkTime) using fibers
(ConvertThreadToFiber/CreateFiber/SwitchToFiber). It issues decrees; its CW is
the net thread's (fiber FP-state switching is OS behaviour, see section 5).

## 3. Control-word changes (fpu-sites.json)

117 FLDCW, 1 FRSTOR, 1 FLDENV, 2 FNCLEX, 5 calls to `_controlfp`/`_control87`:

* **Game code 0x401000-0x7CFFFF: none.**
* Statically linked D3DX9 (0x7D0000-0x7F8000; HLSL compiler strings
  "row_major", "pack_matrix", "texcoord"...): FLDCW save/restore pairs, and
  0x7F1965 `_controlfp(_PC_53)` for a compiler context restored at 0x7F1A39.
  None is reachable from 0x765410/0x75CA70/0x4F9970/0x755290 (direct calls).
* CRT: `__ftol` 0x96F7C0 (truncating RC, restored), x87 transcendental
  helpers `__trandisp1/2`, `__ctrlfp`/`__clrfp`, `__math_exit` 0x97792B,
  0x977895 (keeps PC, forces RN + masks), `_CIpow` x87 body 0x96D8ED (FRSTOR).
  The ten sites reachable from the tick are all CRT math reached through
  `_CIpow` 0x96D890 and the sibling dispatchers 0x96D4C0/0x96D5E0; each
  restores the caller CW on return. Executed check: the CW was unchanged
  after all 3132 `_CIpow` calls in native-tick-round2.json (both bodies,
  PC24/53/64).
* `__setdefaultprecision` is reached only from `__fpmath` (process start).

## 4. Per-call-path precision table

| Call path | Thread | CW (static) | Confidence |
|---|---|---|---|
| 0x75CA70 frame decision, 0x76AF90/0x76AF30 timeline | net | 0x027F PC53 | static + OS default |
| 0x752AE0 -> 0x76F790 -> 0x4F9970 world tick (queues, payloads, manager, transitions) | net | 0x027F PC53 | static + OS default |
| 0x755290 admin tick, 0x76B780, decree handlers via 0x752E60/0x74FD10 -> 0x74D710 -> 0x766CE0 | net | 0x027F PC53 | static + OS default |
| 0x76F370 SetGameSpeed from decree 0x75D810 / cutscene 0x4F1080 / triggers 0x5420D0 | net (decree/tick paths) | PC53 | static |
| 0x76F370 from UI 0x61E620 / 0x76F530 | not proven (UI handlers) | PC24 if main thread | open |
| 0x775BF0 session launch (SetFrozen, launch world dump with one RNG draw) | net (0x776560 <- 0x755840 admin message handlers) | PC53 | static |
| `KKC_Sim::CreateGWorld` 0x76AA90 -> GWorld::Init 0x4F27E0 (step = 1/GameTicksPerGameSecond) | callers 0x513410 / 0x5126C0 have no direct callers (virtual) | not proven | open |
| render-time interpolation 0x76B120, renderer, UI | main | 0x007F PC24 (after CreateDevice) | static + D3D documented behaviour |
| D3DX shader compilation | main | PC53 inside 0x7F1965..0x7F1A39, restored | static |
| any CRT math inside the above | same | internal changes, caller CW restored | executed (`_CIpow`) |

## 5. Residual assumptions (why a dynamic check is still useful)

1. Windows' initial x87 CW for a new thread is 0x027F (not inherited from the
   creating thread). This is OS behaviour, not visible in AA.exe.
2. Direct3D 9 does not change the CW of threads other than the one calling
   into it, and the net thread makes no Direct3D calls (none found
   statically from 0x765410; virtual calls are not followed).
3. Fiber switches for the SAI keep the net thread's CW (OS behaviour).
4. World creation (0x76AA90) runs on the net thread.

## 6. Dynamic check (not run here; requires the original game)

Either way works and changes nothing in the game files:

* Debugger (x32dbg or WinDbg) attached to the original AA.exe on a test
  machine, offline single-player skirmish. Logging breakpoints (no break) at
  0x4F9970, 0x755290, 0x76F370, 0x76AA90, 0x4F27E0, 0x891B45, 0x891B8A,
  0x891F0E, each logging the thread id and the x87 control word (x32dbg
  `log "{tid()} {x87ControlWord}"`; WinDbg `r fpcw` with `~.`). Also log the
  CW right after `CreateDevice` returns (0x891B48). Expected: CreateDevice on
  the main thread with CW 0x007F after the call; every 0x4F9970/0x755290 hit on
  one other thread with CW 0x027F. Also record the CRT flag [0xB4B340] (which
  `_CIpow` body the machine uses).
* Without a debugger: a proxy DLL that patches the first instruction of
  0x4F9970 with a jump to a stub that executes only `fnstcw` and
  `GetCurrentThreadId`, appends both to a file, then runs the displaced
  instruction and jumps back. The stub must not touch the FPU state otherwise.
* Repeat on an Intel and an AMD CPU and at two game speeds.

## 7. Emulator caveat for all oracles (emulator-limits.json)

Unicorn 2.1.4 does not raise the x87 precision flag from inexact arithmetic,
and computes FPATAN/FSIN/FCOS/FYL2X through binary64 (FPATAN(1,1) returns
significand ...C000 instead of the real ...C235). The tick-driver oracles use
only +,-,*,/, compares and stores and are unaffected. Results of x87
transcendental instructions and of CRT tails that depend on the sticky
precision flag (0x97792B) are only binary64-accurate in Unicorn fixtures:
this applies to the `_CIpow` x87-body rows and to the 80-bit 0x445400
captures (see README.txt section 10).

## 8. Sticky precision flag (PE) on the tick path

Question (movement agent: `aa_movement::ground_step_original` /
`OriginalTurning` take PE as an input): is the x87 status flag PE set when
gameplay code inside the tick computes a turn angle?

Answer (static): **yes - PE is set on every world tick of the native game**,
except for a theoretical first frame whose wall-clock store is exact.

* Every frame starts in `TGC_NetNode::Update` 0x765410 with
  `fild timeGetTime(); fmul 0.001f (0x9F4580); fst dword [NetNode+0x58]`
  at 0x76543D..0x76544F. The binary32 store of n*0.001f is inexact for
  practically every millisecond count, which sets PE (all exceptions are
  masked, so it only sets the sticky bit). This precedes 0x75CA70 and
  therefore every world tick, queue delivery and turning computation of
  that frame; the timeline arithmetic 0x76AF90 and many other ops follow.
* Nothing reachable on the net thread clears PE (fpu-sites.json):
  - FNINIT/FINIT: no site in AA.exe.
  - FNCLEX: 0x96BDD2 in `__fpmath` (process start, main thread) and
    0x97DE2C in `__clrfp`, reached only through `__raise_exc` 0x97D756 when
    `__handle_exc` 0x97D9FB reports an UNMASKED exception. With the CWs in use
    (0x007F/0x027F/0x037F) every exception is masked; for a masked precision
    exception `__handle_exc` calls `__set_statfp(0x20)` 0x97DE58, which
    re-raises PE, and returns "handled".
  - FRSTOR 0x96DA09 in the `_CIpow` x87 body restores the state saved by the
    FNSAVE at 0x96D9FD around `__powhlp` 0x9787D5, i.e. the caller's status
    including an already set PE (only flags raised inside `__powhlp` are
    discarded).
  - FLDENV 0x979E42 (0x979C46) is not reachable from the frame or the tick.
  - No FXRSTOR/LDMXCSR on the tick path; Direct3D is not called from the net
    thread.
* Thread switches preserve each thread's x87 state. SAI fibers run after the
  tick in the same frame; whether or not Windows switches the FP state for
  fibers, the main fiber's PE was already set by 0x76544F.

Consequence: ported gameplay code that runs inside the world tick (or the
admin tick) should be called with PE = set (`precision_exception_pending =
true` in aa_trig, the equivalent input in aa_movement). This matches the AI
agent's choice for its 52A930 oracle. Cleared PE only applies to code run
before the first frame's time store on a fresh thread. Unicorn cannot
confirm this dynamically (it never raises PE from arithmetic,
emulator-limits.json); the debugger procedure in section 6 can log the x87
status word at the same breakpoints (x32dbg `x87StatusWord`, WinDbg
`r fpsw`): bit 5 should be set at every 0x4F9970 hit.
