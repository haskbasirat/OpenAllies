Saved scheduled component Init and Detection zone prefix, Development10 work.

Status: author verified; independent review pending. Findings JSON pins sources,
fixtures, actual observations, logs and the copied test executable. No playable
battle, full Actor Init or successful zone solver/fog effect is claimed.

COMMON INIT: native564D80 is the actual primary+8 entry for Construction15,
Detection22 and Lair25. Version>=22 returns before head/queue/RNG reads. Older
versions use the actual Catalog load queue/head owner and original periodic RNG.
Construction/Lair AfterInit is actual4B2830 RET. Native proof90rows/180 separate
original-v14 machines/all3PC has14 executed address pins and one required50AC60
entry. The full existing queue proof is referenced by digest, not substituted
with a successful fake child callback. Runtime gate replay90rows passed1/1.

DETECTION: observed real tutorial02 owners2696 (weurope_farm6x6_01) and6477
(thing_visibilityactorlarge) immediately before56EC90. Both have statebit21=0,
Actor138=0, live deathbit0=0 and positive Detection radius. Actual map version27
skips564D80 queue reads. Metadata is in boundary-01, captured read-only at the
exact held component boundary; the entire diagnostic census is not a proof of
successful earlier records. Source-only actual public binding separately passed
2owners x3livePC while keeping the same Detection Box and Zone Vec allocation.
Its pre-ObjectInit flags differ from the exact boundary by native spatialbit26;
the test explicitly checks the bits this path actually reads, radius and XY.
It never changes actual flags/position to manufacture equality.

ActorShell owns one typed zones_148 allocation, with native lazy constructor
size0/capacity9. Raw148/14C/150 words are absent. map_components/ActorStore move
that same allocation. Constructor-null/cap9 projections occur only in native
oracle tests. The old Organization owner count read also uses this typed size;
no raw zero was reintroduced. Focused ctor/move/creation regressions3/3 passed.

56EC90 creates actual zone6 in this vector through a separate persistent zone
identity domain, then borrows that stored Zone directly through existing
515F40/552640/515970 ports. It keeps the actual allocation on later errors.
The54-row/108-machine original-v14 proof covers two actual inputs plus labelled
dead/zero/negative/clamp/existing-slot probes at all3PC.348 address pins execute;
one allocator entry and two stopped boundaries are counted separately. Required
515A20 stops before reading an unavailable World+248 owner;514CE0 is a stopped
release child. Neither returns fabricated solver success. Active same-radius
and empty disabled paths legitimately return. Positive Actor138/source430 paths,
attached view, actor callbacks and remaining world services stay required.

Fresh immutable native replays:
  python research/skirmish-20261016-load-detection-init/native_scheduled_init.py
  python research/skirmish-20261016-load-detection-init/native_detection_activation.py
Set PYTHONPATH to reconstruction/tools/python-packages. --write is generation
only and must not be used for independent replay.

Copied frozen-activation-tests.exe can run these exact filters without rebuild:
  load_detection_activation::tests::actual_inputs_and_labelled_storage_probes_match54_native_activation_prefixes
  load_detection_activation::tests::actual_two_detection_owners_keep_same_allocations_and_error_prefix_at_all_live_pc --ignored --exact
Actual test requires AA_GAME_ROOT. Leave AA_DETECTION_ACTIVATION_CAPTURE unset
for a read-only replay. A new path makes a create_new actual-output capture.
This EXE is a test binary, not a game release or a Development build.

Build used locked/offline release -j4 with test-only LTO=false/codegen-units16.
Shipping release profile is unchanged. Historical assertion/compile failures
remain in logs and findings; they are not replaced by later success claims.

Next separate work: actual WorldInit solver/presentation/SVar owners and
concrete subsequent callbacks. That work must not turn this prefix proof into
a claim of complete zone effects or whole game initialization.
