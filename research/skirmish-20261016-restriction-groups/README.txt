Loaded RESTRICTION_GROUPS before Team/Kingdom Init
================================================

Production:
  aa-mapworld::restriction_groups::RestrictionGroups
  aa-runtime::restriction_groups::LoadedRestrictionGroups

The runtime owner is constructed from an identity-checked exact Chunk and the
current Catalog, with explicit actual source precision. It is not Default,
Deserialize or Clone. It retains the exact map/payload receipt, decoded allowed
and restricted lists, every attempted current typed lookup and canonical source
provenance, the same cached Arc<NativeLayoutData> Property arena used by saved
actors, source precision and the closed source registration projection.
profiles() borrows the existing aa-economy RestrictionProfiles kernel. The owner
must be persisted once by the live LoadWorld and require_catalog checked before
using it. Creating another identical-looking Property arena is rejected. An
installed Bank source precision and full source identities must match Catalog,
independently of live Init PC. A separately labelled diagnostic Bound owner is
used only to test those mismatch guards; completed means still reject it.

Original code and wire order
----------------------------
4FCA33..4FCA86 gates world version>=8, opens RESTRICTION_GROUPS, calls505600 at
4FCA4F, then opens TEAMS at4FCA68. This is a static original/v14 caller-byte
pin, not execution/certification of the whole4FC600 caller.

505600 clears the manager, reads envelope version0, manager word+0, then a
signed profile count. Profile5049C0 reads envelope version0..4, name, stored
word+0 and localized label before the allowed actor/property lists. Profile2..4
then reads explicit restricted actor/property lists. Counts use the native
signed-positive loops; each string has its original u16 UTF-16-unit length.
The original finite wire widths are not game RAM/network/actor allocation caps.
Checked decoding rejects truncation, invalid version, wrong chunk identity and
trailing bytes. It never treats missing serialized fields as empty lists.

For49AD70, KeyOnly means stream virtual48=true and59E520=false (authored RMP
loading); nonempty keys then consume no inline value. KeyAndValue is a distinct
explicit storage contract. Runtime translation/display-string effects are not
claimed here. Version0's discarded legacy label is retained in the input view.

Fresh runtime manager semantics:
  *4BDBA0(ids,2B) accepts standard and RandomActorData virtual38 predicates.
   Unknown/type-mismatched actors are native skips, retained as null lookup
   receipts. Current Thing Translate is applied once; canonical returned IDS is
   not translated again. Private resolved root identity/category/Translate keys
   are checked against the public ActorCatalog lookup transport. Public omission,
   rename, duplicate registration or reordered lookup transport cannot silently
   manufacture an unknown or another target.
  *463FA0 on GameData+464 selects registered Property identities from the one
   existing NativeLayoutData owner. Explicit current lists accept any registered
   Property; they do not apply the legacy is_technology complement predicate.
  *Duplicate allowed/restricted Actor entries are logged/skipped (native lines
   188/1C4). Property duplicates remain separate nonnull entries.
  *504450 prepends hash nodes.593570 walks old buckets/chains and prepends into
   grown buckets, reversing earlier collision-chain precedence. The portable
   producer preserves that lookup order for duplicate profile names, including
   across rehash. It does not assume that the newest equal profile always wins.
  *The query kernel consumes restricted maps at+34/+44, using canonical string
   keys and nonnull membership. Allowed maps are retained as decoded inputs and
   complete lookup/duplicate receipts; they are unused by current-version query.

Actual shipped census and independent capture
---------------------------------------------
All80 authored RMPs have this chunk, manager envelope0.24 chunks are nonempty,
with37 profile records, all envelope4. Total serialized references:23679 allowed
actors,2093 allowed Properties,762 restricted actors,247 restricted Properties.
Campaign/tutorial02.RMP is30778 bytes, manager[envelope0,word3,count1], profile4.
Its actual GWORLD envelope is27 (not23), and TEAMS has one absent slot.

source-lookups.runtime-capture.json was captured through the production owner
over all80 maps. Independently compiled capture_source_lookups.rs reads the
same authored chunks and existing native-order Thing/Property source registries
without creating runtime profiles/World. Its maps/lookups/source_files exactly
match that production capture. source-lookups.json retains this independent
current typed transport input receipt; it is not a fabricated unknown registry.

Native oracle and seams
-----------------------
native_restriction_groups.py executes505600,5049C0,5042D0,504150, map clear,
504450,593570, current typed4BDBA0 and463FA0 wrapper bodies and the actual
Actor/RandomActor true and ObjectData false virtual38 leaves. Both PE images
execute separately at PC24/53/64. Executed address bytes are original=v14;
the vtable predicate words and caller range are pinned separately.

Explicit nonexecuted seams are source lookup transport427BD0/4915F0 (fed the
independent actual current source receipt), stream reads, CString storage/copy,
allocation/free, C-locale ASCII _wcsicmp, diagnostic log and localized table
lookup. The underlying full Find/Translate/history manager code is not newly
certified by these transport seams; its current lookup comes from the existing
source producers. Native allocation failure/reentry/SEH destruction effects,
display localization and whole source startup are not claimed. FS page0 is
guarded; only actual FS-prefixed SEH access is admitted. No native trig/CRT/FP
arithmetic is approximated by these string/integer readers.

The native proof includes all80 maps plus synthetic aliases/unknown/type
rejections, duplicate actor and Property entries,64-profile multiple rehash,
a40-profile case where rehash changes which equal profile is visible,
canonical-shadow identities requiring exactly one Translate, and schema2/3.
Rust replay checks complete wire consumption, manager metadata, typed lookup
order/results hashes, duplicate-profile lookup precedence and restricted maps.
Portable tests also cover wrong identity, every truncated byte, signed-negative
counts, localization storage modes, public registry corruption, foreign Property
arena, and explicit legacy-complement refusal. Final hashes/results:findings.json.

Boundaries
----------
This is a fresh, successful owned manager load, not a resumable native failed
manager reload. The caller must abort failed startup; no partially loaded World
is certified. There is no empty-profile fallback for an unavailable chunk or
unbound producer. Native unknown references are genuine documented skips only.

Profile0/1 input decoding identifies the absence of explicit restricted lists;
runtime binding refuses with a required native complement error. Supporting them
requires actual4BC6C0 category iteration (19,42,20) and its native linked category
list order, the allowed-map complement and GameData+464 registration vector
enumeration with Property+5C is_technology. These are not guessed from names or
defaulted to empty. No shipped profile currently reaches this historical path.

Existing NativeLayoutData documents its own source projection limitations,
including unported ModifierPropertyData effects. This owner only borrows that
producer's canonical registered Property identities; it does not turn them into
a certificate for unrelated modifiers, whole global source lifecycle or battle.
Non-ASCII canonical keys remain explicit unsupported until native locale/hash
behavior is proved. All shipped profile/reference keys are in the ASCII domain.

Reproduce
---------
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python -B research/skirmish-20261016-restriction-groups/native_restriction_groups.py
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  # AA_CAPTURE_RESTRICTIONS must remain unset for immutable replay.
  cargo test -p aa-mapworld --test restriction_groups --release --locked --offline -j4
  cargo test -p aa-runtime --test restriction_groups --release --locked --offline -j4 -- --include-ignored

Native script is immutable by default; --write is an author fixture generation
operation. AA_CAPTURE_RESTRICTIONS is likewise an author-only source capture,
not normal validation. Normal tests must not overwrite either reference receipt.
