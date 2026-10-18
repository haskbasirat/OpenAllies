PHASE8 SAVED COMPONENTS: ORGANIZATION (SLOT 7) AND TRANSPORT (SLOT 24)
2026-10-15

Goal: load the saved Organization and Transport components exactly as AA.exe
does, so the regiments, herds/flocks and water transports of the original maps
stop blocking restoration ("unported saved component slot 7/24": 1067
Organization and 47 Transport records in the phase-8 all-maps inventory).

Status: both loaders are ported and oracle-proven (Unicorn runs of the original
x86, every seam recorded). They are NOT wired into restoration yet: that is the
restoration agent's dispatch (aa-mapworld / aa-runtime), see WIRING below and
requests-to-lead.md.

---------------------------------------------------------------------------
FILES
---------------------------------------------------------------------------
crates/aa-actors/src/organization_state.rs   Organization Load 57D130 + shared
    stream Cursor, LoadError/Message, Definitions trait, layout Load 4C3180
crates/aa-actors/src/transport_state.rs      Transport ctor 58B9A0 + Load 58C810
crates/aa-actors/tests/native_organization_saved.rs   replays the fixture (2 tests)
crates/aa-actors/tests/native_transport_saved.rs      replays the fixture (1 test)
research/phase8-saved-components/
  native_machine.py        standalone Unicorn machine (page-0 null guard, seams)
  native_organization.py   oracle -> native-organization.json (205 cases)
  native_transport.py      oracle -> native-transport.json (290 cases, 47 real)
  real-payloads.json       every slot-7/24 payload prefix of the 80 original maps
  payload_dump/            extraction tool (own workspace; aa-mapworld decoders)
  disasm.py, strings_at.py helper listings; *.asm.txt the three main functions
  findings.json, requests-to-lead.md

---------------------------------------------------------------------------
NATIVE FUNCTIONS
---------------------------------------------------------------------------
Organization (0xDC bytes, vtable A3D434; component factory slot 7):
  ctor 579BD0 (already ported: ground_states::OrganizationState::new, phase7
  oracle; re-checked here before every Load case)
  Load 57D130 (vtable +2C; Ghidra had no function there, decoded from the
  instructions; writer 579170):
    50BCD0 Updatable (version <= 1, id, GWorld+284 table, 858AD0 on bad or
      duplicate id) / 50B250
    8589B0 version <= 10 (41DFB0 "Bad data version")
    [+10] virtual +8 = 4C3180 layout Load: version 0, 4C3050 LayoutData
      reference (registry B3AB00 via 463FA0; empty -> null -> fault 4C31D0;
      unknown -> 41DFB0 "Unknown IDS"), 430240 parameter resize, saved count
      (858AD0 "Data changed number of slots"), 4C2E60 ActorData references
      (registry B39F58 class 0x2B via 4BDBA0)
    loaded LayoutData != ActorData+2F0 default: same parameter count ->
      4C1DB0 replacement, else 858A00 "Changing number of slots for layout
      data (%s -> %s)"
    430240 members +28 / recruit items +34, 45F180 +50, 45D8A0 +78 (+7C =
      slot count), 578300 maximum morale (45D580 ActorData+2C0, 4CF110 +
      501C90 owner modifier, ratio rescale of +5C), +D8 = FLT_MAX
    4C2910 VerifyDefaultLayoutRestrictions (locked / required / defaulted /
      45EEF0 property checks, 404F30 logs) -> 858A00 "Organization layout no
      longer adheres to default layout"
    saved slot count (858A00 "Layout size changed from %d -> %d")
    per slot: index word; index >= 0 -> 4FC200 (4FAE80 NESTED GObject record,
      null -> 858A00 "Failed to load element for organization ..."; IsGActor
      assert dialog 855D30 latched by B3CC47), ActorData+174 bit 8 (858AD0
      "has no element component"), 57CBA0 add member at that index
    57CBA0: member store, 5705A0, 561760 (Body), 4CED10/4CEBB0 (no
      Experience), 5788B0, +D8 min of member Mover+28 speeds (Mind +118), primary
      +44 / state 0x22 (4D1080), 579A00 leader when primary or no leader,
      524B80 (Mind), 57C800, 4D4CF0, 4CF0F0, 4D1350, +64 morale-line count,
      5784C0, ActorData+2D8 list -> 4CEF10(...,8,member), 5147C0, state 0x25
    versions 2..6: recruit items by Kingdom index (owner virtual +108, +218
      count, 4E13F0; missing -> 5A1040 map warning)
    version >= 7: recruit items by ActorData reference (4E2A60 on Kingdom
      +1FC; 858A00 not found / not paired; 4F0110 assertions lines 0x234 and
      0x240; 4E1200 pairing)
    leader index (858AD0 "Bad leader position"), 59E300 morale = saved * +60,
    4C2D90 formations +70 (and +74 for version >= 3; 858AD0 "Bad desired
    formation index"), version 0 word, +78 pairs, +50 words (v >= 1),
    +84/+8C/+8D and 4C0BA0 damage record (4C09C0 KKC_DamageType via 48D500;
    +A4/+A8 by GWORLD version 0xF/0x12/0x14) (v >= 4), +88 (v >= 5),
    +B4 (v >= 6), +BC/+C0 (v >= 8), +D0 (v >= 9), +D4 (v >= 10)
    +B4 cleared when greater than GWorld+E4; +74 = 0; 57BFF0 set formation
    (4D49B0 group 4, 57BF50 with 4CEF80/4CEF70/4CEF10/57BEF0/4D1900/4B2830,
    577F30 for +8C/+8D/+8E)
Transport (0x60 bytes, vtable A40100; slot 24): ctor 58B9A0, Load 58C810
  (Ghidra had no function there), writer 58B380. No Updatable record. +8/+18
  cost/upkeep ResourceVectors (4CD440; Load 4CD3F0 -> 4CD0F0), +28 ActorData
  (4C2E60), flag byte -> +2C reference-counted KKC_OrganizationLayout (4C3050,
  4C2110, virtual Load 4C3180), +30 8 bytes, +38/+39, +3C, +40, version-0
  discarded word list, +44 (v >= 4), +48, +4C (+50 8 bytes when nonzero)
  (v >= 3), +58 string (v >= 5), +5C (v >= 6).

---------------------------------------------------------------------------
ORACLES (PROVEN)
---------------------------------------------------------------------------
PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-saved-components/native_organization.py
  python research/phase8-saved-components/native_transport.py
Both run the ORIGINAL constructor then the ORIGINAL Load on the same object
(original image; every executed instruction byte-identical in the v14 exe).
Page-0 accesses fault unless FS-prefixed (null_guard); every allocation goes
through the 6F7810 seam (0xA5 fill, nothing zeroed by the harness). Throws
(41DFB0, 858A00, 858AD0, 4F0110) stop the run and record the message; the
partial object state is then compared too.

native-organization.json: 205 cases. Versions 0..10 x PC24/53/64; GWORLD
versions 0xE..0x1C; Updatable variants; every layout/verify/member/recruit/
leader/formation/state-group/timer/damage-type branch above; precision cases
where PC24 differs from PC53/PC64 (subnormal 59E300 products, double
rounding); a truncation sweep. Outcomes: 123 loaded, 8 Fatal, 11 Warning,
7 Exception, 2 Assertion, 3 native null dereference, 51 truncated.
native-transport.json: 290 cases. Versions 0..6 x 3 precisions x resource
counts, carried layouts (inner LayoutData different from the outer one, count
mismatch, unknown/empty references -> faults 4C2141/4C31D0), legacy lists,
ResourceVector versions/counts, truncation sweep, and all 47 Transport
payloads of the 80 shipped maps (versions 4/5/6, all load). Outcomes: 185
loaded, 1 Fatal, 6 Exception, 2 null dereference, 96 truncated.

Rust (CARGO_TARGET_DIR=target/s-saved):
  cargo test -p aa-actors --release --locked --offline -j 4 --test native_organization_saved --test native_transport_saved
  -> 3 passed. Every case: outcome (error kind, format, arguments, offset),
  bytes consumed, the complete ordered callback sequence, and the full state
  (including partial state after errors) equal the native run.

---------------------------------------------------------------------------
NOT PROVEN / LIMITS (honest list)
---------------------------------------------------------------------------
* World callbacks are seams: the oracle proves WHEN and with WHAT the
  original calls them, not their effects. 579A00's seam stores the leader
  (+40) because later native branches read it. Whoever binds a callback must
  use that callback's own proven port (see WIRING).
* Real Organization payloads: 1082 in the shipped maps (versions 8/9/10, all
  layout version 0, Updatable version 1). Their prefixes decode consistently
  (real-payloads.json), but complete replay needs the nested member records,
  i.e. the restoration record loader 4FAE80 and Element Mind (slot 4, not
  ported). Not run end-to-end.
* Unreachable from 57D130, implemented but not exercised: 4C2910's first
  "Layout mismatches LayoutData" branch, 4C1DB0's count log, 578300's member
  modifier path (members are null there), 57BF50's +74 != 0 branch.
* Refused instead of reproduced (explicit LoadError::Unsupported): saved member
  slot >= slot count (57CBA0 writes outside the native array; Rust test
  organization_out_of_range_member_slot_is_refused), negative ResourceVector
  count (4CD133 stores it as the size; one fixture row), out-of-range LayoutData
  group/formation indices, non-finite x87 arithmetic inputs. Binary32 overflow
  of a store produces the masked-exception infinity.
* Known partial-state gap: 4C30C3 stores the null lookup result to Layout+8
  before the "Unknown IDS ... KKC_OrganizationLayoutData" throw; Rust keeps
  the previous LayoutData (LayoutDefinition cannot hold null). Error is equal.
* Stream end of file: the engine stream's own behaviour is not established;
  both sides stop at the same read with LoadError::Truncated {at, wanted}.
* Host x87 hardware not cross-checked (no transcendental ops are involved;
  only add/mul/div/compare and binary32 stores).
* The stream vtable +14 query of 4C2D90 is seamed to "not writing" (the
  load direction); its real class was not identified.

---------------------------------------------------------------------------
WIRING (for the restoration agent; details in requests-to-lead.md)
---------------------------------------------------------------------------
Organization: at actor stage slot 7 call
  aa_actors::organization_state::load(&mut OrganizationState<String>, bytes,
      &OwnerFacts<String>, WorldFacts, &mut impl OrganizationWorld<String>,
      Precision) -> Result<usize, LoadError>
on the constructed state (owned.rs), and return the consumed length to the
ActorTail stage decoder. SavedComponent::decode_prefix cannot decode slot 7 on
its own: the payload embeds complete nested member records.
Transport: add Component::Transport(Box<TransportState<String>>) and call
  TransportState::new(owner, registered_resource_count)
  TransportState::load(&mut self, bytes, &mut impl Definitions<String>)
      -> Result<usize, LoadError>

No Cargo.toml / Cargo.lock change. No original game file, save or release was
modified. Removed engine caps: none (all native vectors became Vec; the
native arrays had no fixed caps here).

===========================================================================
ROUND 2 (2026-10-15): ORGANIZATION CALLEES, DENIZEN INVENTORY, 7C7840
===========================================================================
Ownership check first (grep of research/phase8-*/inventory.json and crates/):

PORTED HERE (no other port existed)
  crates/aa-actors/src/organization_callees.rs
    4CED10 experience_source: actor Experience+C, else the Experience+C of
      its Element's organization, else 0
    4CEBB0 rank_changed(actor, old, new): Performer 57E010 (only new != 0
      reaches 57DCF0, a callback), then Organization 57A330: every non-null
      member (4364C0, attached byte clear) without its own Experience,
      recursively
    561760 body_set_organization: nonnull owner and a scheduled (1, 0) Body
      Updatable entry (50B220) -> 50B1C0(1, 0) (both callbacks)
  Oracle native_organization_callees.py -> native-organization-callees.json
  (25 cases); test tests/native_saved_organization_callees.rs.

  Transport requirement: vtable A40100 +20 = 7C7840 (`mov eax, 3`).
  KKC_GActor::ValidateComponents 4CFB00 reads each component's +20 as the
  slot it requires and logs "Actor %s component %d requires component %d
  which is missing" (404ED0, line 0x5E8) on the first missing one: Transport
  requires the Mover (slot 3). transport_state::REQUIRED_COMPONENT.
  Oracle native_transport_requirement.py (original 4CFB00 over 8 component
  sets with the real vtables) -> native-transport-requirement.json; test
  transport_requires_the_mover_component.

  Denizen Inventory consumer (denizen_state.rs, transferred to me):
    DenizenState::with_inventory(owner, &[InventorySlotDefinition]) builds
      56B720's +18 slot list from the producer list (aa-rules
      ActorCatalog::native_denizen_inventory(..).slots, head first; NOT
      re-ported here).
    DenizenState::load_occupants(bytes at the version>=3 count word, health,
      pc) -> consumed: 56C39E..56C45B. The first min(count, slots) saved pairs
      go to the slots head first (+0 read straight into the slot, then the
      multiplier; +8 = fstp32(ActorData+284 of the target (45D730, null
      modifier) * multiplier)); the remaining pairs are read and discarded;
      a negative count reads nothing.
    DenizenState::apply_occupants(&decoded pairs, health, pc): the same for
      the restoration decoder's CountedList (decode-first: a truncated list
      fails before any slot write; the original wrote the earlier slots).
    DenizenState::new(&Definition{inventory_is_empty}) is kept unchanged so
      catalog.rs and aa-mapworld tests compile until the switch-over.
  Oracle native_denizen_inventory.py -> native-denizen-inventory.json: 252
  cases. Original 56B720 (with 568880/4C1D60/4C1CF0 over synthetic +34C
  organization layouts, 45EEF0 seamed) then the complete original 56C320.
  Versions 0..4, counts below/equal/above the slot count and negative, PC24
  rows that differ from PC53/PC64 (subnormal products), truncation sweep, and
  ALL 168 shipped Denizen payloads with a nonempty Inventory (carriers and
  airfields; 6 or 8 slots; 6, 8 or 12 saved occupants) built from the real
  aa-rules slot lists and target health. The test also checks that the
  aa-rules producer list equals the original constructor's list for all 168.
  Test tests/native_saved_denizen_inventory.rs (2 tests).

RECORDED FOR OTHER AREAS (not duplicated; see requests-to-lead.md)
  5705A0 Element attach (combat: mirror of its detach_membership 570670)
  5784C0 owner property propagation to a new member (combat: mirror of the
    578540 loop inside its remove_member)
  4CEF80 actor has property from source (aa-body property_api sibling of
    4CEF70; 581280 -> 5032D0 with a source filter)
  524B80 Organization Mind member added (AI agent, CAI.cpp Mind range)
  4E1200 recruit single pairing (economy recruit items)
  57DCF0 Performer ability rank gating (combat Performer), the callee of
    rank_changed

Round-2 commands:
  PYTHONPATH=... python research/phase8-saved-components/native_{organization_callees,transport_requirement,denizen_inventory}.py
  CARGO_TARGET_DIR=target/s-saved cargo test -p aa-actors --release --locked --offline -j 4
    -> 52 passed, 0 failed (whole crate; mine: native_organization_saved 2,
       native_transport_saved 2, native_saved_organization_callees 1,
       native_saved_denizen_inventory 2)
All five fixtures were regenerated after the harness gained observation
probes (native_machine.py); outcomes unchanged.

Not proven in round 2: the callbacks' own effects (57DCF0, 50B220/50B1C0);
the restoration wiring (not done by me); Denizen command objects (51B580,
count > 0 still refused by the restoration decoder).
payload_dump now also emits Denizen payloads and, with AA_GAME_ROOT set to the
game folder, the aa-rules producer slot lists and target ActorData+284
(merged into real-payloads.json: denizens, denizen_inventories).

===========================================================================
ROUND 3 (2026-10-15): "Actor name collision/action-table validation"
===========================================================================
What the native code actually is: the restoration guard fires for every
actor whose saved MapString key +DC is nonempty (aa-mapworld actor_finish.rs
finish_unlinked_actor). The original code at that point is the tail of
GActor::Load 4D8CF0, 4D971B..4D99C2 (after the 4D67F0 refresh). It contains
no action-table step; it is:
  1. +158 required object: a nonzero saved GTID must resolve in [B364C0]
     (generation and object), else 858A00 "actor %s required object ID %d
     missing - did it fail to load or is this a load order problem?"
  2. MapString validation (+DC key, +E0 text): expected name 4D8CA0 =
     ActorData+10 (GetLocalized("name"), 47D960/5B5620) when +170 == 0, else
     the numbered name 4D8AB0 (n%10 / n%100 pick actor_name_numbered[_st|_nd|
     _rd]|ds through 4B9F40; negative remainders give an empty name; format
     "%dth %s" etc. from strings_rtse.tgi via CString::Format/_vswprintf).
     wcscmp equal -> 5A1040 "MapString %s deleted - is unnecessary" and the
     pair is cleared (49AD00 with the empty CStrings B43B40/B43B48); else,
     unless 59E520(ctx) (strings read from the stream), a key missing from
     the map string table (4BAA20 create 0) logs "MapString %s missing -
     keeping '%s'".
  3. Numbered names (ActorData+1B0, owner virtual +108 nonnull, +170 == 0):
     actor version < 0xE with a key -> _swscanf(+E0, "%d", &+170); nonzero
     -> "fixing numbered name (%d)" and the pair is cleared. Still 0 ->
     Kingdom 4E4980 on the owner word +E8 assigns the next per-definition
     number (4CF1A0 stores +170); with no key -> "assigning numbered name (%d)".
  The "name collision" is the numbered-name counter: Kingdom +35C maps
  ActorData -> count (linear scan; insert 1 or increment). It is not saved;
  saved +170 numbers are not registered, so new assignments can repeat a
  saved number exactly as in the original.

Port: crates/aa-actors/src/actor_names.rs
  finish(SavedName{required_158, &mut key, &mut text, &mut number},
         actor_version, &mut impl NameWorld) -> Result<(), LoadError>
  expected_name / numbered_name / format_numbered (%d then %s, %%; other
  conversions refused), scan_decimal (_swscanf "%d": static "C" tables only
  - wide whitespace below 0x100 = 09..0D, 20, A0 from [B358A4]; ASCII digits
  (narrow "C" isdigit); 64-bit accumulate, low 32 bits; a char >= 0x100 at
  the whitespace test is classified by Windows GetStringTypeW, so refused),
  NumberedNameCounters<D>::assign (4E4980).
  AA.exe calls setlocale only for LC_NUMERIC (7F1965/7F1A39), so the "C"
  ctype tables used here are the only ones the game ever has.
Oracle: native_actor_names.py -> native-actor-names.json, 90 tail cases plus
two native 4E4980 runs (1 and 7 buckets). The tail is entered at 4D971B on a
rebuilt frame ([esp+20] version, [esp+A8] load context, ebx = 0). The
statically linked CRT (_vswprintf, _swscanf, wcscmp) runs natively; only its
import-backed environment is seamed (_getptd TLS block, _malloc/_free,
_lock/_unlock). CString buffer growth now runs through allocator +0/+8
(native_machine.py). Test: tests/native_saved_actor_names.rs (2 tests).

Not proven: real-map replay of the 269 affected records (needs the map
string table [B36544] contents and ActorData+10 texts from the restoration
and definitions sides); the Windows-classified (>= 0x100) whitespace case;
counter wrap past 2^32 (refused).

===========================================================================
ROUND 3, PART 2 (2026-10-15): R3-2 ActorData+10 display name (aa-rules)
===========================================================================
Native: ThingData 47D960 stores +10 = 5B5620(node, "name") and +14 = 1.
TGC_DataFileNode::GetLocalized 5B5620: field lookup 5B4750 (absent ->
TGC_DataFileNodeException "Node doesn't contain field."); a stored value
starting with '#' -> Right(len - 1) (41D390) -> 4B9F40([B36544]; key, B43B48),
the static empty CString being the default of a missing key; any other value
verbatim. The parser text is widened byte by byte (857210), so UTF-8 bytes in
a .tgi stay separate code units (C3 A9 -> U+00C3 U+00A9). A strict-template
"localized name" without a value fails earlier, in the parser ("Field 'name'
missing from node using strict template").
Port: crates/aa-rules/src/native_display_name.rs (aa_rules::native::display_name)
  native_stored_name(&ResolvedNode, &mut Lookup) -> Result<Vec<u16>>
  ActorCatalog::native_display_name(id, &mut lookup)
  Lookup = FnMut(&[u16] key) -> Result<Option<Vec<u16>>> (the 4B9F40 table;
  None = missing key -> empty text).
Oracle: native_display_name.py -> native-display-name.json (14 cases; the
ORIGINAL parser 5BD080 from research/phase8-restoration/native_tgi_parser.py
builds every node, then 47D960/5B5620 run with 4B9F40 seamed). Tests:
crates/aa-rules/tests/native_display_name.rs: 14-case replay + ignored real
data run (AA_GAME_ROOT): 1768 keyed, 191 literal, 0 failures over every
shipped v14 actor definition.
Hook (restoration, actor_names::NameWorld::display_name): call
  catalog.native_display_name(&definition_ids, &mut |key| table_4b9f40(key))

===========================================================================
ROUND 4 ADD-ON: Transport payload load 58BA90 (combat summon seam)
===========================================================================
58BA90(cargo ActorData, name CString, layout*, start x/y, drop x/y) on a
constructed Transport (58B9A0): two fresh ResourceVectors (4CD440: cost =
registered resource count of zeros; upkeep = 56F580(cargo, layout) =
economy_values::layout_values(Upkeep)), then 58B2A0: cost/upkeep copies
(4CD230), +28 cargo, +30/+34 drop, +38 = 0, +39 = 1, +40/+44/+48 = 0,
+4C = 0, +3C = number of layout slots whose group parameter is non-null
(1 without a layout), the KKC_OrganizationLayout reference swap (the old
layout's refcount drops; 4C2190 deletes it at zero), 4CF0F0 on the owner,
+4C = 1, +50/+54 = start, +58 = name. Always returns true.
Port: TransportState::load_payload(cargo, name, layout, start, drop,
  registered_resource_count, &mut impl PayloadWorld) -> Result<bool>
  PayloadWorld: cargo_upkeep (56F580), slot_groups (LayoutData +28 groups),
  refresh_owner (4CF0F0).
Oracle: native_transport_payload.py -> native-transport-payload.json (9
cases: resource counts 0/5/7, layouts with null/non-null group parameters,
old layouts with refcount 1/2/3). Test: native_transport_saved.rs
transport_payload_load_matches_original.
Hook (combat summon::apply, effect 4CCA70): layout = the 4C2290 copy of the
summoned object's ActorData+2F0 default layout when the cargo is an
organization (else None); name = empty (summon passes an empty CString).

===========================================================================
ROUND 4 (2026-10-15): POST-LOAD STAGE (HANDOFF steps 4-5)
===========================================================================
Where it runs. Map load 5A2B90 -> GWorld::Load 4FC600. After every record
section (TEAMS, KINGDOMS, USED_REQUIRED_OBJECTS, STATIC_THINGS,
DYNAMIC_THINGS) 4FC600 runs, in this order:
  1. 4FD3B3..4FD3F1 the PostLoad pass: iterator 4EFBE0 over [B364C0]
     (slot = (cursor & FFFF) + 1, 1 for cursor 0, up to +6000C inclusive,
     signed; null slots skipped; cursor = the object's own +14), virtual +DC
     (PostLoad) of every object.                      -> PORTED (this round)
  2. 4F1E10 world start, only when map settings +A8 is 1 or 4 and 59E520:
     kingdom setup 4E3690/58EA00, every actor state 0x38 (4CEED0) and
     58F010 on its kingdom +32C, 4E37D0(0,1) for kingdoms with +338 == 0.
                                                      -> not ported (inventory)
  3. VWorldState::InitForLoadedMap: 4F5290 / 4F1C20 -> GWorld +22C/+230
     look-at, 404F60 "[4610] ... lookat=(%g,%g)" (presentation).
                                                      -> not ported
  4. Placeholder replacement 4FD47C..4FD781 (class-changed records, 4F9C10):
     the conversion agent (research/phase8-conversion).
There is no separate "PostExisting" virtual for loaded objects. The creation
path (4F9F17..: ctor, +E4, +E8 4D7230, +EC 4CF300, 4F9510, 50C220) is a
different path (world_publication.rs ports 4CF300). Views are created later
by KKC_GWorld::OnVWorldCreated 4F0750 (objects with +C == 0 join +130).

Class virtuals +DC (saved class -> constructor -> vtable -> PostLoad):
  1 Thing     4EF020 A2A680 943800 (ret 4)
  2 Object    4EA690 A2A2A0 4EAA30
  3 Actor     4D6D70 A273F0 4D3560
  4 Kingdom   4E5370 A299D8 4E4220
  5 Team      4E67C0 A29780 943800
  6 Area      4DB6F0 A28A00 4EAA30
  7 Camera    4DBF40 A28B88 4EAA30
  8 Group     4DE490 A29490 4DDFA0 (4D3560, then 4DDCD0/515F40 on +1A4) UNPORTED
  9 Doodad    4DC6F0 A28E90 4EAA30
  (A28D18, Load 4DC690, outside this factory: 4EAA30)
GObject::PostLoad 4EAA30: +50 (the version >= 2 saved "associated" word)
  non-zero and unresolved (generation or empty slot; destroyed flag NOT
  checked) -> 404ED0 GObject.cpp:0x2D8 "Actor '%d' for required object '%d'
  (%s) not loaded" (+50, +14, [+4]+10), then virtual +12C (GActor/Kingdom
  4CFF70 = 4ECC50 + flare 515F40; GObject 4ECC50: clears +50, vt+50, ...).
GActor::PostLoad 4D3560: 4EAA30; component vt+30 for slots 0..30 in order
  (pointer re-read per slot); then +118, +120, +128 lists drop every node
  whose GTID is unresolved or whose object +8 bit 0 is set (50C510).
GKingdom::PostLoad 4E4220: 4D3560; +34C/+350 unit groups: members that do
  not resolve are removed (4D19F0(id, 0)), an emptied group is removed
  (shift, 4E3ED0/4E28D0 with no member, free); then the +214/+218 recruit
  items with the count read ONCE: item +8 byte set, or +10 == 0, or a live
  +10 without +8 bit 0 -> skip; else 5A1040 "Recruit item single '%s'
  attached to dead actor" ([item+4]+10), item vt+8(1), 4E1C70(-1) (a live
  +10 actor: 578DA0 on its Organization, its +E8 Kingdom +224 += 1; +10 = 0).
  An index at or past the CURRENT count reads [0+8]: native access violation
  (an item virtual that removes items does that) -> Error::NullDereference.

Component virtual +30 (factory 4CF480 vtables; verified per constructor):
  Body        A3B0E8 561DF0  561C60 = 561B40 (SVar UnitSupplyEnabled, no
              Organization +7C, ThingData+2A8 > 0.0 by fcomp) and no Element
              organization (+80 -> +10) and +16C unresolved. Needed and
              50B220(1,0) false -> 50AC60(0); not needed and scheduled ->
              50B1C0(1,0).
  Properties  A3E3DC 581260  Component+C (Updatable list head) null -> 50AC60(0)
  Supply      A400C0 581260  same function
  Denizen     A3C14C 569F80  +40 command list; each live member with a Mind
              (+70) gets 52A020(command) (refcounted Mind+30)
  Wall        A40198 58CB40  GWorld+2C < 26: 50B250, then 50AC60(0) when
              ThingData+42C (not in the tutorials)
  Settlement  A3EEA0 586F60 = 586BC0(0): recreates settlement GThings through
              4F9C10 -> UNPORTED (Error::Unsupported; not in the tutorials)
  Defender    A3BBD4 5676B0  +24 entries {key, value}: dead key -> a live
              value loses 4CE6C0(owner) from its +118 (4D0CD0/4F1430), the
              entry is removed (79E3C0); live key + dead value -> value =
              4CE6C0(key). Then +1C == 0: 567140(ability, null) for every
              Ability (count read once) and 50B250; else 50B220(0,0) false ->
              50AC00(0) and 50B220(3,0) false -> 50ACC0(1.0f, 0).
  Hq          A3CA4C 5736B0  frees +20[i], +24 = 0; +10 == -1 and owner +E8
              Kingdom with +234 -> +10 = 4E2640 (count = [+234]+40 <= 0 -> -1
              without a draw; random % count; taken (+36C) -> lowest free
              index or -1; a found index is prepended to +36C)
  every other class (Projectile, Character, Mover, the three Minds 4CF480
  builds, Performer, Organization, Element, Economy, Structure,
  Construction, Recruit, Experience, Builder, MobileDeployment, Research,
  Detection, Aircraft A3AF78 (A3AF48, listed in my round-3 notes, is its
  +28 subobject), Transport, Lair, Flare, MetaGame, Portal, City): 943800.
Queue helpers (aa-world queues; their Updatable gates vt+8/+C are 6A71D0
= true for all the components above): 50AC00 = queue 0 periodic, 50AC60 =
queue 1 periodic (both one world RNG draw in 50B2C0), 50ACC0 = queue 3
add_after, 50B220 has, 50B1C0 remove all matching, 50B250 cancel all.
Entry (0x1C, 50ABC0): +0 Updatable, +4/+8 bucket prev-link/next, +C/+10
owner prev-link/next, +14 data, +18 queue byte. 50B220 starts at the
Updatable's most recent entry and follows the BUCKET next (+8), so it sees
other owners' later entries of that bucket and misses the owner's entries
in other buckets (8 native runs: native-post-load.json
updatable_has_50b220; post_load::native_has_scheduled). aa-world's
Queues::has is an owner-list test: requests-to-lead.md R4-2.
The Hq allocation draws the same world RNG (4F02B0 GWorld+D4, 85D840), so
the pass order fixes the RNG stream (slots ascending, components in slot
order inside an actor).

Tutorials (tutorial-post-load-inventory.json, payload_dump --masks):
  tutorial01 (33 top-level actor records): Body 9, Properties 15, Denizen 2,
    Supply 5, Defender 4 reach non-trivial PostLoads; the rest 943800.
  tutorial02 (44): Body 25, Properties 27, Denizen 4, Supply 4, Defender 13,
    Hq 2. No Wall, Settlement or class-8 Group record in either map, so the
    two unported PostLoads are never reached there. Kingdoms (4E4220),
    Objects/Areas/Cameras/Doodads (4EAA30), Things/Teams (no-op) also run.
  Note: the restoration decoder has no Hq (slot 30) saved Load yet
    (requests-to-lead.md R4-4); tutorial02's 2 Hq records need it first.

Port: crates/aa-actors/src/post_load.rs
  post_load_pass(&mut impl PostLoadWorld)       4FD3B3 + 4EFBE0
  class_post_load / object_post_load / actor_post_load / kingdom_post_load
  component_post_load, body_post_load (+ body_supply_enabled 561B40,
  body_needs_supply 561C60), scheduled_post_load (581260), wall_post_load,
  denizen_post_load, defender_post_load (+ defender_clear_ability_target
  567140 null target, organization_of 4CE6C0, remove_from_list_118 4D0CD0),
  hq_post_load (+ kingdom_allocate_hq_index 4E2640), detach_recruit_item
  (4E1C70). Errors: NullDereference (native access violation), Undefined
  (a GActor field read on a non-GActor object, GTID 0 in the table),
  Unsupported (Group 4DDFA0, Settlement 586BC0, the 567140 attach branch).
Oracle: native_post_load.py -> native-post-load.json: 30 whole-pass cases
  (26 plus PC24/PC64 replicas of the two float cases: 561B40's fcomp of a
  loaded binary32 is precision independent), synthetic GTID tables with the
  real class/component/Updatable vtables; executed natively from 4FD3B3 to
  4FD3F3 (iterator, every PostLoad above and their helpers); seams: the six
  queue helpers, 4F02B0/85D840, 404ED0, 5A1040/856560, vt+12C, 578DA0, the
  recruit-item vt+8. Every call is recorded in order; the final state
  (lists, Updatable lists, Mind commands and refcounts, Defender entries and
  Ability targets, Hq index/items, Kingdom groups/items/+224/+36C) compared.
Test: crates/aa-actors/tests/native_saved_post_load.rs (3 tests: the 30-case
  replay through a mock world, the explicit Unsupported/Undefined errors,
  and the 8 native 50B220 walks).

Restoration hook (requests-to-lead.md R4-1): after the DYNAMIC_THINGS stage
(every record restored and registered; before 4F1E10 / InitForLoadedMap /
replacement), call aa_actors::post_load::post_load_pass(&mut world) with
world: PostLoadWorld over the restored registry.

Round-4 commands:
  PYTHONPATH=... python research/phase8-saved-components/native_post_load.py
  PYTHONPATH=... python research/phase8-saved-components/native_transport_payload.py
  PYTHONPATH=... python research/phase8-saved-components/native_display_name.py
  CARGO_TARGET_DIR=target/s-saved cargo test -p aa-actors --release --locked --offline -j 4
    -> 60 passed, 0 failed (whole crate; mine: native_organization_saved 2,
       native_transport_saved 3, native_saved_organization_callees 1,
       native_saved_denizen_inventory 2, native_saved_actor_names 2,
       native_saved_post_load 3); clippy --tests clean for my files.
  AA_GAME_ROOT=... CARGO_TARGET_DIR=target/s-saved cargo test -p aa-rules --release --locked --offline -j 4 --test native_display_name -- --include-ignored
    -> 2 passed
  payload_dump --masks <tutorial01.RMP> <tutorial02.RMP> (own workspace,
    CARGO_TARGET_DIR=<scratch> cargo build --release --offline)

Not proven this round: a real restored-map PostLoad pass (needs the
restoration's complete registry: every tutorial record restored first);
Settlement 586BC0 and Group 4DDFA0 (unported, absent from the tutorials);
the queue helpers' own effects (aa-world, tick agent) and 52A020/578DA0/
item vt+8 internals (seams); 4F1E10 and InitForLoadedMap (inventory only).

===========================================================================
ROUND 5 (2026-10-15)
===========================================================================
1. HQ component (slot 30): constructor 574490 and saved Load 574040
   Layout: +10 badge index (ctor -1), +14/+18/+1C slot vector (ThingData
   +530 null slots, 1 when <= 0; 430240), +20/+24/+28 pending items (ctor:
   null buffer, capacity 1). Items: 12 bytes {+0 kind (3 = none), +4, +8}.
   Load: 50BCD0 Updatable (GWorld+284 table, 858AD0 "Bad or duplicate
   updatable ID"), version <= 3, +10; a badge index != -1 needs the owner's
   virtual +108 Kingdom (null: native access violation at 5740AA) and is
   reset to -1 with the 404F60 log (HQ.cpp 0x20F) when >= [Kingdom+234]+40
   ("HQComponent (%s): loaded badge index %d no longer valid for nation
   '%s'", owner virtual +24 = 4D99D0 display name, nation +10). Version 1:
   a counted word list read and freed (4D1AF0). Otherwise a counted list of
   flagged item records (5737E0: version < 3 = byte + word, kind 0 only when
   the byte is set; version 3 = kind, +4, +8); items whose kind is not 3 fill
   the slots in order (573750), the rest (and overflow) go to +20 pending,
   which PostLoad 5736B0 frees.
   Port: crates/aa-actors/src/hq_state.rs (HqState::new(owner, slots_530),
   HqState::load(bytes, &mut impl HqWorld) -> consumed bytes).
   Oracle: native_hq.py -> native-hq.json (26 cases: versions 0..4, legacy
   lists, overflow, slot counts 0/-2/3/5, badge checks, Updatable errors,
   truncation). Test: tests/native_saved_hq.rs.

2. aa-world queues (additive edit in crates/aa-world/src/queues.rs, the tick
   agent being finished; existing behaviour and tests unchanged):
   Queues::has_from_owner_head(owner, queue, data) = native 50B220 (walk from
   the owner's most recent event along its BUCKET successors, any owner);
   Queues::remove_matching(owner, queue, data) -> count = native 50B1C0
   (the owner's own list, each match cancelled in order with the 4F1A70
   cursor move of Queues::cancel). Native runs: native-post-load.json
   updatable_has_50b220 (8) and updatable_remove_50b1c0 (7, 50ADE0 and
   4F1A70 executed). Test: crates/aa-world/tests/native_queue_links.rs
   (rebuilds each recorded link topology with deferred add_at inserts).
   aa-world: 10 passed (queues 3, dispatch 1, removal 2, unit 2, links 2).

3. Kingdom PostLoad recruit items (R4-3), now native end to end:
   the +214 items are RecruitSingles (vtable A29694, +8 available, +C actor,
   +10 organization GTID). 4E4220 tests +10 (organization); virtual +8 is
   4E0EC0 (+8 = 1). 4E1C70(-1): a live organization WITH an Organization
   component calls 578DA0(item, -1), whose only use of the index reads and
   clears Organization+34[-1] (the other 578DA0 callers pass a real slot
   index) -> Error::Undefined (oracle: a probe at 578DAB stops on an index
   outside +34/+38). Without one, its +E8 Kingdom +224 += 1; +10 = 0.

4. Virtual +12C (GObject 4ECC50, GActor/Kingdom/Group 4CFF70):
   4ECC50: +50 = 0; virtual +50(ThingData+1C bit 10) (GActor 4D4A10 =
   actor_creation::set_blocking, GObject 4EE840); flags8 bit 30 cleared;
   for GWorld +154 kingdoms 0..count-2 (the LAST one is never visited):
   non-null, +190 == 0, object +58 > 0 (virtual +8C 4EA3D0) and a memory
   record +5C[kingdom+18C]: 79DEB0 (UI agent [B36528] with +20), 4EC4A0
   refresh, 79E7B0 (record re-read). 4CFF70 adds: ThingData virtual +24
   (462E50: +34 needs_space_for_settlement_zone) -> zone 2 (+148[2]; a
   vector of <= 2 entries is a null `this`) moves to +20/+24 with the 4BB000
   radius (tile + tile + [B39F58]+74 + 1e-4, x87 at the thread precision).
   Port: post_load::{required_object_lost, gobject_required_lost,
   discovery_zone_radius}.

5. World load tail (4FD3F3..4FD47C): crates/aa-actors/src/world_load_tail.rs
   after_post_load: gate (map settings +A8 in {1, 4} and 59E520) ->
   LoadTailWorld::world_start (bind to aa_economy::world_start::world_start,
   which already ports ALL of 4F1E10: there is no non-economy part left);
   4F5290 local kingdom (app state [B36550]+E4 == 3 -> none; by local player
   name 4F48E0 over GWorld +148 names +18 (wcscmp) when GWorld +48 == 0 or
   the mode is non-zero; else 76AC70: [B36558]+8 player +28; by-name with no
   local player = native access violation 4F5304); 4F1C20 look-at (map
   centre [GWorld+30]+C/+10 x 0.5 in the x87; the kingdom's +284 actors
   without flag bit 22: best ThingData +1B4 among Settlement holders, else
   flag-bit-29 actors, else 4CE6C0 organizations, strictly above -1; then
   +1F8, then the centre); GWorld +22C/+230; 404F60 log with doubles.
   Oracle: native_load_tail.py -> native-load-tail.json (24 cases incl.
   PC24/PC64 replicas with subnormal and maximal map sizes). Test:
   tests/native_saved_load_tail.rs.

6. Settlement PostLoad 586BC0(0) and GGroup PostLoad 4DDFA0:
   586BC0: per entry i < +28 (re-read), a GACD block (ActorParams::new
   defaults; definition 45D7C0(kind): ThingData +49C/+4A0/+4A4 for kinds
   0..2, null otherwise -> native access violation at 4F9C92; position owner
   +20/+24 + entry offset (x87 fadd at the thread precision), owner +28 z,
   direction from the entry, owner Kingdom virtual +108) goes to 4F9C10;
   a result's GTID lands in +3C[i]; a null result only marks failure.
   4DDFA0: 4D3560, then a +1A4 target: flag bit 2 = position target (clamped
   by 552640 and written back), else flag bit 0 with a live +10 object (its
   clamped position); the position moves zone 7 (+148[7]) with radius +1B4.
   Port: post_load::{settlement_create_things, group_post_load,
   group_target_position}; PostLoadClass::Group is no longer unsupported.

   Pass oracle (native_post_load.py, 42 cases now: the 30 previous ones
   updated, 12 new) executes all of the above natively with seams for
   virtual +50, 79DEB0/79E7B0, 4EC4A0, 515F40, 552640 and 4F9C10.

7. Component zone creators. Already ported, oracle-proven, by the
   actor-creation agent in crates/aa-actors/src/component_virtuals.rs
   (native-component-leaves.json): InitExisting +8 Defender 566DB0 (ZOC 4),
   Settlement 585940 (zone 2, 584CF0), Supply 58B0E0 (zone 0, 58AE10 =
   aa-economy supply), and +14 Organization 579DD0 (ZOC 4), Denizen 568070
   (ZOG 5), Wall 58D3B0 (zone 3), City 563FE0 (zone 8), Detection 56EC90
   (aa-zones). They create/move zones through world methods named after
   4D69A0 / 515F40 (aa_zones::component::create_zone / zone::move_to_position).
   I did not duplicate them. Three addresses of the zones README list are NOT
   zone creators: 568040 is a pure predicate (8 lists of 8 bytes, entries
   5..7 ignored, all others empty) used by the Settlement update 587290;
   563E10 is the Character Updatable's deferred update (+18); 58CF50 is the
   Wall Updatable's periodic update (+14). The zone-5/8/3 creators are
   568070 / 563FE0 / 58D3B0. aa-actors cannot call aa-zones directly: it is
   not an aa-actors dependency (Cargo.lock change), so every port names the
   aa-zones function its world method binds to.

Round-5 commands:
  PYTHONPATH=... python research/phase8-saved-components/native_{hq,post_load,load_tail}.py
  CARGO_TARGET_DIR=target/s-saved cargo test -p aa-actors -p aa-world --release --locked --offline -j 4
    -> 74 passed, 0 failed (aa-actors 64, aa-world 10); clippy --tests clean
       for my files (incl. aa-world queues.rs / native_queue_links.rs).

Not proven: real restored-map runs; 4F9C10 / 552640 / 515F40 / 4EC4A0 /
79DEB0 / 79E7B0 / virtual +50 internals (seams owned by the creation, zones
and UI agents); 568040 / 563E10 / 58CF50 (not zone creators, unported).
