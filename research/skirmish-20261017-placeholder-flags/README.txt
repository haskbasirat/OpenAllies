Placeholder replacement flags,2026-10-17

This is an immutable unchanged-definition projection, not full4BC7F0/source-
loader/World readiness. It does not remove placement's required-storage guard.

Native4BC7F0 first verifies RandomObject receivers, then RandomActor receivers,
then each category31 receiver against all Nations, then calls each Thing's8C.
Actual4BC5E0 passes replacement+38 as THIS and original+38 as argument to46D8F0.
A failed containment reaches410C40 BEFORE flag comparison. It then calls the
ORIGINAL definition's virtual3C and chooses A74C00 forFALSE, A34C00 forTRUE.
On mismatch it calls404ED0 BEFORE rereading/storing the two masked writes.

Important correction: standard ActorData4624B0 installs A05540. Its3C is
4C9FA0 xor eax,eax; being an Actor does not imply TRUE. The earlier diagnostic
withdrawn-kind-heuristic-census.json used the wrong premise and is explicitly
withdrawn. Corrected source_census.rs/actual-source-census.json use actualFALSE.
No production state ever used the withdrawn diagnostic mask.

46D8F0 is reproduced literally for reached finite fields. If argument.kind is
larger it swaps the two and negates the recursive Boolean. The rect/circle
branch calls444420 on the ITEM, not the container; it is not replaced with an
intuitively designed geometric containment rule. Rectangle comparisons use
positive half-extents and allow equality. Unread radius words can be NaN;
reached nonfinite fields remain explicit unsupported errors.

State::bind borrows actual Catalog/Bank/source-PC and the sealed private
NativeSourceRequirements owner. It invokes existing skirmish_start::Definitions
once itself; a caller cannot pass forged public pairs as proof. That producer
already binds historical immediate placeholder/deferred replacement references
with reverse/prepend order. All canonical Thing/Bank tokens/provenance and the
complete registered Thing census are checked. Existing source requirements
exclude all resolved SettlementComponent shared writes and nonempty Upgrade
graphs; the actual random receiver census is kinds18/19, never category31 kind2.
Actual standard ActorData38 points4F0500(TRUE), so49C460's nonactor-ownable
clear cannot alter these placeholders. Other8C verification effects are not
certified as globally completed by this flag projection.

Each category31 must have real standard ActorData kind2 storage and every Nation
must provide a positive replacement. Missing/null entries require actual current
DataCheckingStrict/diagnostic ownership and are refused. Each native containment
must pass and each A74C00 comparison must already be unchanged. If any mask
differs, actual warning/mutable source/order ownership is required; this owner
never silently chooses a last Nation or fabricates a final word.

The private State retains immutable source identity/provenance/category/flags
and source PC. flags validates the current same source universe and Bank PC,
canonical source/category/base flags on every read. Root retains the immutable
cache INSIDE THE SAME Bank, so staging moves both together. Catalog's early-load
helper can bind once; its explicit in-bank helper reads the same already-bound
cache after the old Catalog Bank slot is empty. Every cached call validates the
field-writer source and PC. Those shared files are evolving; only the specific
methods/forwarders are within this review scope.

Actual source diagnostic census:1959 Things,56 Random targets,6 Placeholders,
5 Nations,30 positive pairs perPC. All90 comparisons at3PC are unchanged under
the correct mask; complete explicit Upgrade census is empty. Actual Native
4BC5E0 replay confirms all90 containments and no warnings/stores. This diagnostic
census alone was not treated as a source-completed Bank certificate.

native_placeholder_flags.py additionally runs204 synthetic shape/mask/error-
prefix rows at all3PC. Total294 rows/588 separate original-v14 executions.
It executes actual standard ActorData false virtual; labelled alternate-vtable
probes execute real4F0500(TRUE). Diagnostics are explicitly named seams; full
mismatch rows execute native flag stores, whereas production deliberately stops
at the required mutable-writer boundary. Native bytes/vtable pins are retained.
Normal invocation is immutable; --write is fixture generation only.

Source changes are confined to new runtime facade/tests/export, not aa-rules
parsing or definition loading. Existing source producer and source-writer proof
are reused. Test-only no-LTO/codegen-units16 override does not change shipping
ThinLTO defaults. Original binaries/assets/profiles remain untouched.

Final cache regression: frozen-cache-tests.exe replays both runtime unit tests
with AA_GAME_ROOT and --include-ignored:2/2 PASS,13.41s. It tests immutable native
rows, the actual completed source factory, warm wrong-PC/active mutable Bank
loan refusal, and the same Bank moved out of the old Catalog slot. The private
cache is neither cloned nor reinstalled. Exact binary/log and the two shared
Catalog method snapshots are pinned by the final freeze.json; the historical
pre-cache freeze is retained separately. Full fresh creation/mission/battle
completion is outside this bounded flag checkpoint.
