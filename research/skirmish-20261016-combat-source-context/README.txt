Source-phase combat-value settings/context, 2026-10-16

New isolated module
-------------------
crates/aa-runtime/src/combat_value_source_context.rs, with one append-only
module export. No parser/loader, root manifests, saves, profiles, original
executables/assets or releases were changed. Root exposed the existing
Catalog::require_source_owner as pub(crate); this module uses that same guard.
No completed CV value or fake battle-start callback was added.

SourceFpContext has NO Default. supplied_diagnostic requires the actual caller
to state its assumed x87 control/status, CRT SSE2 availability, MXCSR and a
nonempty diagnostic label. PC24/53/64 derives from the supplied control word;
reserved PC or non-nearest rounding is rejected. CRT dispatch must select the
already ported SSE2 path, with default rounding/FTZ/DAZ controls. Status flags
are retained as context, not presented as newly emulated exception behavior.

Every supplied context explicitly reports is_observed_native_startup=false.
The constructor does not capture hardware, Windows/D3D state or the original
application. In particular, restore PC53 is NOT silently substituted for the
earlier main-thread source-load precision. The renderer precedes completion;
the prior static PC24 expectation is not a captured native receipt. A real
application source-context lifecycle binding remains required for publication.

Numeric binding
---------------
bind_numeric(&Catalog,&SourceFpContext) borrows the Catalog, retaining its
source owner and actual Settings declarations. It binds all10 values:

  A4   CombatDamageMinimum
  144  CombatValueRangePower
  148  CombatValueAreaPower
  14C  CombatValueOffenseWeight
  150  CombatValueDefenseWeight
  154  CombatValueMovementWeight
  158  CombatValueMovementDefault
  15C  CombatValueMeanValuesDiscardPercent
  lazy CombatValueRangedMinimum
  lazy CombatValueDefenseWeightRanged

Native variable kind is float2, authored field type is Fixed4. Base
Game/SVars.tgi precedes Game/SVars_AA.tgi, at most one assignment per input
file. Missing assignments, other source files, wrong types and duplicates
remain errors. No hardcoded registration default replaces a missing source.
Each declaration's source hash must match the native-order Catalog. Its
typed value, exact source span and bits are independently cross-checked against
the Catalog's actual resolved SVars node; caller-mutated public Settings.raw
is not blessed just because it retains an old source hash. Unknown loader
diagnostics must be classified; known native syntax notices are retained.

474C50 applies registered WorldRule defaults before CV completion. The binder
requires the actual native WorldRule registry and refuses any rule registered
under one of these10 IDS until its real default application is bound. Actual
current data has9 WorldRules, none targeting these10 values.

formula_settings returns the seven existing aa_combat::combat_value::Settings
fields; mean_discard and the two lazy source values remain separate. This
immutable source owner is not a replacement for live mutable SVars or an
implementation of lazy global cache lifetime.

CombatValueCSV ownership and the user-file correction
---------------------------------------------------
The current root wildcard set is UVars.tgi, UVars_AA.tgi, UVars_Restored.tgi.
The first two explicitly assign CombatValueCSV=null; the third has no such
assignment. All files/hashes and every effective assignment are retained.

bind_csv_from_sources resolves the complete original/v14 root UVars wildcard
set, in native ordinal ASCII filename order. bind_csv_files is the explicit
laboratory-input entry point. Case-colliding filenames are rejected globally,
including nonadjacent collisions after case-sensitive sorting.

CsvScope::Unknown is an error. FreshNoProfile is an explicitly labeled fresh
DIAGNOSTIC application, not evidence about the user's actual original profile.
SuppliedOrderedOverrides requires caller-provided authorized bytes, COMPLETE
order, label and OverrideFile.native_load_flags; no filesystem/profile discovery
is performed by this module. Supported native flags are0 and3 only.

Important native fact recovered during review: root UVars passes4B43B0 flags0
at419C6A..419C90. UserUVarsFilename passes3 at419F72/419F82.4B43B0 sets
Settings+24 from flags&1. In restricted mode4B3C10 does NOT overwrite an
existing variable without native flag16. CombatValueCSV is source-created
String4/flags0, so this user-file stage cannot change it. Ignored assignments
are retained with source provenance and input flags, without changing value.

The missing-variable branch runs BEFORE that restriction. Therefore this
binder explicitly requires an actual root-created CSV assignment before any
supplied override, rather than inventing an existing flags0 variable. It does
not claim the restricted stage could never create an absent variable.

Ordinary supplied flags0 documents overwrite the same source-variable identity.
Nonempty CSV values remain an explicit4BB590 debug-stream error; they are never
silently replaced with empty. CString length is the gate, not NUL-prefix text.
The bounded text reader supports typed plain ASCII String sources and the empty
quoted string. Unknown quoted/expanded/non-ASCII forms are unsupported. It may
conservatively reject an unsupported restricted document even where native
Settings would skip its field; no unsupported form is turned into success.

Native evidence
---------------
native_context.py executes both pinned original and v14 images at all3 PCs:
*180 numeric cases: full original Fixed assignment5B9E20 and native CRT parse,
 actual field insertion, Settings constructor4B2CE0, float constructor401710,
 Add4B3470 and full Settings4B3C10/4B2F80/40E0A0 overwrite. Cases preserve one
 variable identity/type2/flags0, include real10 source literals plus ordered
 replacements, percentages, negative zero and large decimal rounding.
*36 CSV cases: full plain String assignment5BB0B0, source-created String
 variable and sequential normal/restricted Settings loads.12 rows specifically
 prove flags3 preserves the preceding root value rather than applying override.
*No unguarded page-zero accesses. Only FS-prefixed accesses to the SEH word
 are allowed. All genuine executed instruction bytes are pinned against both
 images; inherited allocator/string/TLS hook ENTRY bytes are explicitly omitted
 from the executed-instruction pin set.

Allocations, string-pool/TLS and source-name/logging are declared seams. Source
field documents and initial constructor values are explicit laboratory inputs.
This is not full wildcard discovery, native application startup, profile import,
hardcoded/locked/bounded variable-policy coverage or completed CV arithmetic.
static-context.json separately pins the original startup caller/loader flags.

Validation and remaining work
-----------------------------
Focused tests replay numeric/CSV outcomes, source-owner/raw/span refusal,
unsupported FP/CRT modes, WorldRule presence, unknown profile scope and the
filename-collision regression. Actual data binds all10 values at each explicit
PC, verifies no matching WorldRules and empty CSV from all3 hash-checked roots.
Final input evidence is actual-source-inputs-final.json; the earlier smaller
export is retained as superseded schema evidence. No user profile was read.

Exploratory failed logs are preserved. tests.log stopped while a root-owned
forward module was not yet present. tests-v2.log mixed a pre-fix library build
with the newly added nonadjacent-case regression and exposed the old collision
guard; it was fixed with a global lowercase filename set. tests-accepted.log
passed all4 after that fix. tests-final.log passes all4 against the final216
native cases and the final conservative unquoted-empty refusal. A direct final
real-data run exported the current binding schema (1 pass). Ordinary library
Clippy succeeds with3 warnings confined to unchanged restoration.rs and
saved_regiment_mind.rs; this module has no warnings or added allowances. No
strict-warning cleanliness is claimed for the existing crate. No scope check
was removed to make those runs pass.

These are real source inputs under an explicit diagnostic context. Global
means, all reached shared ActorData/Ability source completion, source-load
FP/CRT receipt, completed+1E4/+30C/+36C publication and live Performer+60 remain
separate work. The Settlement census shows no reached SettlementComponent in
the current1028 registrations, so460940 fitting must not be blamed as the
current shipped-data CV blocker. Full battle startup is still unfinished.
