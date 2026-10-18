"""Pin bounded same-owner organization creation integration; no battle mint."""
from pathlib import Path
import hashlib,json,shutil,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def pin(p):
    path=ROOT/p
    return dict(path=str(path.relative_to(ROOT)).replace('\\','/'),sha256=hashlib.sha256(path.read_bytes()).hexdigest())
owned=[
 'crates/aa-runtime/src/stored_organization_spawn.rs',
 'crates/aa-runtime/src/stored_organization_spawn_fields.rs',
 'crates/aa-runtime/src/stored_organization_spawn_fields_tests.rs',
]
shared=[
 'crates/aa-mapworld/src/fresh_creation.rs',
 'crates/aa-runtime/src/map_actor_creation.rs',
 'crates/aa-runtime/src/fresh_organization_completion.rs',
 'crates/aa-runtime/src/stored_organization_members.rs',
 'crates/aa-runtime/src/stored_member_deployment.rs',
 'crates/aa-runtime/src/member_deployment_fields.rs',
 'crates/aa-runtime/src/actor_world.rs',
 'crates/aa-actors/src/organization_spawn.rs',
 'crates/aa-actors/src/world_creation.rs',
 'crates/aa-actors/src/actor_creation.rs',
 'crates/aa-spatial/src/rectangle_actors.rs',
]
check=Path(sys.argv[1]) if len(sys.argv)>1 else HERE/'check-06-tests.log'
if not check.is_absolute():check=ROOT/check
log=check.read_text()
assert 'Finished `release` profile' in log and 'could not compile'not in log
assert '2 passed; 0 failed'in(HERE/'fields-tests.log').read_text()
slot=json.loads((HERE/'native-spawn-slot.json').read_text());notify=json.loads((HERE/'native-notify.json').read_text())
assert len(slot['rows'])==720 and len(slot['pins'])==108
assert len(notify['rows'])==168 and len(notify['pins'])==63
pins=dict(slot['pins'])
for a,b in notify['pins'].items():assert a not in pins or pins[a]==b;pins[a]=b
snap=HERE/'source-v1';snap.mkdir(exist_ok=True)
for p in owned+shared:
    destination=snap/Path(p).name
    if destination.exists():assert destination.read_bytes()==(ROOT/p).read_bytes(),str(destination)
    else:shutil.copyfile(ROOT/p,destination)
dependencies=[
 'research/development10/organization-spawn/native-create-member.json',
 'research/development10/organization-spawn/native-rectangle-actors-v2.json',
 'research/development10/organization-spawn/native-member-slot-writeback.json',
 'research/skirmish-20261017-fresh-organization-completion/findings.json',
 'research/skirmish-20261017-member-deployment/findings.json',
 'research/skirmish-20261017-member-deployment/native-actor-shape-getter.json',
 'research/skirmish-20261017-member-deployment/point2-findings.json',
]
artifacts=[
 'native_spawn_slot.py','native-spawn-slot.json','native-replay.log',
 'native_notify.py','native-notify.json','notify-replay.log',
 'fields-tests.rs','fields-tests.exe','fields-tests-build.log','fields-tests.log',
 'check-01.log','check-02.log','check-03.log','check-04-tests.log',
 'disassemble.py','disassembly.txt','README.md',
]
doc=dict(schema=1,
 status='Bounded production adapter and field kernels checked; genuine full same-world member spawn remains unobserved.',
 scope='Same staged Organization completion, raw57AD30 stack creation, live4FBF40 queries and placement writeback, existing57D950 creation/membership, current578040 guard/Kingdom refresh, and bounded real hero table postpass.',
 owned_sources=[pin(p)for p in owned],shared_successor_snapshots=[pin(p)for p in shared],
 shared_qualification='Whole shared-file hashes are context snapshots, not exclusive ownership or full-file acceptance. Parent owns live query/placement kernel changes; State owns add-member; SavedLeader owns completion/hero/deployment constituents.',
 native=dict(new_rows=888,new_executions=1776,images=['original','v14'],precisions=['Single24','Double53','Extended64'],distinct_executed_byte_pins=len(pins),
   slot=dict(rows=720,executions=1440,pins=108,scope='Whole57AD30 through explicit4FBF40 child; native inline constructor fields/raw51/occupied/late-cutoff behavior; CString/vtable addresses and unwritten53 padding excluded.'),
   notify=dict(rows=168,executions=336,pins=63,scope='Whole578040 current read/guard order through explicit4E14E0/statistics/classification/SDK child transports and cutoffs.'),
   qualification='Synthetic child cutoffs represent required service boundaries, not native C++ exception unwinding. Explicit child-entry bytes are excluded from executed counts. No native game process is launched by these producers.'),
 rust=dict(exact_production_field_tests='2/2 passed after final field interface/formatting;720+168 native rows replayed.',
   production_typecheck=pin(str(check.relative_to(ROOT))),
   support='Small exact-source field harness links pinned warm aa-actors ActorParams and JSON support. No full current runtime/client executable link or end-to-end World acceptance is claimed.',
   support_libraries=[pin('target/native-runtime/release/deps/libaa_actors-da01587d98fcfe3e.rlib'),pin('target/native-runtime/release/deps/libserde_json-3f0b3bb60c709793.rlib')]),
 dependencies=[pin(p)for p in dependencies],
 dependency_qualification='Existing child receipts are cited separately; their runs are not included in the new1,776 execution count. Native getter static/previous-constructor pins remain qualified by their original receipt.',
 artifacts=[pin(str((HERE/p).relative_to(ROOT)))for p in artifacts],
 invariants=[
  'The same Staged/ActorStore/World/spatial index/source Bank/queues/RNG are borrowed; no object, installed component, live pose or candidate list is cloned or reconstructed.',
  'CompletionWorld is a narrow service extraction from the unchanged completion body; old EntryWorld callers delegate through its blanket implementation. No unrelated assertion latch default is invented.',
  'The captured Organization Box is checked by nonowning allocation identity; current layout/parameters are read at each native boundary. Fresh actor component reads remain distinct where native rereads actor+7C.',
  'Raw ActorParams51 preserves noncanonical byte payloads. Native stack parameter blocks are separate real local owners, not snapshots of mutable world owners.',
  'Live Cursor begin/next/finish delegate the parent helpers over the actual current index. Native marks survive callbacks and structure creation; no Vec candidate shortcut is used.',
  'Candidate3EC precedes lazy canonical17C membership. Current actor position, owner and Kingdom/Team relation fields are borrowed at their existing call points.',
  'Successful578290 writes back actual XY for57D950; copied stack facing remains the incoming value. Runtime uses the same existing native placement algorithm.',
  'Real4F9C10 allocation/init/register and same installed State57CBA0 add-member run in order. Positive unbound recruit slots never certify a working item arena or successful pairing.',
  'ActorD8 uses the actual Hq component slot30, not an invented numeric pointer word. Kingdom4E14E0 captures count once and rereads one current2E8 actor per callback before actual action rebuild.',
  '578040 retains current state1 guard before positive statistics, then actual SDK-presence and repeated Actor154 reads. Missing positive owners fail at reached services.',
  'Hero pass reads actual LayoutSlotEntry.number, current members, captured Bank token, distinct owner108 calls and same Kingdom find_single(token,true). No assumed empty table or default hero count.',
  'Source preflight validates the same moved Bank universe/precision. Valid managed-graph behavior is the admitted domain, not malformed native-pointer fault parity.',
  'No new fixed memory/VRAM/bandwidth/list budget or fake native ABI pointer is introduced.'
 ],
 remaining_required=[
  'Actual full organization/member creation and mission/battle acceptance await the next genuine strict startup run; typechecking and bounded fields do not mint StartedMission/readiness.',
  'Positive recruit-item pairing4CEA60, hero attachment57B830/localized notification, new member Mind198 deployment, positive statistics/SDK effects and real destruction retain explicit required errors.',
  'Actual source point2 parser/deployment-source successors and other authors add-member callbacks are separately owned and qualified.',
  'Source existing57D950/create/completion/hero/placement/query receipts prove their bounded kernels; this adapter does not claim every child has completed.'
 ])
out=HERE/'findings.json';out.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=hashlib.sha256(out.read_bytes()).hexdigest(),new_native_executions=1776,exact_field_tests=2,distinct_executed_pins=len(pins))))
