SAVED KINGDOM FLOW STORAGE -- 2026-10-16

Scope
The runtime adapter saved_kingdom_flows::BorrowedKingdom applies the proven
aa_economy::borrowed_flows arithmetic directly to SpecialShell's authoritative
1A0 effective, 1AC raw, and 1B8 consumed vectors. No state clone, ownership
conversion, resizing, reset, stockpile mutation, or rollback is performed.
Missing storage is an explicit error at the first reached native access.
Earlier stores survive a later missing cap, input, or destination slot.

Native ownership and timing
Original4E5370 creates the three flow vectors with length0/capacity8. Their
allocation contents are not initialized values. End helper4E2EA0 sizes and
zeros1C4 stockpiles and258 starting resources only. The existing complete
constructor oracle is research/phase7-restoration/native-special-constructors.json.
Original4E8550 Load restores stockpiles but does not initialize the flow arrays.
Only the real4E5DF0 Init path may reach4E5EAB..4E5FB5 after Actor4D7230 succeeds
and any pre-version22 scheduling completes. The reset must not be inserted as
an Economy refresh fallback. Team membership, recruit initialization, SAI,
world registration and later PostCreate effects remain required lifecycle work.

Independent lifecycle proof is in lifecycle-proof/, owned by restore_review.
It pins original/v14 vtables and proves60 bounded dispatch cases:4FAE80 Load,
bit7 decision, immediate Init(NULL), PostCreate(NULL,1), then4F9510 registration.
Team records precede Kingdom records; those precede static/dynamic records.
The global PostLoad pass is later and does not substitute for this lifecycle.
Its dispatch leaves do not claim complete Init bodies or whole-world success.

Natural saved-map observations
The bounded diagnostic restores the unmodified original tutorial02 and
DeveloperMaps/(3v3)Firefight using real v14 definitions and Settings. It makes
no actor edits and does not reset arrays. All3 tutorial and7 Firefight Kingdoms
have bit7 clear, yet all three flow lengths remain0 with capacity8; stockpile
length is7. Thus bit7 deferral cannot explain the missing flow initialization.
natural-two-map-load.json records the initial snapshot; later explicitly named
snapshots retain the source check and effects of concurrent runtime integration.
InspectPartial diagnostics are not completed native lifecycle/publication or
playable battles. In particular, an AwaitingPostLoad marker is insufficient
evidence of successful per-record Init in the current restoration architecture.

Source binding
Reuse kingdom_economy::Definitions::bind(ResourceCatalog,ActorCatalog,precision).
It verifies source hashes and native Resource order. Runtime Catalog's root-owned
cache supplies that immutable definition object; this adapter adds no parser.

consumption_count(&Definitions) reads that actual registry count with checked
signed conversion. This does not infer a count from a Kingdom vector:
48A8E0 constructs Resource manager47F670 at GameData+10;47F670 installs that
same pointer into B3A660. Therefore B3A660+28 and B36530+38 denote the exact
same count word. native-source-bindings.json contains24 original/v14 window
executions acrossPC24/53/64 and counts0,1,7,17. Only those original constructor
windows are executed; whole GameData construction is not claimed.

provider_maximum(&Catalog,precision) is called lazily after the raw store.
Native50880C registers EconomyLimitedResourceMax as the integer variable at
fixed index109, cached pointer offset1B4. The existing native wildcard sorting
proof puts SVars.tgi before SVars_AA.tgi. New42 original/v14 runs execute typed
integer insertion5BBE70 and complete Settings4B3C10, demonstrating sequential
overwrite of the same type3/flags0 variable. The helper requires exactly those
two Int declarations, matching Settings/Actor source hashes, no authored
WorldRule for this variable, and supported signed decimal i32 values. It neither
returns a guessed default nor accepts an unknown future WorldRule override.
Real v14 validation proves count7 and cap100 at all three explicit precisions.

Tests
The runtime test replays876 applicable original/v14 rows from the independent
1026-row borrowed-flow oracle. The excluded rows inject synthetic pointer/count
or setter/coefficient faults that the concrete SpecialShell owner cannot expose;
they remain covered by the pure economy primitive tests. All replayed results,
including retained failure prefixes, match; vector allocations, lengths,
capacity hints, and stockpiles remain unchanged. Constructor-only storage is
refused without growth. A short effective vector retains the preceding raw
store and does not call an unused provider-cap callback. Integer settings
fixtures check source order independent of Vec order and source-hash mismatch.

Command (target directory owned by this agent):
  CARGO_TARGET_DIR=target/s-saved-kingdom-flows
  AA_GAME_ROOT=E:\Games\Axis and Alies
  cargo test -p aa-runtime --test saved_kingdom_flows --release --locked --offline
    -j4 -- --include-ignored --nocapture
See tests.log for the final result. No original game files, saves, release
artifacts, shared parser files, or runtime restoration coordinators were edited.
The lib.rs export's previous state is in backups/skirmish-20261016-saved-flows/.

Remaining boundary
The actual saved per-record Init/PostCreate/registration chain must be bound
before constructor-only Kingdoms can accept nonzero resource-domain flow calls.
This adapter supports genuinely initialized canonical storage and never hides
that missing lifecycle. No claim is made that728 nonnull-owner failures, full
skirmish, multiplayer, or campaign are completed by this work.
