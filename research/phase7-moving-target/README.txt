Phase7 moving-target update and enabled-ability maximum

aa_mind::moving_target::update ports533C30, including ordinary Element5317A0
soft-shape projection, target generation checks, native projection/query order,
distance stores, Kill range reduction, target speed getter effects, and replan.
It updates the existing element_move::Payload.plan/path, never another route.
The associated Installed binding identifies actual Move+7C, Kill+F8 and Join+164
states in the single Mind Arena. Installed bindings and word184 must remain
stable for a call; explicit Payload/Arena mutations by required services are
supported. Stale IDs and service failures remain errors at their actual point.

The real concrete Element Mind vtable isA35810. Its+A4 is52D780, comparing
native word184 with2. Earlier exploratory fixtures incorrectly used Regiment
A335A0's constant-false getter; those were corrected before this ledger was
created. Current native fixtures execute52D780 and pin its actual Element
vtable entry. Mode0,1,3 andFFFFFFFF select the implemented path. Mode2 returns
an explicit unsupported error at the branch after preceding projection/query
effects. Its Formation45D4C0, actor+108 and556070 placement tail is not ported.
Four original mode2 prefixes stop exactly at531995, checking the Rust error
boundary without claiming that branch or whole update succeeded.

Required World services are actual actor snapshots, source soft shape, extended
radius,554350 strict=false actor query,4CE6E0 replacement identity mapped into
SpatialIndex,552250 query positioning,553BA0 collision,57EC80 maximum range,
and target speed virtual+18. The speed return is discarded natively, but its
call and side effects are preserved. No clear-world query, fabricated shape,
zero radius or successful missing callback is provided. Caller-owned world
services must bind the existing native spatial/ActorStore implementations.

For kind0/1 target soft shapes5317A0 uses captured target center. Rectangle
projection uses the added aa_combat::surface::placed_projection46E4F0, including
both local delta float stores, existing46E150, and ordered world-coordinate
addition. Only rectangle/square uses it here, so unused default-point input
cannot invent a point-geometry fallback. Earlier surface functions are unchanged.

aa_combat::admission::maximum_range ports complete57EC80 enabled-ability scan.
Each enabled ability reads its current modifier, compares an extended4C8270
result against the stored maximum, then calls4C8270 a second time on a winner
before storingf32. The caller supplies a stable owned ability table. This
adds a helper; earlier admission/effective_range logic is unchanged.

Validation:330 complete original533C30 executions, four original mode2 prefixes,
and108 complete57EC80 executions across explicit PC24/53/64. Target fixtures
cover soft shape kinds, rotated rectangles, query hit/miss, Join/Kill activity,
owner/target/flags/previous-point mutation and stale generation. Maximum fixtures
cover empty/disabled/mixed ability tables and modifiers. Three target tests
include failed-query and mode2-prefix rejection; three admission tests include
the new maximum helper plus prior admission/range regressions. Original surface
tests are rerun after the additive placed-projection helper. Strict scoped
Clippy is required. Native stack checks are not Rust ABI claims.

Native target fixture seams: radius, maximum-range and speed virtual results;
actor query construction and first-hit result; existing harness surrounding
actor services. Actual soft getter, descriptor projection, GTID lookup, state
checks, derived Element mode getter and path reset execute original code. The
query positioning leaf executes its nonsnap prepared-context branch. Native
maximum scan and all its arithmetic helpers execute without behavior hooks.
Visited windows include intercepted service entries, not branch coverage.

Limits: finite supported nearest precision, stable owned tables/bindings;
arbitrary reentrant mutation of those tables is excluded. Derived soft getters,
mode2 formation placement, actual full runtime service wiring, playable combat,
AI and completed matches are not claimed by this component checkpoint.

Commands:
python research/phase7-moving-target/native_target.py
python research/phase7-moving-target/native_maximum.py
cargo test -p aa-mind --test moving_target_native
cargo test -p aa-combat --test native_admission --test native_surface
cargo clippy -p aa-mind -p aa-combat --all-targets --no-deps -- -D warnings
