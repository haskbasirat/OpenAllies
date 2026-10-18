PHASE 6: SOURCE TEAMS, KINGDOMS AND ACTOR OWNER IDENTITIES

The typed aa-mapworld reader now decodes every original TEAMS and KINGDOMS
chunk sequentially. KINGDOMS has no record-length index; record ends come from
the recovered native virtual Load schemas, never searches for IDs or names.
All80 original maps contain21 present teams and328 present kingdoms. Every
Kingdom saved ID and every Team saved ID matches the restored IDMGR generation.

Runtime-facing API:
- Teams::decode(&Chunk), Kingdoms::decode(&Chunk, world_version)
- WorldInventory has optional teams/kingdoms plus explicit decoding errors.
  Unsupported schemas preserve the full original opaque chunk and hash.
- OwnershipInventory::from_sources maps raw GActor owner handles to exact saved
  Kingdom IDs. runtime_owner_slot(record_ordinal, final_runtime_flags) applies
  the original bit23 owner gate and returns the source Kingdom slot. The caller
  must provide resolved Thing flags, not saved raw flags or a guessed player ID.
- All11701 class3 Actor prefixes resolve:4189 null and7512 non-null Kingdom
  references, no missing/duplicate/stale targets. This is a source binding, not
  creation or publication of a live Kingdom object. Full kingdom constructors,
  definition registry lookups, recruited objects, queue restoration, component
  fixups and complete live world startup remain future work.
- Crete owner12344 belongs to saved Kingdom slot4; it is a minor kingdom whose
  word18C is FFFFFFFF. Header player slots cannot substitute for these handles.

Native proof:
- native_teams.py executes complete original4E6970 GTeam::Load,4EF160 GThing,
  4CD0F0 vectors,49AD70 localization key and8589B0 version reads. All80 chunks
  end at their exact source byte length. Host string storage/translation lookup,
  allocation and stream I/O are explicit seams. Key-only RMP localization mode
  is proven, not assumed to be every possible saved-game localization mode.
- native_kingdoms.py executes complete original4E8550 GKingdom suffix on every
  authored record. The preceding Thing/Object/Actor wire is an explicit seam:
  bounded base decoding plus independently executed native ActorTail, covered
  by phase6-mapworld and phase6-actor-payloads. Definition/reference lookup uses
  unresolved results; no application of real registry definitions is claimed.
  Commander construction is a host seam with zero live ability slots, so the
  actual native loader takes its mismatch fallback and consumes each original
  serialized Ability through4C8B40. All serialized byte reads still execute.
- Original50BCD0 Updatable,4E6C20 recruitment,4CE0B0 commander,4C8B40 ability,
  58DBE0 tactical,58EE80 strategic and58F2E0 strategy-entry readers execute.
  Version0 strategic wire has60 words; version1 has66. Raw words retain order;
  no uninitialized/gap runtime fields are fabricated.
- 4FCDC6 verifies Kingdom.word188 equals source slot. word18C remains separately
  preserved and may be FFFFFFFF. Ghidra's decompilation incorrectly removed
  several genuine v12/v15/v16 loops; original x86 control flow is authoritative.
- native_leaves.py executes23 synthetic variable-count/version cases for
  Properties0..3, Ability0..1, Tactical0..2 and Strategic0..1. These fixtures
  verify field order and real native stream consumption, including older gates.
- Root's recruitment.rs has separate original list/item oracle coverage.

Preservation and explicit limits:
All raw float words, UTF16 units, presence bytes, signed loop-count words and
source spans survive. Native resource clamping, definition lookup, duplicate
tech filtering, AI runtime reconstruction and post-load state mutation are not
performed by these wire decoders. A nonempty v12 named-state list is explicitly
unsupported rather than skipped; all328 original records have count0. Actor
payload accepts only its proven component/property subset and rejects other
unbounded payloads. Unknown modified chunks remain opaque with an exact reason.
No original assets, game installation, saves, or older release caches changed.

Evidence SHA256:
 native-teams.json
 f8a86f90bc0e4fb0d3023b4bd6a87f44e297e0c9630ac7ea8f5f47e51b91c8a6
 native-kingdoms.json
 f92cf844cba27c76b9f2bd0495a67b0db7ea5f9706f4d620d2faa0dba80711dc
 native-leaves.json
 97b664dac2f06f40e5588a90ab1efe9d66e3dd39254f08dcd52b0638c8c830c2
 Original executable
 5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26
 v14 executable
 3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1
 Relevant original/v14 instruction ranges are checked for byte identity.

Validation:
 python research/phase6-kingdoms/native_teams.py
 python research/phase6-kingdoms/native_kingdoms.py
 python research/phase6-kingdoms/native_leaves.py
 $env:AA_GAME_ROOT="E:\Games\Axis and Alies"
 cargo test -p aa-mapworld -- --include-ignored --nocapture
 cargo clippy -p aa-mapworld --all-targets --no-deps -- -D warnings
 Current complete mapworld suite:27 tests pass. Team/Kingdom source boundaries,
 all owner bindings, real minor kingdom, native owner flag gate and23 leaf
 fixtures pass. Rust corpus audit is examples/audit_kingdoms.rs; read-only.
