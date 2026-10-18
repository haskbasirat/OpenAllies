Saved Mind action integration, 2026-10-16
=======================================

Scope
-----
Bind original Mind virtual+B4 (Element530510, Regiment521E10, Aircraft518720)
inside saved Element5705A0 -> Actor4D1350 rebuild, using actual restored owned
components. The existing aa-mind/action_mask native port retains call/bit order.
The production bridge moves the existing RestoredMindParts Mind core and state
map into Arena; it never creates a substitute Mind, clones gameplay state, runs
a constructor again, or allocates a new command identity. The same parts retain
installed IDs, path, commands, and precision. Success and callback errors return
the original core and states to the original component slot. A logical active
Mind ownership marker keeps component-presence queries true during the borrow.

Bindings
--------
Actor+F0/F4: actual shell words, separate from component state bits.
Element+10: actual restored organization identity, including the loading owner
held outside the ID registry. Other IDs use the generation-checked registry.
Performer abilities: actual saved Ability arena handles, enabled flags, and
validated ActorLocalAbility/global definition identity. Canonical ActorData
names shadowed by a Thing translation are explicitly refused here.
Properties+1C: owning actor handle, validated Properties owner and modifierflags.
Organization status: actual words+28/+2C of the same owned Mind, actual current
embedded state IDs and stack, actual Organization+5C/+60 and definition+308.
MobileDeployment: exact source forward+534/reverse+548 pointers and threshold
from the source producer, actual Body health/max and Properties flags; native
575BB0/575C00/575FB0 reuse the separately proved aa-actors port. The source
arena is cached per x87 precision, including producer errors.
Recruit582C40: new source-backed adapter from the recruit-actions agent. Saved
enable bytes remain zero unless actually changed; a real nonempty list is not
discarded. Reaching unbound5827B0 eligibility is an explicit error.
Kingdom+218: actual owned RecruitItemTable.singles.len(), not a duplicate scalar
in SpecialShell.words (214 is the vector pointer and218 its count).

RULES and load mode
-------------------
GWorld4FC600 reads a separate RULES stream; MAP_HEADER is not substituted.
WorldInventory now retains the original RULES bytes and source hash. The same
512420 decoder has a strict standalone wrapper. Definitions populate every
WorldRule before serialized same-type entries overwrite it in order. The
misleading Rule.enabled byte is actually LOCKED and never gates512200.
FogOfWar's effective definition default is EXPLORED1, not the earlier SVar2.
The settings subdirectory contains the native merge/clamp/setter evidence.

Catalog.native_load_mode is optional, defaultNone. Attempt::load and legacy
load_with_mode preserveNone. The new load_with_native_mode requires the caller
to explicitly supply MapResource+A8 (independent of diagnostic RestoreMode).
The value is retained through Attempt/Staged. Missing mode is an explicit error
when mode-sensitive settings are read. The source test states mode1 explicitly;
no skirmish UI mode selection, editor session or full load lifecycle is implied.

Evidence
--------
Existing phase7-mind/native-action-mask.json covers219 pure Mind mask cases.
native_saved_masks.py executes unchanged original/v14 Element530510 on two
observed tutorial01 actors at PC24/53/64 (12 executions,372 instruction sites).
Original bit stores and Performer capability loops execute. 4CF110 executes
against actual optional Properties pointer layout. The virtual action query
reads the live native F0/F4 mask. Mobile predicate returns are explicit observed
leaves, separately proved by552 native MobileDeployment cases; this oracle does
not claim a new complete native MobileDeployment/loader integration. Pagezero
is unmapped. No game process or release file is modified.

Rust source tests compare those native outputs, preserve exact Mind/state/command
values and stack/command allocation addresses, and exercise an absent required
Element failure after the first action store. Missing native load mode must
error while returning the same owned core. Other tests prove actual tutorial02
Recruit list handling and source settings over all80 original RULES chunks.
The historical Element prefix fixture is retained, but its test now compares
only the membership stores it proved; later Recruit/Research behavior is not
credited to that earlier stopped oracle.

Limits and next stops
---------------------
This is saved restoration glue, not a playable skirmish or multiplayer release.
The current source tutorial01 mode1 load stops at Organization578A30 source
class-reference binding; tutorial02 reaches Research583FC0. Subsequent action
contributors, owner recruitment predicates, Economy refresh570020, SAI observer
phase, and non-null presentation publication remain named errors where reached.
No full80 record census was run by this subtask; parent owns combined-source
census and publication acceptance. The source settings test is not that census.
Source Actor fields108/118 are constructor-initialized. The projection reads
them eagerly; corrupt/absent initialized fields and mismatched identity errors
are conservative unsupported cases, not claims of native exception timing.
Ability capability projection materializes every enabled descriptor before
the native any-match scan; a malformed later handle can therefore fail earlier.
The parity scope is valid constructed source/Ability storage.
Regiment/Aircraft dispatch compile against existing proven ports; current new
source-observed12-case oracle covers Element only.

Backup
------
backups/source-before-saved-mind-actions-20261016.zip preserves the initial
assigned adapter/test/lib source. Earlier complete source snapshots remain
under backups. Development08 release and all original game assets are untouched.
