The application retains the real OS identity captured by native MainThread
constructor85FA70. Each Created event reads the current OS identity again and
compares it with that retained owner. No precision enum or named role stands
in for an operating-system thread.

The resource worker source is now identified. CRT initializer9C8560 constructs
Application atB39690;4177E0 names the embedded+150 thread SIM and+1F8 thread
ASYNC. ThereforeB39888 is the actual ASYNC owner. ResourceManager5B0400 builds
its+EB callback thunk to5ADDD0, registers its+118 signal with that same thread
in5ABD50, and Application::Init419BA4..C8 starts it through85E7B0/85E430.
The native create call uses `__beginthreadex(NULL,0,child+54,NULL,4,&id)`;
handle, OS ID and state2 are stored before parent registration and OnCreated,
then the original zero start flag resumes it. The ordinary ASYNC task is not
the main-thread loading callback.

Actual LoadGame vtableA43794 has+10=6A71D0 (TRUE) and+14=6A71F0 (FALSE).
Its Block selector enters5B1600, protects with Simulation+24 and temporarily
releases the recursive lock while waiting. Native5ADDD0 pops the same queued
task, stamps its actual current Thread owner via5B18C0, takes Simulation+24,
then invokes the actual LoadGame Run5A4280 through virtual+1C. Native worker
entry thunk→85F700→85DCE0→85ED90 reaches the same ResourceManager+EB thunk.

Proof is bounded: native constructor fragments, OS APIs, base constructors,
locks, allocator disposal, signal waits and waiter registration are explicitly
admitted services. The native dispatch fixture stops at5A4280 entry; it does
not claim to execute map loading or start a real Windows thread in Unicorn.
All executed instruction bytes match both original and v14 images. Static
vtable/name references are recorded separately from executed pins. Every
fixture generator is immutable by default; `--write` is explicit regeneration.

Production ResourceWorker uses a real std::thread named ASYNC. The child may
only capture its OS identity before a parent registration/release barrier;
game code runs after the callback succeeds. Child startup failures, parent
registration failures/panics, and task failures/panics are explicit errors.
The same JoinHandle is reaped by join or Drop. This is one load job, not a port
of all thread signals, manager recurring queues or native Win32 priority.
LoadExecution retains the same immutable application owner and calls the OS
provider on every current-ID request; timestamps remain actual external owners.

FP scope:85F700/85DCE0/85ED90 and5A4280 do not set a precision profile.
CRT worker entry97135A invokes [B3509C], whose image value7D723C is RET.
This agrees with research/phase8-tick/fpu-precision.md. New native fixtures
run at all three explicit PC values. No new dynamic worker-CW measurement or
name-based precision policy is claimed by this module.

Author checks: native MainThread capture24paired/48executions; native ASYNC
source/start/dispatch24paired/48executions; external Rust capture1/1 and
actual Windows worker3/3. Windows tests call Kernel32 GetCurrentThreadId on
both threads and verify distinct real identities, registration before task,
current-ID rereads and failure/cleanup behavior. Map worker integration and
Created timestamp/queue integration are separate root-owned acceptance steps.
