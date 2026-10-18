Native combat mean producer, 2026-10-16

Implemented crates/aa-combat/src/combat_mean_coordinator.rs and its isolated
integration test. lib.rs only appends the module export. No existing formula,
Catalog, definition parser, manifest, SDK, original game file, user profile,
save, executable or release was changed.

What this adds
The earlier combat_value::mean_values port evaluates supplied snapshots. The
new coordinator executes the bounded 4BBF90 owner procedure against a borrowed
World, retaining the source list and object identities, required lazy reads,
progressive shared manager writes and assertion/log callback effects. Its API
is combat_mean_coordinator::{World, Field, compute(world, explicit Precision)}.
No means, completed ActorData values, settings defaults or source-load PC are
manufactured. Existing exact aa_numeric finite x87 arithmetic is reused; there
is no host-float reduction, parallel reduction, sorting substitution or SDK code.

Recovered behavior
- Count the manager's native-kind-2 list, then traverse it again. The +68 list
  uses successful registration order in reverse. RandomActor kind19 is a
  separate registration list and must not be added to this mean pass.
- Zero only +74,+78,+7C,+80,+84,+88. Prior +8C/+90 survive all earlier failures.
- Read bit14, call 45E6D0 and compare with the CURRENT +74. On a new maximum,
  call 45E6D0 independently again and store that second result. Reread flags
  before the Body/Mover/Performer and Experience/Element eligibility gates.
- Capture signed ability count once, reread the ability array/slot each
  iteration, capture *local before the base-power gate and retain that same
  definition as power getter ECX. Capture local+4 before 4BE680. Recharge is
  called independently twice when the first result is at least the stored
  animation/reload floor; the second result becomes the stored rate.
- Maintain five independent descending sorted columns. Equal values insert
  after existing equals. There is no ability-count-derived gameplay cap.
- Load discard SVar 87 (+15C) ONCE at 4BC2E8 and retain its extended value
  across both native _ftol2 count calculations. The decompiler omits this
  retained-stack dependency; its apparent second getter must not be invented.
- Sum each selected value through a binary32 store, interleaving attack/rate
  and then defense/movement/health stores. Keep each reciprocal extended.
- Read assertion guard B3B4F8 before the last three scaling stores. Compare
  the stored local defense with a fresh current attack read. Run 855D30 only
  when required, store returned AL to the guard, then reread attack, defense
  and health after that callback. Divide by extended attack-minus-defense
  for lifetime before storing the offense word.
- Emit seven 404F60 diagnostics in native order, rereading each shared field
  after the previous callback. A log callback can therefore change later log
  values without retroactively changing the already stored lifetime/offense.

Proof and scope
native_means.py freshly maps the two pinned PE images using phase8-ai's strict
FS-only page-zero guard. It executes 4BBF90, native sorted insertion/comparator
and native _ftol2. Native-mode rows execute all reached getter bodies too;
dynamic-mode rows replace declared getter entry responses to test required
lazy calls and shared mutations. Native allocation/free/assert/log are declared
leaves. The 702 instruction pins are a union: a getter entry is a genuine pin
only because another native-mode run executed it, never because a seamed entry
was counted as executed. The producer checks both image bytes for every pin.

Final native-means.json has 210 cases: 2 images x 3 PCs x 35 cases. Of these,
102 use original getter bodies, 108 use explicit dynamic getter responses;
150 finish and 60 stop at ten declared failure/unsupported boundaries. The
Rust test compares the ENTIRE semantic trace, final eight manager words,
guard state and success/failure for every row. The trace includes list reads,
source flags/count/ability/reload reads, calls, mean reads/stores and logs.

Mutation probes cover independent extent/recharge returns; flags reread after
extent; captured reload before animation callback; a captured two-ability
count despite callback setting its source count to zero, and a changed second
array slot; list-next reread after a getter; assertion mutation before terminal
means; and per-log current-field rereads. The power-definition capture probe
uses an explicit external storage intervention at the base-power FLD memory
read. That instruction has NO application callback. This is an injected
identity-stability probe, not evidence that AA performs application reentrancy
at a plain memory read.

The ten stopped boundaries are extent, power, animation, second recharge,
defense, movement, health, assertion, fourth log, and empty attack trim. A
required-service failure is represented by a native seam stop and Rust error,
not by a claim that original AA safely returns an error. Empty trim stops
immediately before native reciprocal division. The original would enter an
unsupported Inf/NaN path; the Rust port refuses and retains its prior stores.

Real registered-input subset
Six cases use the existing independently checked actual-source-inputs.json
(SHA256 3759502a6a4a931a547139d28bf1482b338e27eb25038df79ff49c92e640123f)
from combat-value-source. The second Rust test checks every selected field,
definition ID/source/hash/span and filtered native list order against that
receipt. This test does not itself reopen game archives; the existing runtime
source test separately does the actual current-source binding.

The ten definitions, in FILTERED native manager order, are usa_corps,
usa_airfield, gbr_corps, regiment_usa_infantry, herd_horse, usa_infantry,
rus_infantry, jpn_infantry, gbr_infantry and ger_infantry. Ineligible entries
remain present and are gated by their real flags. The eligible combat subset
produces PC24 attack 0x41755556/offense 0x41655556/lifetime 0x415F417C;
PC53/64 produce 0x41755555/0x41655555/0x415F417D. This visible one-ULP
difference is why a restore-world PC or an assumed source PC cannot be used.
These are NOT the means of all 1028 registered ActorData definitions.

No Settlement extent fallback is supplied. The separate provenance census
proved zero shipped registered Settlement components and zero WallSets, so
that branch is unreached for that shipped source receipt. This coordinator
still requires extent() when a supplied owner actually reaches bit14; raw
wall_diameter is never substituted for a completed 460940 fitted radius.

Remaining requirements
- Actual shared source graph owner and receipts for the preceding native
  completion passes (4BB480/4BB130/4BB730) on every reached definition.
- Actual source-load x87/CRT environment, rather than the diagnostic explicit
  precisions exercised here. No native startup FP capture is claimed.
- Actual typed discard binding and source ownership. The new source-context
  binder can provide it under explicit context; it is not wired as a default.
- Actual all-manager values and the later 4BB590/4618B0 recursive shared-owner
  completion. Existing snapshot formula ports are retained and are not a
  receipt that this full source graph already completed.
- Effective CSV UVar/profile context for the later actor-value pass; no active
  user profile is opened and no empty/default override is assumed.
- Per-record Init/PostCreate, live world/Performer production and full battle
  lifecycle remain separate work. No playable battle or completed +1E4 claim.

Supported numeric domain is finite round-to-nearest arithmetic, normal x87
intermediates as defined by aa_numeric, supported binary32 stores, safe trim
indices, nonempty trimmed ranges and a nonzero terminal offense divisor.
Native exception flags/traps, nonfinite/overflow/unsafe-index behavior and
allocator/SEH failure or allocator callback reentry are explicitly excluded.
Growable Rust columns do not retain the legacy initial allocation capacity.

Validation
Native: python research/skirmish-20261016-combat-means/native_means.py
Rust: cargo test -p aa-combat --test combat_mean_coordinator --release --locked
      --offline -j 4 --target-dir target/s-settlement-completion -- --nocapture
Clippy: cargo clippy -p aa-combat --all-targets --release --locked --offline
        -j 4 --target-dir target/s-settlement-completion
Final local test log tests-v2.log: 2/2 tests pass. clippy.log: no warnings.
Independent review accepted the frozen source and fixture: fresh 210-case
native replay matches byte-for-byte; the reviewer also ran the two new tests
and four existing native combat-value tests (6/6 pass) and independently
checked 52 instructions in three original/v14 byte-identical ranges.
See independent-review.json (SHA256
35f1c6a8f375bcfbc86206cf556df94a232576ef3f9f9f6228c3a0951f797e29).

Superseded attempts are retained and excluded from the final claim. The first
native construction attempted an unnecessary write to unmapped 02255064;
removing that unrelated address fixed the harness before native execution.
native-run-v2.log records the superseded 204-case run. The reviewer strengthened
the two-ability captured-count probe and added the captured-definition probe;
native-run-v3.log records the final 210 cases. tests-v1.log failed only because
a laboratory guard serialized as JSON bool while the test expected an integer;
the fixture now serializes native guard bytes as integers. No native behavior
or trace comparison was weakened to make that correction.
