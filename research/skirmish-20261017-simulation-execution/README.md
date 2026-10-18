# Retained simulation execution and synchronization gate

The actual native application starts its embedded `+0x150` Thread at
`41ABCF..41ABDC` through `85E7B0 -> 85E430`. With the native application at
B39690, Thread+48 is the global B39828 read by synchronization loggers.
The native resource thread is the distinct application+1F8 owner.

The new Rust `simulation_execution::SimulationThread<S>` owns one real OS
thread, parent/child identity handshake, release barrier and a retained job
queue. `S` is initialized on that thread, may contain Rc/non-Send state, and
never crosses back to its parent. Send jobs borrow the SAME S, and only Send
results cross back. This is portable thread plumbing, not an implementation
or completion claim for the original recurring networking/simulation loop,
priority/TLS/thread registry, or all native Thread lifecycle methods.

`SimulationIdentity::sync_gate` reads the current OS ID anew, compares the
actual retained simulation ID, and only on equality reads the actual SAI
presence/+5C/+5D through the required lazy closure. Different main/resource
OS threads therefore return false without reading absent SAI loans. The
logger's enabled/suspend guards remain in the existing Synchronizer caller.
No caller-provided role ID, fixed thread number or hardcoded gate is used.

Evidence:
- native_gate.py/native-gate.json: 60 original/v14/all-PC whole native integer
  logger rows, including present/absent SAI, current-ID equality/mismatch and
  noncanonical flag bytes; 12 native ID/state publication slices. The Windows
  import is a named scripted OS seam. Full Application Init/Thread execution
  is not claimed. 41ABCF and85E430 instructions are pinned against both images.
- Rust integration tests: actual Windows IDs, same non-Send Rc state across
  jobs, main-thread lazy SAI skip, positive flags on the real child, explicit
  initializer failure/join, stopped identity refusal, all60 native gate rows.
  rust-tests01.log and final guard regression rust-tests02.log: 7/7 passed (four are included Windows producer tests).
- Native01 failed because the v14 extended image needed additional mapped
  pages; native02 reached all rows but attempted to pin an explicitly named
  out-of-image string allocator seam. Both failures remain. Native03 passes.

First actual Firefight startup attempt01 stopped at the required logger gate
while SyncA was genuinely enabled; that result is retained under
research/development10/playable-startup. Attempt02 uses the retained actual
thread identity, preserving the native main/resource-thread short circuit.
Successful identity binding alone does not certify map load, battle or AI.
The final stopped-thread lifetime and foreign-application allocation guards
are portable validation only, not additional native gameplay fields.
The source-profile/GameSpy sibling loan now also lends the SAME current SAI
bytes for the positive real-thread equality branch without a cached copy.

Builds use release optimization with explicit lto=false/codegen-units=16
command overrides in this agent's target/s-local-bootstrap. This is not the
final default thin-LTO release executable.
