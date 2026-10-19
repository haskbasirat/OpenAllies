KKC_GActorCommand::GuessCommandData 0x4DA570 (Source\Sim\GActorCommands.cpp)

What it is
----------
A right-click command built by the UI (0x620760: ctor 0x4D9C80, no explicit
command, flag bit6 clear) or any decree without +08 asks 0x4DA570 which of the
39 ActorCommand definitions it means for one actor.
__thiscall(this=command, actor, bool argument2) returns the definition
pointer or 0. Callers (xrefs): 0x4CF430 (actor vt+150 of GActor A273F0, GGroup
A29490 and GKingdom A299D8; argument2=0), 0x61F210 (first selected actor,
diagnostic name only; argument2=0), 0x4ED360 (each selected actor, argument2 =
selection head actor != tail actor), 0x4DE740 (positive group path, minimum
def+0C over members) and 0x61EEF0 (UI send diagnostic).

Native algorithm (proved instruction by instruction, 326/326 instructions of
0x4DA570 executed by the oracle)
* mask = 0x10 if flags bit2 (position), |0x20 if bit3 (auxiliary).
* flags bit0: target = B364C0 lookup of +10 (generation word table+4+2*slot,
  object table+20004+4*slot). Unresolved: 404F60(B38B70, ".\Source\Sim\
  GActorCommands.cpp", 0x12A, "KKC_GActorCommand::GuessCommandData",
  "GuessCommandData - target actor %d is invalid", handle) and return 0
  before any variable is read. Resolved: K = target vt+108 (Actor/Group/
  Kingdom 4D1E80 = [+E8]; Object family 4C9FA0 = 0); mask |= 0x100;
  actor vt+140(K) -> 0x2000 (4E52A0: vt+144 == 3); actor vt+138(K) -> 0x1000
  (Actor 4D1EB0: [+E8] == K or vt+144 == 1; Kingdom 4E5250: this == K or
  vt+144 == 1). Target +7C Organization: 579FD0 (any non-null member +9C)
  -> 0x200, 579F70 (any non-null member 4D1490: Body +10 < +14 ordered and
  Actor+100 bit21 clear) -> 0x800. Without Organization: +9C -> 0x200 and
  then Actor+100 & 0x200000 -> 0x400; Body +60: binary32 +10 < +14
  (fcomp/test ah,5/jp: NaN and equal are false) -> 0x800.
* UVar SwapRightClickWithAltRightClick: 4BE130 looks the kind1 record up in
  B39938 ONCE (static guard B3CB9C bit0, record pointer B3CB98); 4BE150 reads
  its byte on EVERY call. Alt = flag bit10 XOR swap (4DA77C not al).
  Only when Alt: HotkeyAltRightClickCommand kind4 record looked up once
  (B3CB9C bit1, pointer B3CB94); its +10 UTF-16 text is read every call and
  looked up by 497220 in the ActorCommand enum B3CBA0+10 (hash 405030 folds
  a..z, compare 96CAD6 = CRT wcsicmp); not found = -1.
* for i in 0..0x27: actor vt+34(i) (4D1F90 F0/F4 action bit) must be true;
  hot >= 0 skips every i != hot; def = [[B36530]+298][i]; argument2 skips
  def+3C (non_group_command). matched = def+2C & mask.
  matched bits 4..7 (location branch): needs def+40 (can_hot_command_
  location) or i == hot, then matched == def+2C & FFFF00FF returns def.
  Otherwise (actor branch): needs def+3F (can_hot_command_actor) or i == hot;
  type def+0C != 4: matched == def+2C & FFFFFF0F returns def; type 4: list =
  4D44E0(actor), target re-resolved from +10 and target+4 read (null ->
  native fault), first list value with value+4E0 == target+4 and
  ((mask|0x50000) & wanted) == wanted completes the command (flags |= 2,
  +14 = value) and returns def.
  Every path that did not return: if K != 0 and actor vt+13C(K) (vt+144 == 2)
  and (matched|0x1000 == mask or matched|0x2000 == mask) returns def.
* 4D44E0(object, &list): Builder +A8 -> prepend every [[object+4]+504]
  value (no dedup); Organization +7C members filtered by [[member+80]+1C]
  (4364C0 filter byte 1, Element null -> fault) recurse; Settlement +98:
  +18/+1C children recurse, then +14 parent; recursion results are merged by
  prepending values not yet present. Unbounded on cycles (stack overflow).

Field proofs
* def+0C = type, def+18 = component, def+2C = behavior flags: 4740F0 stores
  arg1 at +0C, arg2 at +18, arg3 at +2C; 475870 calls 4740F0(type, component,
  flags) e.g. (0, 4, 0x2130) for kill; 4B4E60 then rewrites +0C with the
  registration index (warning when different), so [+298][i]+0C == i. These
  are aa_rules ActorCommandDefinition.mind_type / component_slot /
  behavior_flags (the fixture checks all 39).
* def+3C/+3F/+40 = actor_command_ui non_group_command / can_hot_command_actor
  / can_hot_command_location (48CF0A..48CF97 store order, research
  skirmish-20261016-player-orders/command-ui). The live binding re-runs the
  same actor_command_ui::load_fields in the supplied Bank and the test
  compares all 39 with that native fixture.
* Actor+100 = Components.state.low (StateBits::construction is bit21).
* Enum B3CBA0 (built by the ORIGINAL 4DA9D0(1)/4DA2F0/4DA1E0/496F60 in the
  oracle) holds exactly the 39 aa_rules PREDEFINED_ACTOR_COMMANDS
  (value = mind_type, name = id, extra 0) in order.
* 61E220 (actor decree) never stores the guessed definition (no 4D9A90):
  a permitted default actor command reaches 4D9B70 with command+08 null
  ([null+18]) - a native fault.

Oracle
------
native_guess_4da570.py (Unicorn, research/phase8-ai/native_machine.py) runs
the original code in AA.exe and in the pinned v14 image at PC24/PC53/PC64;
all six runs are identical and every executed instruction is byte-identical
in both images. 757 cases (420 random, 167 construct/type-4, 136 hotkey x
flag combinations over the actual table, 24 Settlement merges, 8 two-call
cache rows, missing-variable rows); 254 return a definition (indexes 0..38
spread), 41 complete the command (flags|=2, +14), 151 log an invalid
target, 176 record the native 4D44E0 list, 35 fault natively ([null+4] at
4DA8EB x18, Element-less filtered member 4364EC x13, missing swap record
4BE152, missing hotkey record 4DA7CF, null Settlement child 4D4525, a
recursion cycle (UcError, stack)). 95 direct 497220 lookups (case variants,
trailing/leading spaces, embedded NUL, non-ASCII). Definitions are either
the actual 39 (type/flags + native controls) or synthetic (random types
including extra type-4 entries, random flags/bytes; +3D/+3E/+41 random
noise). Seams: 6F7810/6F7430 heap, 401B00/4012C0 CString, 9738A9 _getptd
(CRT "C" locale = initial locinfo B353F0), 4B2E60 records, 404F60 log;
actor/target vtables: +34/+108/+138/+13C/+140 Python seams returning raw
EAX (0x100 / FFFFFF00 = false via AL), every other slot traps.
Coverage: 4DA570 326/326; 4D44E0 258 (unexecuted: the entry clear of a
non-empty output list and the post-loop free of a non-empty member list,
both unreachable from 4DA570, plus one alignment nop); 579FD0, 579F70,
4364C0, 4D1490, 497220, 405030, 96CAD6 (C-locale path) and the enum
constructor fully on their reached paths.

  python native_guess_4da570.py --write   (refuses to overwrite)
  python native_guess_4da570.py           (replay; byte-identical, see
                                           native-replay01.log)

Caveats: the CRT locale is the C locale (wcsicmp folds only A..Z); a game
that called setlocale with another LC_CTYPE could fold non-ASCII
differently. Laboratory words are explicit oracle inputs, not mission data.

Rust (crates/aa-runtime/src/player_order_guess.rs, module
player_order_world::guess)
----
Kernel (generic, proved by the fixture):
  pub trait World { type Object; type Organization; type Kingdom;
    type Definition; type Error: From<Required>; resolve_10, kingdom_108,
    hostile_140, friendly_138, neutral_13c, organization_7c, members_28,
    element_flag_1c, construction_9c, state_100, body_60,
    swap_right_click_4be150, hotkey_alt_right_click, allowed_34,
    definition, builder_a8, builder_targets_504, settlement_98,
    definition_4, required_object_4e0 }
  pub fn guess_command_data<W: World + ?Sized>(world: &mut W,
    command: CommandFields { flags_0c, target_10 }, actor: W::Object,
    non_group_excluded: bool) -> Result<Outcome<W::Definition>, W::Error>
  Outcome { guessed: Option<Guessed { index, definition }>,
    completed_object_14: Option<D>, invalid_target_logged: Option<u32> }
    .apply(&mut flags, &mut object14)
  pub fn candidates_4d44e0, organization_construction_579fd0,
    organization_damaged_579f70, damaged_4d1490, hot_command_497220.
Live binding (guess::Live over Catalog + Banks + ActorAccess + Globals):
  B364C0 -> ActorAccess identities (generation-checked IdentityRegistry);
  vt+108 -> Actor+E8 (class 3/4 only); vt+138/13C/140 -> the shared
  relations::borrowed 4CF040/4E52D0/4E10F0 kernels over the live Kingdom/
  Team derived SpecialShell (B28158/B2815C null paths required); +7C/+80/
  +9C/+A8/+60 -> component slots Organization/Element/Construction/Builder/
  Body; +100 -> Components.state.low; vt+34 -> F0/F4 (allows_with_bank);
  definitions -> PREDEFINED_ACTOR_COMMANDS + controls bound in the same
  Bank; +504 -> Catalog native_builder_data; UVars -> two new default
  methods of player_order_world::Globals (required until bound).
Public entry points: guess::guess_plain_in_bank, guess::guess_wire_in_bank
  (UI CommandWire), player_order_world::authorize_selected_command_in_bank
  (4CF430 over a caller-owned PlainCommand, for UIWorldState 436950).

Decree path now
* 4CF430 glue: explicit -> def+0C; default -> 4DA570(cmd, actor, false),
  null -> permission false (the former "4CF430 unresolved4DA570" branch),
  then the existing 4CE680 kernel. authorize_selected/actor (borrowed
  commands) run it on a probe and refuse if a completion would be lost.
* 61E220 permit (87): 4CF430 on the same arena command (completion persists).
* 61F210 inspect_selected (86): 4DA570(cmd, first actor, false) on the arena
  command, then the diagnostics sink (unchanged trait).
* 4ED360 null group: per actor 4DA570(cmd, actor, head != tail); null skips
  the actor; otherwise 4D9A90(def) (token + bit6), 4CF430/4CE680, 4D9B60,
  4D9A90(0).
Resolved errors: "4DA570 actual default command selection", "4DA570 actual
default actor command selection", "4DA570 default selected-command source",
"61F210 first selected actor's actual4DA570 default command", and the
permission_native unresolved branch is now reached with the real guess.

Remaining precise gaps (each a required(...) error at the reached leaf)
* UVars SwapRightClickWithAltRightClick / HotkeyAltRightClickCommand
  (4BE130/4BE150, 4B2E60, registry B39938 with user-profile overrides): no
  owner on the order path; Globals defaults are required errors, so every
  default command stops there until a host overrides them (an invalid
  target still logs first).
* Type-4 construct: ActorData+4E0 needs the completed NativeSourceRequirements
  owner (4DA8F4), the 4DA994 command+14 Object-definition token (B39F58
  registry) has no portable owner, Settlement+14/+18 has no Rust component.
* Non-ordinary targets/actors (Object family, GGroup class 8) and
  B28158/B2815C null-relation globals.
* 61E220 delivering a guessed default command: native [null+18] at 4D9B70.
* 4ED360 bounded entry: guessed types other than move5/attack0 hit the same
  pre-existing bound as explicit commands.
* 4ED360 positive groups/formations (and 4DE740 group guess), 4D1900/4DFF30
  live group preparation, 61F210/4D9A40 diagnostic sinks: unchanged.

Tests (CARGO_TARGET_DIR=target/agent-orders-guess, release, AA_GAME_ROOT set)
* aa-runtime --lib player_order: 13/13 pass (rust-tests.log), including
  guess::tests native_4da570_rows_replay (all 757 rows + 8 second calls,
  every event sequence, result, flags, +14, log text, cached lookups and
  every native 4D44E0 list), native_enum_b3cba0_and_497220_lookups,
  actual_definition_fields_match_registration_and_controls,
  live_definitions_bind_actual_controls (real v14 sources, moved-Bank style
  controls binding for all 39) and the existing permission/selection rows.
* tests: actor_command_ui 2/2, local_order_host 1/1, player_order_decree
  2/2, saved_command_bindings 6/6, player_order_binding 1/2 with
  --include-ignored. The failing
  actual_moved_bank_source_selection_and_reentrant_mind_prefix_keep_same_owners
  stops at tests/player_order_binding.rs:334 (expects
  resolve_thing_reference ThingCheck::B0 to refuse an Actor); that function
  is untouched by this change (it already admits classes 2/3/4/6/7/9), so
  the failure predates this work.
