# Actual view-startup role and precision

Keep the explicit **Double53** view-startup policy for the captured v14 runtime.
No production precision was changed by this investigation.

Both already sealed Windows traces reached430290 from43F0A0. Independently
decoding each raw716-byte WOW64 context gives Legacy CW=FXSAVE CW=0x027F at
both430290 entry and43F124 return. That is round-nearest PC53 with exceptions
masked. These events ran on the same TID as CREATE_PROCESS's initial thread:
31776 in capture-02,52704 in capture-03. Session constructor return on each
same main thread also has0x027F. Session phase transitions were recorded on
different TIDs17964/18880; the Grid observations are not simulation-worker
events mislabeled as the main thread.

The main frame410E70 consumes notifications through40FBE0/785060 before normal
UI update/render. The notification dispatcher invokes current AppState
virtual1C. The base414620 case1 calls40CF00→5C3000 synchronously; the Loading
override682FD0 calls that same base handler. The actual selected AppState
vtable was not recorded in the captures, so that derived route is a pinned
static alternative, not an extra observed stack frame.5C3000 calls700E30 and
43F0A0 on its current thread. Its5B17F0/5AED10 resource completion task is
created/submitted after43F0A0 returns, rather than transferring43F0A0 itself
to a resource worker. Nine equal original/v14 code windows pin these edges.

The prior phase8 static document predicted main/UI PC24 from bare Direct3D9's
non-FPU-preserve device creation. That external-runtime inference does not
establish the state of this configured target: the observed v14 AA.exe,
D3D9.dll wrapper and dgVoodoo.conf combination instead has main-thread PC53
at these actual startup points. This report does not identify the mechanism
or claim that the wrapper permanently preserves precision. No native-system
Direct3D comparison or new debuggee run was performed.

The existing complete code census has no game-code FP control-state writer
below0x7D0000. Direct-call closures of5C3000,43F0A0,700E30,7010D0,6FF130 and
723E00 contain no controlfp/control87 call or891890 CreateDevice. Their listed
control-state sites are CRT math/exception helpers, which may temporarily
change/restore control state. This is a direct-call lower bound: virtual calls
and external DLL behavior are not proven absent by that graph.

Consequently, using53 for the captured startup's camera reset, scene and
terrain numerics is supported by the actual prefix context and the absence
of a game-level mode setter in the examined paths. Later scene/terrain CW was
not captured by these stopped traces; it is a supported continuation inference,
not whole43F0A0-return hardware evidence. Keep precision an explicit role input
and retain all-PC kernel proofs. Do not assign24 simply because a function is
named UI, or prove this role from the separately sourced Bank precision.

The script reads only existing evidence and both PE images. It verifies the
two capture/script digests, three copied runtime-file pins per capture, raw
Legacy/FXSAVE agreement, debugger context-write masks and cleanup. CONTROL,
DEBUG and post-cutoff INTEGER|CONTROL writes do not include FP/SSE state.
It launches no executable, debugger or native process and does not touch active
profiles. The observations stop at the sealed Grid return; no later main-frame
or full-battle FP claim is made.

```powershell
python research/development10/view-startup-execution-role/review_role.py
```
