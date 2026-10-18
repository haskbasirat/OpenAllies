Native Builder ActorData targets, 2026-10-16
==========================================

Implementation and API
----------------------
crates/aa-rules/src/native_builder_data.rs projects native BuilderComponent
BuildActor.actor_ids into ActorData+504 head/+508 tail lists. The persistent,
immutable NativeBuilderData arena owns stable list-node handles, canonical
target identities, and final head-first target vectors, including duplicates.
Catalog (root-owned integration) caches one Arc arena for its source lifetime.
No new parser, source-order loader, dependencies or manifests were changed.

ActorCatalog::native_builder_inputs() produces registered owners and final
typed lookup. Inputs::resolve_partial() retains earlier nodes, head/tail stores
and first failure; Inputs::complete()/native_builder_data() refuse a failed
Builder subsequence. NativeBuilderData::definition(canonical_id) is a direct
canonical lookup; it never applies Translate again. BuilderDefinition.ids is
the stable head-first list used by the runtime adapter. node(handle) exposes
the actual arena chain for bounded consumers and proof.

Native order and errors
-----------------------
ActorData 00464BD0 at 00467F08..00468007 sees BuilderComponent, sets +174 bit18,
walks BuildActor entries in authored order, and creates 16-byte deferred refs.
00473AD0 prepends each reference to the global +824 head. Its Builder vtable
A03474 invokes 00461820, which forwards to 004604C0. 00475230 walks deferred
references head first. Thus owners are visited in reverse native registration
order and each owner's references in reverse authored order.

004604C0 calls FindActor 004BDBA0 with requested kind 0x2B, throws for missing
target, then requires actual target +174 Body bit0. Both failures precede
allocation or list mutation. On success it captures old +504, allocates 8,
writes canonical target and next=old head, reads +508, stores new +504, and
sets +508 only if the prior tail was null. Reverse callbacks plus prepend
produce original authored per-owner order, retaining duplicate targets.

Native4624B0 initializes +504/+508 to zero (EBX is zeroed at4624E1; stores at
462CF9/462CFF). Those instructions are byte pinned, not re-executed as a full
constructor. Successful non-reentrant allocator behavior is an explicit oracle
and source-projection precondition. No arbitrary allocator mutation/exception
or allocation-failure behavior is claimed. Partial unknown/body failures retain
all earlier successful callback nodes, lists and tails; remaining callbacks do
not run. No invalid reference is turned into an empty completed certificate.

Canonical and derived identities
--------------------------------
Registered owner roots are selected by exact source provenance, rather than a
second Find on an owner name that later Translate may shadow. Targets use the
current completed catalog's final lookup once. Native CString lookup stops at
the first U+0000 and folds ASCII case. Node values remain canonical identities,
even if another Translate would redirect that name as a fresh authored query.

Standard ActorData (native_kind2) and RandomActorData (native_kind19) are both
included. Random ctor469980 calls parent4624B0; its LoadData468D20 calls464BD0.
Both actual vtables A05540/A09B30 have +38=4F0500 (returns true). ObjectData's
A13318 +38=4C9FA0 returns false. Eighteen additional original/v14/PC24/53/64
probes execute actual4BDBA0 and those actual vtable leaves, with only427BD0
Find transport supplied. They prove kind2B accepts standard and RandomActor
data and rejects ObjectData. Random ctor/load parent instructions are metadata
pins, not full random-data constructor/loader execution.

Proof and tests
---------------
native_builder_data.py executes actual473AD0/475230/461820/4604C0 against the
original and v14 executable at all three x87 precision modes. Its explicit
seams are final typed4BDBA0 canonical lookup, successful allocator6F7810,
throw410C40 (stops, never returns), CString retain4012C0 and deferred cleanup.
The actual typed4BDBA0 virtual-predicate proof is separate as described above.
Page zero faults except legitimate FS:[0].

252 generated callback cases and six real-source callback cases compare exact
target lists, head/tail handles, node chains, attempted callback order, and
first failure. Tests include duplicates, multiple owner order, late aliases,
embedded NUL/case, empty/unknown targets, and no-Body failures after successful
prefix stores. No failing callback is answered as success.

Five focused Rust tests passed with --release --locked --offline -j4 and
--include-ignored. Source tests preserve a translated/shadowed canonical owner,
avoid retranslating a canonical target, verify derived RandomActor owner/target,
and refuse invalid classes/references and unverified registration order.

Real source census: 1084 canonical ActorData identities (1028 standard and
56 RandomActorData), five Builder components, five nonempty lists, 15 refs.
Each faction's *_engineer builds its bunker, bunker_aa and bunker_artillery in
that exact authored head-first +504 order. All source provenance and existing
catalog diagnostics are retained in actual-source-inputs.json.

Reproduction
------------
 AA_GAME_ROOT=E:\Games\Axis and Alies
 AA_BUILDER_DATA_CAPTURE=<new absolute actual-source-inputs.json>
 cargo test -p aa-rules --test native_builder_data --release --locked --offline -j4 -- --include-ignored --nocapture
 python research/skirmish-20261016-builder-data/native_builder_data.py --write
 python research/skirmish-20261016-builder-data/native_builder_data.py --real --write

Unset AA_BUILDER_DATA_CAPTURE, then rerun the same Rust tests for real fixture
comparison. Running the Python producer without --write checks its fixture
without rewriting it. The two initial synthetic fixture failures lacked the
native list declaration or explicit template binding; both logs are retained.
The canonical alias assertions were preserved, and the corrected fixture binds
the real list-shaped template. No source/parser behavior was weakened.

Limits and historical timing
---------------------------
This is a current-source, Builder-callback-subsequence projection. It assumes
the relevant ActorData LoadData completed and unrelated global deferred
callbacks complete without Builder-visible mutation/failure. It does not certify
global475230 completion, reconstruct prior source-manager failures, or execute
other deferred behavior. Body/Builder flags are the focused presence predicates
set by their source nodes, not an invented full component mask. Registered
historical owner identities are preserved, but arbitrary historical partial
LoadData/manager states remain outside this API's completed-data claim.

No original assets/executables/saves/profiles or releases were modified. No SDK
implementation was copied. Root owns runtime4D4CF0 integration and its proof;
this source arena does not itself complete Actor startup, battles or multiplayer.
