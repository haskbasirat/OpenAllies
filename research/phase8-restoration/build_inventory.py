"""Annotate the InspectPartial all-maps census (unsupported-inventory.raw.json,
written by crates/aa-runtime/tests/restore_inventory.rs) into
unsupported-inventory.json for work splitting. Native addresses are the
functions that own each unsupported operation, as far as identified."""
from pathlib import Path
import json, re, collections

HERE = Path(__file__).resolve().parent
raw = json.loads((HERE / 'unsupported-inventory.raw.json').read_text())

# (regex on normalized error, operation, component, rust site, native addresses, notes)
KNOWN = [
    (r'^source class # differs from definition class #$', 'saved class code vs resolved definition class',
     'DYNAMIC_THINGS record dispatch', 'aa-mapworld/src/restore.rs restore_dynamic',
     ['4FC340 (per-section loop, seeks to next index entry when a record yields null)',
      '4FAE80 (per-record loader: reads IDS/class/saved ID, generation check, definition lookup 427BD0, class compare)',
      '4F26F0/4EA050/4F9840 (class-2 Object record against a different definition class: conversion branch when the caller passes its context)'],
     'Native does NOT abort: mismatching records are converted (saved class 2 with context) or logged and skipped. 7780 are trees saved as class 2 Object whose v14 definitions are class 9 DynamicDoodad.'),
    (r'^unsupported native definition: missing definition ', 'definition missing from v14 sources',
     'DYNAMIC_THINGS record dispatch', 'aa-mapworld/src/bootstrap.rs resolve_definition via restore.rs',
     ['4FAE80: 427BD0 definition lookup returns null -> 5A1040 log, record skipped (returns null)',
      '4FC340: 858F50 seeks to the next DYNAMIC_THINGS_INDEX offset'],
     'Native logs and skips the record; the map still loads. Port the skip (with the log) instead of stopping.'),
    (r'nonempty Turret source loader', 'ActorData Turret list', 'Performer',
     'aa-runtime/src/actor_world.rs performer_inputs (PORTED in phase8)',
     ['464BD0 Turret children', '4C7660 ctor', '4C7820 LoadData', '45DC30 (+4 read by 57F4D0)', 'B279C0/B279C4 zero vector'],
     'Ported; see native-turrets.json. Records then reach the next stop.'),
    (r'live action contributor/publication after early construction', '4D1350 action-mask rebuild during saved Performer construction',
     'Performer ctor -> 4D1350 contributors (Construction 565140 first for buildings)',
     'aa-runtime/src/actor_world.rs ConstructionActions::Saved; aa-actors/src/ability_flags.rs',
     ['4D1350', '565140 Construction contributor', '4CFCC0 changed-mask publication', '569C70 Denizen', '589A60 Structure', '575FB0 MobileDeployment', '582C40 Recruit', '583FC0 Research', '585AE0 Settlement'],
     'Saved factory context only binds the owner; contributor virtuals and 4CFCC0 publication are unbound.'),
    (r'^map input: unported saved component slot #$', 'saved component reader', 'see slot', 'aa-mapworld/src/saved_components.rs decode_prefix',
     ['slot 4 Mind: Element 536340 (base 52C580), Organization-mind 528C20, Aircraft-mind 51C3C0', 'slot 7 Organization 57D130', 'slot 24 Transport 58C810'],
     'Reader dispatch documented in research/phase7-restoration native_saved_components.py; these slots have no ported codec.'),
    (r'component Character requires completed source input resolved Character view animations', 'Character constructor view/animation binding',
     'Character', 'aa-actors/src/owned.rs; aa-runtime catalog ground_for_animation_set', ['563930 Character saved reader'],
     'Saved Character needs the actual Object variant -> Character view set selection.'),
    (r'component Supply requires a specialized concrete factory', 'Supply factory', 'Supply', 'aa-actors/src/owned.rs construct', [], 'No concrete Supply factory port.'),
    (r'component MobileDeployment requires a specialized concrete factory', 'MobileDeployment factory', 'MobileDeployment', 'aa-actors/src/owned.rs construct', [], ''),
    (r'component City requires a specialized concrete factory', 'City factory', 'City', 'aa-actors/src/owned.rs construct', [], ''),
    (r'detection-memory refresh#D#F# requires native spatial callbacks', 'Actor detection-memory refresh', 'Actor finish', 'aa-mapworld/src/actor_finish.rs', ['4D67F0'], 'Needs spatial services.'),
    (r'completed ground Ability currently requires MAIN_DAMAGE effects', 'Ability effect kinds', 'Performer Ability', 'aa-rules/src/ability_load.rs', ['4BF990 Ability LoadData'], ''),
    (r'Ability effect input kind ENCHANTMENT not implemented', 'Ability ENCHANTMENT effect', 'Performer Ability', 'aa-rules/src/abilities.rs', ['4BF990'], ''),
    (r'Denizen Inventory requires resolved target definitions', 'Denizen nonempty Inventory', 'Denizen', 'aa-actors/src/denizen_state.rs', ['56C320 Denizen reader'], ''),
    (r'Resource IDS ipcs has # matching declarations', 'Kingdom Resource name not in v14 registry', 'Kingdom prefix', 'aa-mapworld/src/special_state.rs; aa-rules resolved_definition', [], 'Developer maps reference resource ipcs; native lookup failure behaviour must be traced.'),
    (r'nonempty strategic entries require native actor/definition binding', 'Kingdom strategic AI entries', 'Kingdom AI', 'aa-mapworld/src/special_state.rs', [], ''),
    (r'ambiguous template ', 'duplicate template declarations', 'definition resolver', 'aa-rules/src/actors.rs', [], 'Native template registry overwrite order unproven.'),
    (r'ambiguous Thing IDS ', 'duplicate Thing IDS declarations', 'definition resolver', 'aa-rules/src/actors.rs', [], 'Native registry overwrite order unproven.'),
    (r'Construction without Economy data', 'Construction zero-cost path', 'Construction', 'aa-rules/src/construction.rs', [], ''),
    (r'Actor name collision/action-table validation', 'Actor name collision validation', 'Actor finish', 'aa-mapworld/src/actor_finish.rs', [], ''),
    (r'Map Load Canceled: City no longer has a city zone', 'City Load zone check (native cancels the whole map)', 'City', 'aa-actors/src/lair_city_state.rs',
     ['564CE0', '45D6A0 (ActorData+5B4 > 0)', '5A0910 KKC_MapResource::ThrowCancelException'],
     'Native behaviour: saved City v1 whose current definition has zone_radius 0 (legacy IDs translated to non-center blocks) cancels the map load.'),
    (r'native component Load rejected saved slot # absent from current definition', 'saved component absent from definition', 'component dispatch', 'aa-mapworld/src/restore_components.rs component_load_action',
     ['4D8290 -> 858A00 diagnostic'], 'Saved slot bit not in the current definition mask (data drift after legacy-ID translation).'),
    (r'OrganizationLayoutData \+#\.\.\+# fields are unavailable', 'Organization LayoutData fields (LayoutInfo producer)', 'Organization',
     'aa-runtime/src/catalog.rs organization_layout_info (RestoreDefinitions)',
     ['57D130 Organization Load (wired: aa-mapworld/src/organization_restore.rs)', 'KKC_OrganizationLayoutData LoadData (+8 IDS, +18 primary, +1C/+20 entries, +28/+2C slot groups, +34/+38 formations, +44/+48, +50)'],
     'Slot 7 is wired with re-entrant 4FAE80 member records; every Organization stops at the missing LayoutData producer (formations/property handles).'),
    (r'Actor current-component pointer binding is unavailable', 'Actor saved current-component pointer', 'Actor finish', 'aa-mapworld/src/actor_finish.rs',
     ['4D8CF0 tail current component'], 'Saved +current component reference must bind to the restored component slot.'),
    (r'component Builder requires a specialized concrete factory', 'Builder factory', 'Builder', 'aa-actors/src/owned.rs construct', [], 'No Builder constructor/Load port.'),
    (r'MobileDeployment v# temporary object payload', 'MobileDeployment v2 temporary object', 'MobileDeployment', 'aa-mapworld/src/saved_components.rs', [], ''),
    (r'missing definition ', 'definition missing from v14 sources (embedded record)', 'record dispatch', 'aa-mapworld/src/bootstrap.rs', [], 'A definition lookup outside DYNAMIC_THINGS (not skipped by 4FC340).'),
]

def annotate(key):
    for pattern, op, comp, site, native, notes in KNOWN:
        if re.search(pattern, key):
            return {'operation': op, 'component': comp, 'rust_site': site, 'native': native, 'notes': notes}
    return {'operation': None, 'component': None, 'rust_site': None, 'native': [], 'notes': 'unannotated'}

groups = []
missing = collections.Counter()
missing_maps = collections.defaultdict(set)
for g in raw['groups']:
    key = g['normalized_error']
    if key.startswith('unsupported native definition: missing definition '):
        for d, n in g['definitions'].items():
            missing[d] += n
        for m in g['maps']:
            missing_maps['missing'].add(m)
        continue
    entry = {'normalized_error': key, 'records': g['count'], 'maps': len(g['maps']), 'first': g['first'],
             'top_definitions': dict(sorted(g['definitions'].items(), key=lambda kv: -kv[1])[:12])}
    entry.update(annotate(key))
    groups.append(entry)
groups.append({'normalized_error': 'unsupported native definition: missing definition <id>',
               'records': sum(missing.values()), 'maps': len(missing_maps['missing']),
               'definitions': dict(missing.most_common()), **annotate('unsupported native definition: missing definition x')})
groups.sort(key=lambda g: -g['records'])
first_stops = []
for m in raw['maps']:
    f = m['failures'][0] if m['failures'] else None
    first_stops.append({'map': m['map'], 'load_error': m.get('load_error'), 'registered': m.get('registered'),
                        'decoded_placements': m.get('decoded_placements'), 'failure_count': len(m['failures']),
                        'first_failure': None if f is None else {k: f[k] for k in ('stage', 'source_ordinal', 'saved_id', 'definition', 'class_code', 'error')}})
out = {
    'generated_by': 'crates/aa-runtime/tests/restore_inventory.rs (InspectPartial) + research/phase8-restoration/build_inventory.py',
    'mode': raw['mode'], 'precision': raw['precision'], 'warning': raw['warning'],
    'map_count': raw['map_count'], 'maps_without_failures': raw['maps_without_failures'],
    'total_failed_records': sum(len(m['failures']) for m in raw['maps']),
    'groups': groups,
    'first_stop_per_map': first_stops,
}
(HERE / 'unsupported-inventory.json').write_text(json.dumps(out, indent=1))
print(json.dumps({'groups': len(groups), 'records': out['total_failed_records']}))
