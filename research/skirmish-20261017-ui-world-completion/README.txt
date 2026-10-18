Native UI / loading completion, 2026-10-17

Source of truth: original AA.exe 5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26;
v14 AA.exe 3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1.
No downloaded SDK implementation, original binary, save, profile or release was modified.

Actual retained ownership now implemented:
- Application+E4 Loading state is constructed once by the existing application's
  owner (40CB40 -> 682DE0). Application failure retains this same allocation.
- 682E10 binding, 682DB0 menu callback, 682FD0 derived notification prefix,
  and 682EE0 lazy flag/AD/F4 checks preserve native order and partial stores.
- 76ACA0 -> 74FF50 writes the sole retained local Node90, then uses the existing
  real 74E970/304 request emitter. It does not create another Node/TaskHistory,
  unfreeze Sim29 or bypass the later 304/315/775BF0 protocol.
- ActualActivatedSession::tick_current_loading borrows SAME Application.loading,
  SessionF4, current live World constructor identity and VWorld native AD field.
  ViewState.initialized_for verifies the actual weak identity on every read.
- UI5C3000 ordered control owns its reached 1E4/1E0 fields. The first NULL
  Kingdom lookup leaves1E0 untouched; the positive path rereads76C2B0. The
  complete original UI constructor is NOT inferred: these fields start unbound.
- The5C3104/5B17F0 derived task is move-only. An enqueue child must adopt its
  actual slot into the retained queue; failure retains it in whichever owner
  reached publication. UI holds only a weak observer, never a cloned task.
- Actual5B3550 clears SAME VWorldAC before SAME current Loading17 then Task14=2.
  It neither writes AD nor promotes other loading flags. Only this resource-task
  callback writes17. Calling this body alone is NOT a queue barrier proof.

Validation completed:
- native_loading.py: 286 explicit laboratory input cases, original/v14 at each
  PC24/53/64 =1716 native executions,309 unique executed instruction pins.
  Includes Loading constructor/bind/menu/event/tick, AD reread after status
  callback, exact callback cutoff prefixes, Session76ACA0/Admin74FF50 and UI5C3000.
- Existing full5B3550 -> actual40D970 -> actual5B18C0(2) fixture was replayed
  read-only:108 executions,22 executed pins, SHA53900a6fb5f967b7be841ccd6e2d6bc755eebf3b9c6f1056002a616576787979.
- all-rust-tests02.log:4 small standalone exact-production-source tests PASS.
  Loading1320 rows; UI180 rows; completion216 rows; actual task/World identity,
  native flag ordering, AD remains false and weak task observation expiry.
  Test-only generated files contain unchanged production private constructor/
  view prefixes, and the unchanged completion algorithm with a String error
  alias and relocated include path. They do not publish a test-only constructor.
- client-check01.log: cargo check -p aa-client --release --locked --offline -j2
  PASS3m28s; actual concrete adapters and current controller post-CV source
  suffix compiled. This was a production check, no final EXE or runtime test link.
- Early standalone link failures (wrong ThinLTO serde artifact), an include-path
  relocation error and one test expected-count error remain in earlier logs.
  Functional row comparisons passed before the count assertion was corrected.

Explicit boundaries still required before playable/ready claims:
- UI5C3000 camera700E30/700500, complete43F0A0 and4F0750 real view publication,
  actual retained ResourceManager5AED10 queue/barrier dispatch, and wrapper40CF00
  current ApplicationState virtualE8 child. RequiredServices have no no-op defaults.
- Loading kind1 multiplayer status773CE0's actual Session148/14C owner and kind9
  Application40CE10 state table/OnEnter services remain required. Current public
  binding only projects actual kind0/Loading6 notification682E10; it does not
  pretend the surrounding784220 dispatcher or event consumption completed.
- Root owns actual43F0A0/AD1; SDK owns real derived view Init/publication4F0750.
  The fresh UI coordinator intentionally refuses retry into a retained prefix;
  actual teardown/reinitialization is a separate future owner operation.
- Native allocator-null/SEH unwind and arbitrary callback reentry into the UI
  allocation are outside this finite successful-allocation dispatch domain.
  Required child cutoffs pause before the native child; they are not native C++
  exceptions, so no SEH cleanup equivalence is claimed.
- GPU acknowledgments are only backend data. No GPU-only ready bit, invented
  AD value, automatic Sim unfreeze or playable battle result is exposed.

Reproduction (small tests only, no full runtime link):
  python research/skirmish-20261017-ui-world-completion/native_loading.py
  python research/development10/native_view_resource_completion.py
  python research/skirmish-20261017-ui-world-completion/build_standalone.py
  rustc --edition 2024 --test research/skirmish-20261017-ui-world-completion/all_tests.rs
    --extern serde_json=target/s-graphics-review/release/deps/libserde_json-3f0b3bb60c709793.rlib
    -L dependency=target/s-graphics-review/release/deps -C opt-level=1
    -o target/s-graphics-review/ui-world-completion-tests.exe
  target/s-graphics-review/ui-world-completion-tests.exe --nocapture

Parent's next coherent library cohort can use filters:
  aa-runtime --lib application_loading::tests
  aa-runtime --lib ui_world_init::tests
  aa-netcode --lib world_view::tests
No additional full runtime link was started by this author.
