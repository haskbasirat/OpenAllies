Element movement state: original537B90,52FC50 and52E0F0

Production: crates/aa-mind/src/element_move.rs
Payload owns the native MoveState/result-plan fields, collision words+3C,
vector+4C, detached timestamp+74 and word+78. Arena remains authoritative for
state status, parent Mind and its actor. The runtime must retain one payload,
not a second movement snapshot that diverges from ResultPlan/ActionGo.

think implements the original decision order: state diagnostic/status;
detached formation/kingdom checks and timeout; Mover/falling/readiness gates;
moving-target identity/property admission; component74 target updates; path
region/neighbor validation; collision phase reset and replan boolean; arrival
and target completion; and actual ResultPlan or continuation dispatch.
enter clears the appropriate path metadata, clamps the original destination,
selects default speed from the actual Mover, clamps requested speed and writes
the source speed factor before recording world time. leave retains the captured
actor across component cleanup, clears the path and performs the exact stop
operation when active. stop preserves repeated Mover lookups and Character
fallback. Completed changes remain applied if a required service fails.

Small operations included:52CD90 complete path reset;52A1F0/52A240 squared
arrival checks with X stored/Y extended;5525D0 extent−0.0001 clamps;
52D660 stop orchestration;52FD00 generation/flags/self target predicate;
52CAE0 current-or-neighbor region logic;4D43D0 readiness outer gates;52FE60
near-point and soft-shape-range gates. The source0.25 and0.01 thresholds are
squared distance, distinct from collision epsilon. Mover byte+4D is read as
locked, distinct from byte+4C. Native+74 time is stored as raw binary32.

Required services remain explicit, with no successful default implementation:
real source-bound MoverDetachedSeconds registration/value; diagnostic sink;
Organization578E90; tile ownership54EF40;5693B0 and4D10A0; actual Mind virtual
events; moving-target prediction533C30; radius/property/target-point virtuals;
soft-shape descriptors4EA0F0 and46F100; component74 state57DD90/57DEA0;
authoritative regions/adjacency; existing5336B0 ResultPlan;5379C0 continuation;
5767C0 stop,4DB0F0 activation,4CFC80 Character and real scheduler.
Native source and target soft descriptors are captured in source-first order;
the range call is target-vs-source using virtual+10, not target/range+14.
Missing services and invalid component identities produce errors. None are
replaced by straight-line motion or a clear-collision result. The continuation
and larger remaining geometry services have separate owners/proof scopes.

World callbacks may mutate payload/Arena and component state; subsequent reads
follow the original boundaries. The active state/parent identity and retained
component pointers must remain valid until return. Pure field-write methods
must not dispatch unrelated callbacks. Finite arithmetic is supported with an
explicit PC24/53/64; raw moving-state predicates retain IEEE !=0 behavior.
This is state-machine implementation and composition evidence, not a full
restored match, multiplayer or completed gameplay claim.

Proof
native_element_move.py executes unchanged original537B90/52FC50/52E0F0 plus
actual small helpers.384 cases cover status diagnostics, source settings,
formation detach timing, missing/falling/locked Movers, stopping, stale target,
property admission, component74 variants, region paths, replan/arrival and
callback mutation of owner/destination/settings.288 separate original numeric
cases cover both near thresholds and world-edge clamps. All PC modes run.
Six input cases produce different state/trace outcomes across PC modes because
extended radius getter results are deliberately retained until native stores.

Fixtures explicitly intercept the required component/path/virtual services
above. The detached SVar is already registered in the prepared native fixture;
first-time source registration is not replayed. Native allocator free is an
explicit boundary during actual path clear.4EA0F0/46F100 are intercepted as the
already separate soft-shape range service; no claim of its lifecycle proof is
made here. All recorded executed windows exclude intercepted entries and are
original/v14 byte-identical. Constants and the actual state vtable are pinned.

Reproduce:
 python research/phase7-element-move/native_element_move.py
 cargo test -p aa-mind --test element_move_native -- --nocapture
 cargo clippy -p aa-mind --all-targets --no-deps -- -D warnings
Three tests pass, including required-service failures after completed state
writes. Final logs are rust-tests.log,clippy.log andvalidation-record.json.
findings.json is producer evidence; independent peer review is separate.
