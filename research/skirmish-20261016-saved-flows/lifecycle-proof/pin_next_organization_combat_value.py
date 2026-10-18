from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def pin(path):
    data=path.read_bytes()
    return {'path':str(path.relative_to(ROOT)).replace('\\','/'),'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}
comparison=ROOT/'qa-output/economy-lifecycle-20261016/inventory-comparison.json'
c=json.loads(comparison.read_text())['contexts']['managed']
stats=c['statistics']
assert stats['map_count']==80 and stats['failed_records']==796
assert len(stats['groups'])==1 and stats['groups'][0]['count']==796
assert 'combat value over the restore shells' in stats['groups'][0]['normalized_error']
paths=[ROOT/p for p in (
 'crates/aa-mapworld/src/organization_restore.rs','crates/aa-actors/src/organization_state.rs',
 'crates/aa-actors/src/ground_states.rs','crates/aa-combat/src/organization.rs','crates/aa-combat/src/morale.rs',
 'crates/aa-combat/src/constructors.rs','crates/aa-combat/src/combat_value.rs','crates/aa-combat/tests/native_org_tick.rs',
 'research/phase8-combat/native_org_tick.py','research/phase8-combat/native-org-tick-fixtures.json',
 'crates/aa-mapworld/src/performer_restore.rs','crates/aa-runtime/src/catalog.rs','crates/aa-mapworld/src/restore.rs',
 'crates/aa-runtime/tests/restore_inventory.rs',
 'qa-output/economy-lifecycle-20261016/managed-mode1-inventory.json','qa-output/economy-lifecycle-20261016/inventory-comparison.json')]
paths.extend(HERE/p for p in ('findings.json','next-organization-combat-value.README.txt'))
doc={'schema':1,'date':'2026-10-16','author':'denizen_actions','scope':'Read-only next-blocker implementation plan; not an implementation or parity approval',
 'entry':'aa-mapworld organization_restore.rs OrganizationWorld::combat_value default5788B0 error',
 'existing_port':'aa_combat::organization::combat_value / native5788B0+57DC40',
 'observed_managed_diagnostic':stats,'record_failure_identities_unchanged':c['record_failure_identities_equal_to_standalone_and_previous'],
 'context_limit':'Explicit fresh stopped application/SAI context, mode1, clock0 fixture, InspectPartial. Independent missing true per-record Init/PostCreate/registration remains; not old-world or battle acceptance.',
 'required_bindings':['Actual in-progress Organization members/current saved layout/output words68/6C','Out-of-registry owner shell and generation-checked registered members','OwnedPerformer.word60 and actual Performer owner Body.health/maximum, reread Performer for maximum','Current LayoutInfo.slot_groups[i] then saved layout.parameters[group], preserving repeated groups/null parameters','Canonical completed ActorData+174bit5/+1E4; no second Translate and no default/uncomputed substitute','Explicit current arithmetic precision; separate native source-producer precision/context'],
 'risks':['General combat Organization differs from saved OrganizationState; avoid fake full worlds/default fields','Success-only snapshot copyback loses resets and earlier partial stores on error','Eager all-slot preflight moves failures before native zero/stores','Constructorzero Performer.word60 is not evidence of missing true Init/refresh completion','Completed source1E4 arena is not exposed by inspected Catalog bindings; computational pure ports need real source ownership','No mutation/reentry assumption may be hidden by eager component snapshots','Diagnostic failures can cascade'],
 'recommended_bridge':'Narrow borrowed output/read interface shared by proven arithmetic wrapper and saved adapter; alternatively only a rigorously bounded two-output projection committed on every Result path with no reentrant reads',
 'source_lifecycle_gap':'Performer constructor sets word60=0; saved57F970 application does not write it. Actual native refresh and per-record lifecycle completion remain necessary independently.',
 'actions_performed':{'production_edits':False,'new_oracles':False,'cargo_tests_or_builds':False,'game_or_network_test':False,'release_changes':False,'read_only_source_and_existing_evidence_inspection':True},
 'pins':[pin(p) for p in paths]}
out=HERE/'next-organization-combat-value.json';out.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'plan_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'pinned_files':len(paths)}))
