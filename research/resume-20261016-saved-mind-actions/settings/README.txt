Saved Mind virtual+B4 settings and identity audit -- 2026-10-16
Native evidence, bounded production helper, and scoped Rust validation.
Production changes: crates/aa-runtime/src/saved_action_settings.rs and its
module declaration only. Other integration changes belong to the lead agent.

RESULT
5 release Rust tests pass, including the public helper functions over all80
original maps with the actual original/v14 source catalog. Native executions:
93 generated FogOfWar merge/reset/clamp cases,80 original RULES payloads,
9 GameType CString cases, and6 fixed-SVar registration/setter sequences
(PC24/53/64).
The real map FogOfWar values are0 for30 maps and1 for50 maps. Effective
MoraleRetreatPenalty is0.0 for all80. This is settings restoration evidence,
not full-map/gameplay completion.

EXACT BINDINGS
World.setting_cache108():
  Native 53056F loads [B39AB0]; 530575 loads vector[0x108];
  53057B compares the variable's first DWORD with zero.
  This is FogOfWar, native SVar index 0x42 (66), INTEGER type 3.
  Return integer != 0: both 1 and 2 mean true. It is not a Bool declaration.
  5065D0 registration window 507CEF..507D26 pushes name A2FCE8, default 2,
  minimum 0, maximum 2, flags argument 0, then bounded-int constructor
  401940 and Add 4B3470 with index 0x42. Constructor401940 ORs flag0x20,
  so stored flags are 0x20, not zero.

World.setting_cached8():
  Native 521F13 loads [B39AB0]; 521F1F loads vector[0xD8];
  521F25 FLD reads the variable's first DWORD as binary32.
  This is MoraleRetreatPenalty, native SVar index 0x36 (54), FLOAT type 2.
  5065D0 registration window 5079D1..507A07 pushes name A2FEA8,
  default 0x40A00000 = 5.0, flags 0, then constructor401710 and
  Add4B3470 with index0x36. TGI spelling is fixed; do not treat it as int.

SOURCE DEFAULTS AND ACTIVE WORLD
The actual source declarations read in this checkout are:
  Game/SVars.tgi:140       fixed MoraleRetreatPenalty = 5
  Game/SVars_AA.tgi:27     fixed MoraleRetreatPenalty = 0
  Game/SVars.tgi:236       int FogOfWar = 2
SVars_AA.tgi resolves to runtime-expanded-v14/Data/Game/SVars_AA.tgi.
No v14 loose SVars.tgi exists; it resolves to the original archive member
(the recovered original copy was inspected and hashed here).
The source-default penalty is therefore 0.0 (bits00000000). The initial TGI
FogOfWar value is2, but WorldRule definition completion changes it to1 before
the source-default snapshot described below. Do not call2 the final snapshot.
The existing phase7-definitions/native-settings-order.json
proves wildcard lexical file order: SVars.tgi before SVars_AA.tgi. Its native
assignment execution covers Bool only; it is NOT numeric assignment proof.
Numeric assignment is inspected statically here: Settings4B3C10 type4 fixed
uses4B2F80 -> type2 dispatch40E0A0; type1/2 integer uses4B3090 ->40E110.
These flags allow changes; FogOfWar clamps to its registered signed [0,2]
range, while this MoraleRetreatPenalty has no bound flag.

B39AB0 is SettingsB39AA8 +8, the indexed Var-pointer array, not a separate
lazy SVar cache. Neither action-mask routine performs a named lookup or
caches its scalar value. Both read the current variable at their call point.
Startup48DB83 sets ECX=B39AA8 and calls5065D0 at48DB8B to register defaults;
SVars nodes load into B39AA8. At4902A2,474C50 completes WorldRule defaults:
for enum rules it requires int SVar type3, sets bounds0..enumCount-1, and
applies definition+40 via40E110. FogOfWar becomes EXPLORED=1 here.
4902A7 passes B39AA8, sets ECX=B39AD0, then
calls4B3580 at4902B1 to copy the resolved source defaults. GWorld::Init
4F281C passes B39AD0, sets ECX=B39AA8 and calls4B3580 at4F2826 to reset
active SVars from the source defaults for each new world.

DO NOT equate source defaults with every restored map's effective settings.
4FC600 reads its separate RULES chunk through WorldRules::Load512420,
then applies WorldRules::UpdateSVars512200 before terrain and actor records.
MAP_HEADER/creator rules are not the input for this operation.
512200 sends int/enum rules to512160 and float rules to5120F0; these mutate
the active SVar. FogOfWar is an authored WorldRule:
  Game/world_rules_aa.tgi:17..35, enum NONE / EXPLORED / FULL,
  default EXPLORED (ordinal1), not SVar default2.
The apparent FogOfWar entry in world_rules.tgi is inside a block comment.
The raw rule byte named 'enabled' in the earlier header decoder is LOCKED:
5114D0 labels record+4 as '_wr.locked'. It is not an activation switch.
512420 first calls511D90 ->511CE0, filling every registered rule in native
registry order with its definition+40 default and locked=0. Each serialized
record resolves its IDS through470470; known records whose serialized kind
matches definition+20 replace that indexed slot (definition+0C). Unknown or
changed-type records log and leave the default/previous value intact. Later
duplicates overwrite earlier accepted records. Locked does not gate this.
For MapResource loadmode2,4FC600 then invokes511640: definitions whose
always_editable byte+24 is true reset value to definition+40 and locked=0.
Mode1/4 skip that reset. Ordinary FogOfWar inherits always_editable=false.
512200 iterates all effective rules regardless of locked and publishes the
FogOfWar integer through512160, clamping to the enum-derived bounds[0,2].
The new helper implements this FogOfWar projection and refuses unknown load
modes, missing/ambiguous RULES, changed bytes, unsupported definition shapes,
unknown nonempty GameType, and unsupported source declarations.

CSTRING LOOKUP BOUNDARIES
Native rule/GameType lookup uses the UTF16 prefix before the first NUL,
case-insensitively and without whitespace trimming. The helper decodes only
that prefix; malformed UTF16 after the NUL does not invalidate a valid key.
FogOfWar followed by NUL and any tail still matches FogOfWar. A rule key
beginning with NUL is unknown and skipped. GameType differs: serialized
length zero bypasses lookup and stores null, but a nonempty serialized string
beginning with NUL performs lookup for the empty key, then reaches512360's
unknown-reference exception branch. These are separate regression cases.

READ ORDER
Element530510 updates bit5 and, only when attached, bit2 BEFORE looking at
FogOfWar. It reads FogOfWar only when Thing flag29 is clear AND Element+10
is nonnull, then stores bit9. A missing setting should not be read eagerly
on a branch native code skips, and errors must retain preceding writes.
It unconditionally dereferences actor+80 at53055B; an Element Mind requires
an actual Element component even if its organization link is null.

Regiment521E10 always calls Organization578140 after its initial current-
state read, even when the head is Rout. The rout/status0 path clears bits0..12
and never reads MoraleRetreatPenalty. The ordinary path reads the penalty
only after actor action bit7 is true and the live Engage state's52A000
predicate is true. Arithmetic is FLD(setting), FADD Organization+B8,
FADD binary32 constant1.0, FCOMP Organization+5C. The setting is raw f32;
the additions are x87 in the supplied precision, not reassociated or rounded
to f32 between each addition. Existing aa-mind action_mask does this with Binary.

MOVING RestoredMindParts INTO AN Arena AND BACK
- Move the authoritative core and full state map; never use a clone or a new
  constructor to manufacture an arena. Do not repeat RNG-consuming construction,
  bind/install, InitLoad, reset/default activation, or scheduler registration.
- Keep installed.mind_id and every existing StateId unchanged. Every state's
  parent must still identify that same Mind. Mind.actor is the real GTID,
  not the MindId, a serialized command ID, or a map ordinal.
- For Regiment comparisons obtain Rout from installed.state_bindings[0x20C]
  and Engage from[0x1CC]. Native pointer identity determines these comparisons.
  529F70 returns stack head (including a null head value), not the first nonnull
  state. 52A000 rereads that head through the state's parent. Status alone or
  presence deeper in the stack is insufficient. Preserve null nodes/order.
- Validate missing IDs, ownership/parent mismatches, duplicate IDs, kind and
  binding presence before ownership transfer. Restore the same core/state map
  on success AND returned error. Previous writes from a native callback stay
  committed; do not restore a snapshot or roll back actor action bits.
- Callbacks deliberately reread Mind.actor/current stack after external leaves.
  Existing native-action-mask fixtures include owner changes at player/component
  callbacks and stack/word118 changes, so one precomputed actor snapshot is wrong.
- Do not disturb installed.update_head, installed other fields, element_path,
  or command storage. Moving the core transfers its command ID references but
  neither creates nor releases command ownership. Restored command identities
  must remain unique and equal the pending/current references.
- Existing actor_world::stage_restored_mind truly moves all parts. At audit
  time Catalog::load_saved_mind used parts.mind.clone() for its temporary
  saved-load Arena. That is a separate existing site; this audit did not edit it.
  For a new mutable+B4 bridge, retaining such a clone violates single ownership.
- Arena is currently a public pair of BTreeMaps. A move-out bridge must ensure
  same-Mind callbacks see the in-flight arena rather than trying to read an
  empty RestoredMindParts slot. Unexpected removed/reparented IDs must be explicit
  errors with the still-owned partial state returned, not dropped silently.

EXISTING EXECUTION EVIDENCE READ (not rerun)
research/phase7-mind/native_action_mask.py and native-action-mask.json;
crates/aa-mind/tests/action_mask_native.rs. Settings in these fixtures are
synthetic inputs named setting108/settingd8. They prove contributor behavior,
not the source-name/global/default binding recovered in this audit.
research/phase7-mind-world provides the live Arena/ActorAccess lifecycle boundary;
its commands, queues and states should remain canonical during integration.

ARTIFACTS
settings-evidence.json: original/v14 PE SHA256, pinned matching instruction
windows, native string bytes, source paths/hashes/relevant lines, fixture hashes.
audit_settings.py regenerates that static evidence; it never executes native
instructions or modifies production files.
native_fog_rules.py -> native-fog-rules.json:93 generated cases plus80 original
RULES chunks through original512420,511D90,511CE0,511640,512200,512160.
The source-bound FogOfWar descriptor and one-bucket registry are supplied;
other definitions resolve null. Actual470470/4915F0/405030 and native wide
case-insensitive comparison execute. Native512360/48D500 also execute against
a supplied NORMAL GameType descriptor;9 dedicated GameType cases cover empty,
NUL-prefix, mixed case, leading whitespace, unknown and invalid-tail inputs.
The41D290 stream string seam preserves the complete serialized UTF16 units,
including embedded NUL/tail units. CRT __getptd9738A9 is an explicit TLS seam:
it returns an allocated context whose+64 locale pointer equals[B35444]. The
native hash/comparison instructions execute; full CRT TLS setup does not.
Unknown GameType stops at native41DFB0 exception entry; SEH unwinding is not
executed. The producer's older summary docstring is broader than its final
lookup implementation; the explicit boundaries in this paragraph apply.
Rust's real source test additionally checks the actual GameType registry
(NORMAL in80maps).
native_morale_settings.py -> native-morale-settings.json:6 sequences through
original constructor401710,Add4B3470,setter4B2F80/40E0A0. Binary32 setter
arguments are explicit inputs; full numeric TGI LoadData is inspected statically.
Both oracle harnesses guard pagezero reads/writes against non-FS access, and
pin every visited native instruction to unchanged original/v14 bytes.
rust-tests.log:5 passed,0 failed,0 ignored,including real-source80map check.

REPRODUCE (PowerShell, from rust-rebuild)
  python research/resume-20261016-saved-mind-actions/settings/audit_settings.py
  python research/resume-20261016-saved-mind-actions/settings/native_fog_rules.py
  python research/resume-20261016-saved-mind-actions/settings/native_morale_settings.py
  $env:CARGO_TARGET_DIR='target/s-settings'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --release --locked --offline -j 4 saved_action_settings --lib -- --include-ignored

LIMITS
WorldRule LoadData4905E0 source interpretation is static evidence and checked
against actual source via the existing typed loader; not a new full-loader
oracle. No active live-match setting mutation, whole GWorld/Session execution,
gameplay parity, full Mind behavior, or general arbitrary WorldRule support is
claimed. MoraleRetreatPenalty refuses a newly registered WorldRule targeting
that name until its definition/default/override semantics are supported.
