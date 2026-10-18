KKC_VGrid430290 /869C30 /42F540

This is VWorld+94 view-spatial storage, not an input/controller owner.
The same move-only State<V> owns growable per-cell lists of actual typed VThing
handles. Constructor-unwritten28..9F remains unknown. No GTIDs, picking scene,
raw pointer guesses or zero global defaults stand in for this owner.

Native proof:795 paired rows /1590 separate original-v14 executions at all3PCs.
Explicit transported global words include signed zero, NaN payloads and values
changing on each read. Every labelled pre-read/pre-store/allocation/publication
cutoff is tested; real CRT ceil/truncate and native zero fill execute.32 block
windows are pinned. Heap and process-global values are declared host inputs.

Rust proof:2/2 tests passed, including all795 exact result/event/prefix rows and
a missing-global constructor error retaining completed stores/unknown cells.
The current source/tests compiled using a small standalone rustc wrapper with
existing dependencies, not a whole-runtime Cargo build. See frozen executable
and hashes in findings.json. Production Cargo --lib check separately passed.

The caller43FD35..55 reads actual World+3C/+40, which was copied from parsed
resource Creator at WorldInit before final selected-Creator replacement.
The public initialization API therefore accepts that retained typed State.
First grid allocation preserves native ceil/alignment/shift and scalar store
order. Native fixed pointer/cell allocation becomes a growable Rust owner.

Remaining required services: concrete current global seed owner (five native
BSS words are zero initially, but XYZ address escapes to other helpers, so no
current immutability claim); current UVar shift producer; actual VWorld hookup;
Occupy42F860/Unoccupy430410 and repeated-allocation retirement; exceptional
float/int/OOM paths. This report claims no actual VWorld, scene or battle test.

ACTUAL REACHED STARTUP PRODUCER (additive)
Two genuine native Custom skirmish launches now reached430290 entry/return.
Complementary all-thread hardware watches covered all5 current seed words;
both observed zero and no pre-cutoff native writes, with8 same-value isolated
post-cutoff calibration probes. Profile paths were redirected, archives
preserved and both owned processes cleanly reaped. This supplies a sealed
reference seed owner for the known portable prefix, not perpetual native
immutability or battle success. capture01 timed out normally at the menu and
was never used to mint authority.

view_grid_startup::Seeds is now the concrete retained non-Clone owner; its
validated Bound lends the exact reached Globals reads. Two current AVars are
separately retained once in view_grid_variables::State: registered Int/type3/
flags0 defaults16/4, actual source8/4. Required UVar TerrainQuadSize reads the
existing owner through a callback; it is not copied into another current
store. Native signed mins/highest-bit metrics are proved168 paired rows.
Seed verifier2/2 and metrics replay1/1 pass in narrow extraction harnesses;
production checks pass. Actual Catalog factory/live integration is pending.
See startup-findings.json and exact source snapshots for accepted scope.

SAME-OWNER OCCUPANCY (additive)
42FD10/42F860 and430E20/430410 now operate on the same retained State<V>
cell head/tail/node arena and same borrowed VThing38/3C/74/54/68 fields.
Node reuse is growable and typed; no detached render or picking owner exists.
Native693 paired/1386 both-image all-PC calls prove geometry rounding, exact
list order, partial stores, unlink/cleanup/reuse and all declared cutoffs.
Current same-source narrow Rust tests4/4 pass, including old795 ctor rows
and real required diagnostic prefix; production check passes.
Native residual cleanup deliberately does not zero count68; missing diagnostic
errors after the native completed flag/count prefix. No battle/view-directory
integration, other queries, repeated allocation or exceptional numeric paths
are claimed. See occupancy-findings.json and frozen current source copies.

APPLICATION STARTUP INPUT COMPOSITION (additive, actual acceptance pending)
view_startup_inputs::Inputs moves the existing sealed Seeds, current Grid Int
cells and RendererDefaultOrientation owner into one move-only application
bundle. A required completed source Bank and sealed receipt are checked before
construction. Failed source parsing retains every reached partial owner; an
incomplete cell owner cannot supply validated view inputs. No new current cells
or complete manager mirrors are created by this composition.

world_ready_services::Application retains an optional bundle installed once.
Duplicate/invalid installation returns the incoming owner whole. Its dedicated
ViewStartupParts lends the SAME Loading and Inputs allocations; the established
Parts API remains unchanged. Validation accepts the authoritative moved Bank.

Production Cargo check passes. A metadata-only test check was blocked by an
unrelated path-included world_revision test root. Small actual-source replay
attempts were blocked by stale compiled dependency hashes/private old APIs;
they were not executed, and no checks were stubbed. The ignored actual-source
bundle/Application test is saved for the root cohort. See the qualified
bundle-findings-pending-actual.json and retained build logs. Camera/alias source
owners are approved future additions after their own frozen proof/factories.
