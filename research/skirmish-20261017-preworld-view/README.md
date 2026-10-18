# Retained pre-World VWorld allocation

The application now constructs and retains one concrete
`view_init_world::State<ViewHandle, view_assets::State>` before a simulation
World exists. Its opaque allocation identity is published in the same
`ApplicationViews` owner. EndSession notification case2 can therefore call
`current_initialized_for_application()` and read the actual constructor-owned
AD byte without deriving a value from a missing battle, World, view or renderer.

The native authority is the bounded442C00 prefix.442C4C publishes the current
allocation atB36538;442CBB clears that allocation'sAD byte. The new native
fixture observes publication before the AD store with three allocation fills,
both original/v14 executables and all three x87 precisions:18 separate runs,
57 independently equal instruction pins. Base/embedded constructors are named
nonalias seams, and execution stops at442D0E. This is not a complete442C00,
application startup capture, allocator/exception or VWorld readiness claim.

The AD field has one private `Arc<AtomicU8>` owner. The application retains a
read-only `Weak` capability to that same cell. Moving the VWorld preserves it;
dropping the native owner causes reads to fail even while application identity
metadata remains. The existing private43FF36 writer remains the sole production
store of1; fresh construction stores0. Atomic/Weak allocation and opaque
application handle1 are modern ownership mechanisms, not original pointer ABI
or native concurrency proofs. No public writer or arbitrary ready constructor
was added. AC's existing owner/callback remains separate.

`ApplicationViews::construct(world)` creates the real Directory binding before
consuming the retained native owner. It then moves that exact State and opaque
Arc identity into `VWorldOwner`; the sole production caller in
`battle_view_startup` no longer creates a new asset/VWorld state. Failed binding
keeps the constructor state. A second adoption refuses before invoking its
binding callback; repeated missions still require real43E400 teardown/rebinding,
which this checkpoint does not implement. Directory, factory, derived Init,
FX attachment and final completion receipt behavior are preserved.

Validation is separated by scope:

* `exact-queue-prefix-tests.exe`:18/18 pass. This successor compiles current
  production prefix/completion, queued resource task, AC callback, Loading and
  UI kernels directly, retaining the existing bounded Resource identity/World
  revision fragments. It includes the new actual AD reader move/retirement
  regression. Old immutable queue artifacts are untouched.
* `exact-allocation-tests.exe`:7/7 pass. Three tests execute the unchanged
  production generic adoption boundary over a real production prefix State:
  failed-binding retention/one adoption, equal-number foreign identity refusal,
  and dead-application refusal. Four current prefix/completion tests also run.
  Product N is the complete aggregate State; the test N is the real prefix
  owner. The explicit scalar test binding is not a simulated Directory or
  evidence that aggregate/assets/camera/physical policy construction completed.
* New native publication/AD proof passes a fresh immutable replay. Existing
  native prefix/constructor fixture also replays378 separate runs, and the
  unchanged final AD single-store fixture replays54 rows/108 native runs.
* The complete client ApplicationViews ownership test is saved in its normal
  source module for the next full client test cohort. No new full client/unit
  link was started for this checkpoint. Final production check results and
  source hashes are recorded separately in the checkpoint JSON.

The first exact queue build selected an old thin-LTO rules archive and failed
to link. Build02 uses the available matching ordinary archive and passes. Both
logs are retained. Initial production checks encountered independent in-flight
menu export, Ni cache cast syntax and notification timer-result errors; their
owners repaired those sources. No claim of success is taken from those checks.

No original game files, release packages, workspace manifests or Cargo lockfile
were modified. This work makes the early notification read concrete; it does
not certify successful EndSession, full view initialization or a playable battle.
