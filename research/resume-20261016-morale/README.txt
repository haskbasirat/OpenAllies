ActorData morale source projection, 2026-10-16

What is implemented
-------------------
aa_rules::native::actor_morale::actor_morale_contribution(catalog,
canonical_id, precision) returns the binary32 bits read by original
ActorData getter 45D580 when the modifier-context pointer is null.
actor_morale_source returns the same bits plus actor, field, and declaration
provenance. It uses existing ActorCatalog/ResolvedNode typed resolution.
No parser changes, new dependencies, numeric port, or generic ActorData
loader are introduced.

Exact native producer
---------------------
4624B0 standard ActorData constructor: EBX=0 at 4624E1; zero is stored
at ActorData+2C0 at 46280C. Loader 464BD0, fragment 466822..4668A8:
  466822 obtains the actor source node; 466830 calls child lookup 5B4410
  with UTF-16 ElementComponent at A083BC.
  An absent child jumps to 466B17, leaving constructor+2C0 unchanged.
  A present child sets component bit 8 and creates its auxiliary list.
  46688B calls GetFixed 5B5130 with resupply_rate (A083A0), stores+2CC.
  466896 pushes morale_value (A08384); 46689D calls GetFixed 5B5130;
  4668A2 stores the returned value to binary32 ActorData+2C0.
  45D580 with context0 loads+2C0, with no arithmetic or clamp.
The Rust API validates the prior resupply_rate read before morale_value.
An absent component retains the proven constructor zero. Missing,
unassigned, or incorrectly typed morale never becomes a default zero.

Identity boundary
-----------------
The input is an already resolved canonical ActorData IDS. ActorCatalog's
public lookup includes translations. A canonical IDS shadowed by another
translation is explicitly unsupported, instead of following a second
translation to the wrong definition. Dedicated A->B then B->C regression
checks reject B and still resolve unshadowed C. Unknown and non-standard
ActorData definitions are rejected. This is not an exact registered-ID
resolver for arbitrary shadowed registries.

Native oracle
-------------
native_morale.py executes the original TGI parser 5BD080, original
constructor store 46280C, original loader fragment 466822..4668A8 (or the
absent-component branch 466B17), and original 45D580(NULL). A fresh fixture
instruction stores the getter's ST0 to binary32 solely for observation.
Original/v14 bytes are compared for every executed native instruction
window. Heap, line source, template registry, mutex, C++ throw capture,
and page-zero guarding reuse the established layout/definition harness.

The diagnostic CString vararg formatter 405D60 is explicitly a seam:
the actual format string is recorded and placeholder text is supplied.
Wrong-float accessor 5B7D30's native branch, message construction and final
96BEF5 throw execute; this is not a fabricated invalid-type success.
Exact fully formatted diagnostic messages are not claimed.

48 rows = 16 cases at PC24/53/64: zero, negative zero, positive/negative
values, decimal fractions, percent, precision-sensitive literals, absent
component, unassigned field, missing field, wrong float type, inherited
morale, and inherited override. Unassigned required field is rejected by
the native parser. Missing/wrong-type fields throw before the morale store.
All accepted producer words equal the unchanged null-context getter words.

Source coverage
---------------
source-census.json pins all 1647 files loaded through the current original/
v14 native-order source resolver. All 1959 registered Thing definitions are
classified: 1028 standard ActorData definitions, of which 372 have an
ElementComponent, and 931 other kinds. Every standard actor resolves at
PC24/53/64 with no category or field error. The 11 existing syntax notices
are retained in the report. This census checks the Rust projection across
real source inputs; it is not execution of a full native ActorData loader
for every actor. Unreferenced source files are outside the native-order
catalog census.

Validation
----------
Four dedicated tests passed, zero ignored when run with --include-ignored:
  parser_producer_getter_match_original_x86
  rejects_unknown_and_non_actor_definition
  rejects_a_shadowed_canonical_identity_without_translating_twice
  every_shipped_standard_actor_morale_projection
Log: tests.log. Source-only run: source-tests.log.

Reproduce in PowerShell from rust-rebuild:
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python research/resume-20261016-morale/native_morale.py
  $env:CARGO_TARGET_DIR='target/s-layouts'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:AA_MORALE_CENSUS='E:\Games\Axis and Alies\rust-rebuild\research\resume-20261016-morale\source-census.json'
  cargo test -p aa-rules --release --locked --offline -j4 --test native_actor_morale -- --include-ignored --nocapture

The first oracle attempt omitted an explicit template header in synthetic
source text, so ordinary definitions did not inherit the test category;
the native fixture was regenerated from corrected input bytes. No expected
JSON results were hand-edited. initial-tests-failed.log is retained. A
source census run used a relative output path (tests run from crate cwd),
failed only when saving output, and passed again with an absolute path;
initial-census-output-path-failed.log is retained. probe_wrong_type.py and
probe.log preserve diagnosis of the formatter dependency before adding its
explicit diagnostic-only seam.

Limits
------
This is a bounded source getter for freshly constructed standard ActorData.
It is not the full ElementComponent/ActorData loader, validation of all
earlier/later component fields, ActorData hot reload, non-null modifier
context 501C90, live Properties state, Organization restoration, gameplay,
or end-to-end match parity. Integration belongs to the parent task.
No engine cap or gameplay default is changed. Broad combined-source
aa-rules real-data and aa-metagame checks are coordinated by the parent.

