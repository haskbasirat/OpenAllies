PHASE8 LAYOUTS: NATIVE ORGANIZATION LAYOUTDATA PRODUCER — 2026-10-15

Goal: the OrganizationLayoutData objects exactly as AA.exe builds them from
the original data, the loaders that read them from TGI, and the lookups the
Organization constructor 579BD0 / Load 57D130 use (restoration request R2-a:
all 1,067 Organization records and both tutorials stopped at
organization_layout_info).

Status: ported and oracle-proven; every shipped organization definition
loads its layouts with zero native errors. NOT wired into restoration yet
(restoration agent's files): exact wiring in requests-to-lead.md L-1.

---------------------------------------------------------------------------
FILES
---------------------------------------------------------------------------
crates/aa-rules/src/native_layout_data.rs   producer (module
    aa_rules::native::layout_data; mod line appended in native.rs)
crates/aa-rules/tests/native_layout_data.rs  6 tests (2 need AA_GAME_ROOT)
research/phase8-layouts/
  layout_machine.py              Unicorn machine (definitions-agent parser
                                 harness + seams listed below)
  native_layout_data.py          oracle -> native-layout-data.json
                                 (3 shipped runs PC24/53/64 + 35 cases)
  native_organization_component.py
                                 oracle -> native-organization-component.json
                                 (52 cases)
  debug_one.py                   runs one case in-process (debug helper)
  xrefs.py                       absolute-address scan of the image
  *.asm.txt                      Capstone listings of 4C3610, 4C32B0,
                                 4C34A0, 4C3110, 4C1BE0, 464BD0 466C30..
  run-full.log, run-org.log      oracle run outputs
  metagame-check.log, aa-rules-check.log   required post-change checks
  findings.json (write_findings.py), requests-to-lead.md

---------------------------------------------------------------------------
NATIVE OBJECTS AND FUNCTIONS (image base 0x400000)
---------------------------------------------------------------------------
GameData (48A8E0) DataManagers, vf0 483A40 -> vf1C:
  +464 Property        47EFD0(0x40 buckets, mode 1)  vf1C 48B580, create 48B640
                       (0x60 object 47D8D0, vtable A0C834, LoadData 4C48C0 =
                       4C42D0 + is_technology +5C). Set at 48B1B4.
  +470 OrganizationLayout 481740(0x3D, mode 0) also [B3AB00]; vf1C 4818A0,
                       create 481960 (0x54 object 4C2690, vtable A232D8)
  +474 Formation       481A50(0x11, mode 0); vf1C 481BB0, create 481C70
                       (0x28 object 4C2540, vtable A232C4)
  +478 GroupFormation  4840A0(2, mode 0); vf1C 484200, create 4842C0 (49A660)
  vf1C: GetString("IDS") (5B54D0); mode != 0 reuses an existing IDS; the
  create path throws "Data IDS already used: '%s'" (A0B9F8) for a registered
  IDS, sets +C = manager count (registration ordinal), registers, THEN runs
  the type LoadData: a definition whose LoadData throws stays registered with
  its partial state. Find 463FA0 = 48D500 (= 48D490): empty -> null,
  translation map first, then the registry with the target only.
48D660: a throw in any root abandons the rest of that file (catch funclets
  log "%s ... %s" line 0x917); GameData+4 = 85A550(file path) (directory:
  last '/' or '\', else through ':', else empty).

KKC_FormationData::LoadData 4C34A0: 47D960 name (+10, +14 = 1); +18
  organized (ctor default 1), +19 herding, +1A aggressive; icon: 4763C0 (a
  leading '/' run is trimmed, else "%s/%s" with GameData+4 when nonempty)
  then 5A77E0 texture -> +1C; group_formation_ids nonempty -> 48D500 on +478
  -> +20, null -> "Unknown group formation '%s' loading formation '%s'";
  optional [Property] -> new ModifierPropertyData (461ED0) +24, 4C42D0.
KKC_OrganizationLayoutData::LoadData 4C3610:
  47D960; +18 = -1; +4C = 0;
  per [Slot] (5B4220/42BE10): push a 16-byte entry (4C2250 defaults: name "",
    property 0, number 1, required 1, locked 0, morale_line 0); property_IDS
    -> 463FA0 on +464 -> +4, null -> "Unknown property '%s' loading layout
    '%s'"; name (5B5620) +0; required +C; locked +D; number +8; morale_line
    +E (true: +4C += number, wrapping); [B3B6B0] = max(number); number > 0:
    +28 gets `number` copies of the entry index; primary: +18 >= 0 ->
    "Layout '%s' cannot have more than one primary slot.", else +18 =
    +2C - 1 (slot count - 1, so a primary with number 0 points at the
    previous slot, or -1).
  [B3B6B4] = max(entry count).
  per [Formation]: a 0x1C LayoutFormation (vtable A232A8) is appended to
    +34 BEFORE its load 4C32B0: formation_IDS -> 48D500 on +474 -> +C, null
    -> "Unknown formation '%s' loading layout '%s'"; per [Position] a
    12-byte entry (offsets zeroed by 4C1E00/ctor) property_IDS -> 463FA0 ->
    +0, null -> "Unknown property '%s' loading formation '%s'" (2nd argument
    is the LAYOUT IDS); offset point2 -> +4/+8. Then 4C1BE0: position count
    != slot count -> 404ED0 line 0xCB "formation '%s' size %d does not match
    layout '%s' size %d"; else the first slot whose position property !=
    entry[+28[i]] property -> 404ED0 line 0xD5 "slot %d formation '%s'
    property '%s' does not match layout '%s' property '%s'"; false ->
    "Incompatible formation '%s' loading layout '%s'".
  no formation -> "Layout '%s' needs at least one formation";
  BrokenProperty (5B4410) absent -> "Layout '%s' missing broken formation
    property"; else ModifierPropertyData -> +50, 4C42D0;
  +40 = 0, default_formation_ids nonempty -> 4C3110; +44 = 0,
    combat_formation_IDS -> 4C3110; retreat_formation_IDS nonempty -> 4C3110
    -> +48, else the LAST formation whose FormationData +18 (organized) is
    nonzero, else +48 unchanged (0).
  4C3110(name): 48D500 on +474 null -> "Layout '%s' unrecognized formation:
    '%s'"; not in +34 -> "Layout '%s' does not use formation: '%s'".
ActorData 464BD0:
  +1D4 (4C4CF0 at 4650C7): PropertiesComponent Property children ->
    463FA0 on +464 at the actor's load time, prepended; unknown -> "Unknown
    property '%s'".
  466C4F..46707C OrganizationComponent: +174 |= 0x80; supply_used +2F4,
    shortfall_damage_rate +2F8, company_group_priority +320,
    morale_breakpoint +308 (< 0 -> throw), control_range +2FC (<= 0 ->
    throw), operational_range +300 (<= 0 -> throw), +304 = op*op (x87 at PC,
    fstp dword), layout_IDS -> 463FA0 on +470 -> +2E8 (null -> "Unknown
    layout '%s' loading organization actor '%s'"), entrenchment_time +2EC,
    rampages +310, [+314] wait chance/points/direction/distance/max detached
    ratio/aggressive, direction *= binary32 0x3C8EFA35 (9FC064) at PC,
    recruit_time +318, recruit_full +31C, in_supply_detached +31D;
    +2F0 = 4C2110(+2E8) (params = entry count of nulls); per authored Slot i:
    i >= entries -> "Too many slot elements ..."; locked && !required ->
    "Slot definition index %d is locked, ..."; element_IDS empty ->
    4C2310(i, null) and required -> "Slot definition index %d requires
    defining an element ..."; nonempty -> deferred reference 473AD0
    (prepended to GameData+824); remaining entries: 4C2310(i, null),
    required -> same throw.
  460000 (vf0 461750 of each deferred reference; 475230 walks GameData+824
    head first = reverse creation order, after ALL data files, no catch):
    4BDBA0(ids, 0x2B); 4C2310 stores it unless it lacks the slot property
    (45EEF0); null -> "Unknown element ..."; +174 bit 8 clear -> "Actor '%s'
    is not an element ..."; +1D4 lacks the entry property -> "Actor '%s'
    does not have property '%s' ..."; +174 bit 19 and entry number > 1 ->
    "Organization '%s' packable/unpackable element '%s' must be in own
    slot"; owner +17C entries missing from the element +17C list are
    prepended (tail +180 set on the first insert); owner +174 bit 30 and
    element bit 21 and element +558 -> "HQ Organiazation '%s' element '%s'
    has a research component with techs".

---------------------------------------------------------------------------
RUST API (aa_rules::native::layout_data)
---------------------------------------------------------------------------
NativeLayoutData::load(&ActorCatalog, Precision) / catalog.native_layout_data(pc)
  .properties / .embedded_properties / .group_formations / .formations /
  .layouts (partial objects + .error kept, as natively) / .files (48D660
  outcome) / .events (ordered 4C42D0 seam calls, textures, logs, throws) /
  .max_slot_number ([B3B6B0]) / .max_slot_groups ([B3B6B4])
  find_layout(ids) / find_layout_at(ids, RootPosition) (463FA0 on [B3AB00])
  find_formation(ids) / find_property(ids, at)
  layout_info(&layout) -> LayoutInfoFields (= aa_actors LayoutInfo fields
    + formation_40 + morale_count_4c; Err for a partially loaded layout)
  formation_facts(handle) -> (FormationData+1A, +24 property handle)
  layout_formation(handle)
  actor_has_property(catalog, actor, property_handle, pc)   (45EEF0)
  organization_component(catalog, actor, pc) -> OrganizationComponentData
  organization_component_fields(node, ids, at, pc) / link_element(..) /
  element_facts(..)
native_actor_properties(catalog, &data, actor, pc)  (+1D4)
PropertyRef::{handle, from_handle}: registered ordinal+1;
  embedded 0x8000_0000|(index+1). LayoutFormation handle = creation index+1.
StoredName::resolve(lookup): 5B5620 string-table step left to the caller.
native_icon_path / native_directory / native_format / merge_list_17c.

---------------------------------------------------------------------------
ORACLES (PROVEN)
---------------------------------------------------------------------------
PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-layouts/native_layout_data.py [--workers N]   (~6 min)
  python research/phase8-layouts/native_organization_component.py      (~8 min)
Every case runs the ORIGINAL code: parser 5BD080 on the template files
(Game/resource_list, template_fx, template_ability_effect, template_modifier,
template_property, template_terrain, template_group_formation,
template_formation, template_organization_layout(_aa), and for the
ActorData oracle Components/template_organization) and on the case files,
then the original 48D660 with the four original DataManagers. All executed
windows are pinned and checked byte-identical in the v14 executable.
native-layout-data.json:
  shipped: Game/group_formations, the ten Properties/*.tgi, Organizations/
    Formations/regiment_formations and the eight Organizations/Layouts files
    at PC24/PC53/PC64: 160 Property, 3 GroupFormation, 5 Formation and 10
    OrganizationLayout definitions, no throw, no log, [B3B6B0] = 4,
    [B3B6B4] = 6; identical at the three precisions.
  35 synthetic cases: retreat/default/combat formation rules, unknown and
    empty property/formation IDS, two primaries, primary with number 0,
    negative/zero numbers, morale lines, size/property/order mismatch logs,
    no formation, missing BrokenProperty (template without it), duplicate
    layout/formation IDS, throw abandons the rest of the file, layout loaded
    before its formations, case-insensitive references, localized names
    (fixture string table), icon path forms, formation flags, Property mode-1
    reload, '//' value.
native-organization-component.json: 52 cases: template defaults and the AA
  ambient values at PC24/53/64, x87 rounding (incl. two PC24 double-rounding
  subnormal cases that differ from PC53/PC64: 1714918 vs 1714919 and
  2031358 vs 2031359), every throw of 466C4F..46707C, and every 460000 branch
  (unknown element, not an element, missing property with the parameter not
  stored, packable multi/single, HQ research with/without items, walk order
  with two failures, case-insensitive lookup, +17C merge).
Seams (recorded, never executed): 4C42D0 KKC_ModifierPropertyData::LoadData
  (property contents are the Property producer's; reached from 4C48C0,
  4C3610 BrokenProperty, 4C34A0 Property); 5A77E0 texture load (fixture
  handle; path recorded); 4B9F40 string table (fixture table); log leaves
  404EA0..404F90 (recorded with printf arguments); 5AB1D0 path formatter;
  4BDBA0 (ActorData oracle: fixture element objects). 6F7810 allocations are
  filled with 0xA5; page-0 accesses fault unless FS-prefixed; C++ EH seam
  runs the original 48D660 catch funclets (inner-frame destructors skipped:
  reference counts only). The ActorData fragment is entered at 466C4F on a
  fixture ActorData (0xA5 fill) with EDI = parsed actor root, EBX = 0 and
  [esp+2C] = empty CString; every other frame slot is 0xA5-filled (an
  uninitialized read would surface as a mismatch; all 52 cases match).

Rust (CARGO_TARGET_DIR=target/s-layouts):
  cargo test -p aa-rules --release --locked --offline -j 4 --test native_layout_data
    -> 4 passed (synthetic layouts, ActorData cases, icon rules, handles)
  AA_GAME_ROOT='E:\Games\Axis and Alies' ... --test native_layout_data -- --include-ignored
    -> 6 passed: the shipped oracle replayed on the same files read through
       V14Sources (sha256 equal to the oracle inputs) at PC24/53/64; and the
       whole v14 native-order catalog: 165 Property (160 before the layout
       files), 5 Formation, 10 OrganizationLayout, no abandoned file, no
       partial definition, LayoutInfo for all 10 layouts, registries equal to
       the native_load generic registries, and ALL 192 actor definitions with
       an OrganizationComponent load fields, layout, default parameters and
       element links with no native error (herd_ambient_large 4,
       herd_ambient_single 12, regiment_hq 20, regiment_hq_mobile 20,
       regiment_infantry 64, regiment_mechanized 13, regiment_single_unit 4,
       regiment_vehicle_heavy 23, regiment_vehicle_light 32); their layout and
       parameters equal ground.rs OrganizationDefinition for all 192; 45EEF0
       holds for every linked element. The 1,082 saved Organization payloads
       of the 80 maps all name a loaded LayoutData whose entry count equals
       the saved parameter count (see L-2 for 143 records whose saved
       parameters have no v14 definition).
  A deliberate mutation (retreat fallback off by one) makes the synthetic
  replay fail.

---------------------------------------------------------------------------
NOT PROVEN / LIMITS
---------------------------------------------------------------------------
* 4C42D0 (Property contents) and the GroupFormation type LoadData are not
  executed by the oracles nor ported: a throw inside them is not modeled
  (none in the recorded native v14 logs, definitions README).
* 48D660 abandonment by OTHER managers' roots in the same file: the producer
  refuses (explicit Unsupported) when native_load records a first exception
  that is not its own.
* 460000's +17C merge is ported as merge_list_17c but not stored anywhere
  (the list's meaning belongs to other areas).
* Translate roots for these managers: applied (48D2D0 rules), but their
  throw message formats are not ported (Unsupported); none exist.
* Property mode-1 reload after a named load: 47D960 inside the unported
  4C42D0 would throw; refused as Unsupported (none in shipped data).
* Strings with a quoted value (`"..."`) in string fields stay unsupported in
  ResolvedNode::field (definitions/tgi area); none in these files.
* The '//' value divergence of tgi.rs (requests L-3).
* Removed native caps: none (the native vectors grow; initial capacities 7/5
  are not limits; [B3B6B0]/[B3B6B4] are recorded maxima, not caps).
* No original game file, save, profile or release was modified.
