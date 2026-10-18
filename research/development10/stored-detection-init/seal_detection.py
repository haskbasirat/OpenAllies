from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(HERE));import native_after_fields as n
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-after-fields.json').read_text());md=Cs(CS_ARCH_X86,CS_MODE_32);pins=[]
for at,code in doc['executed_pins'].items():
 at=int(at,16);code=bytes.fromhex(code);assert n.h.P.range(at,len(code))==n.h.V.range(at,len(code))==code
 decoded=list(md.disasm(code,at));assert len(decoded)==1 and decoded[0].size==len(code)
 i=decoded[0];pins.append(dict(address=hex(at),bytes=code.hex(),instruction=i.mnemonic+' '+i.op_str))
assert len(pins)==54
assert '1 passed; 0 failed' in (HERE/'core-tests.log').read_text()
assert 'Finished `release`' in (HERE/'check-01.log').read_text()
owned=[ROOT/'crates/aa-runtime/src'/x for x in ['stored_detection_init_world.rs','detection_after_core.rs','detection_after_core_tests.rs']]
shared=[ROOT/'crates/aa-runtime/src'/x for x in ['load_detection_activation.rs','stored_component_schedule.rs','map_actor_init_existing.rs','lib.rs']]
evidence=[HERE/x for x in ['native_after_fields.py','native-after-fields.json','native-after-generation.log','after-native-replay.log','shared-schedule-native-replay.log','exact-core-tests.rs','exact-core-tests.exe','core-build.log','core-tests.log','check-01.log']]
reference=ROOT/'research/skirmish-20261016-load-detection-init/native-scheduled-init.json'
result=dict(schema=1,status='bounded helper/source freeze; actual fresh graph advance pending',
 implementation='New specialized Construction/Detection/Lair InitExisting calls unchanged shared564D80 coordinator over same stored_component_schedule/Staged version/Owners/Queues/RNG. DetectionAfter shares one new storage-independent front kernel with existing held API; fresh adapter creates/borrows same Actor zone6 and delegates actual stored_zone_activation solver with same World248/identities/Bank. No detached runtime, copied zone, reconstructed World, default version, or readiness bit.',
 native=dict(shared_init_existing_paired_rows=90,shared_runs=180,shared_executed_pins=14,shared_required_entry_pins=1,shared_fixture_sha256=H(reference),
 after_front_paired_rows=96,after_front_runs=192,after_front_executed_pins=54,after_front_decoded=pins,
 child_scope='Front56EC90 stops at actual56EB40/4D69A0 child entry or before attached Data430 read. It supplies no successful child result. Real529C30 getter executes. Original/v14/all3PC equality, actual state100->Actor138, ordinary flags then conditional radius comparison and radius reread for child argument.'),
 rust=dict(exact_current_after_core_tests=1,rows=96,production_check='PASS33.57s',
 qualifier='Front probe callbacks stop with required errors; they cannot mint successful zone/solver state. Existing shared564D80 Rust kernel and all existing zone kernels are reused. New fresh storage binding has not yet been exercised by a complete genuine startup result.'),
 corrections='The old held adapter cached radius before flags. New common kernel follows actual native state100/Actor138/flags/radius reads; finite admitted outputs are unchanged, and lazy getter/error prefixes are now native-ordered.',
 limits=['Nonzero ordinary Actor138 keeps source430/Denizen/scheduling required rather than inventing a null guard.','State author owns positive SAI/reveal/component-effect callbacks in actual stored_zone_activation; missing reached services retain zone allocation/prefix.','No whole Actor Init, mission, View completion or playable-battle certificate.','Nonfinite comparisons remain outside the existing aa-zones finite input domain.'],
 owned_source_sha256={str(p.relative_to(ROOT)):H(p)for p in owned},shared_whole_file_snapshot_only_sha256={str(p.relative_to(ROOT)):H(p)for p in shared},evidence_sha256={str(p.relative_to(ROOT)):H(p)for p in evidence})
path=HERE/'findings.json';path.write_text(json.dumps(result,indent=1)+'\n');print(json.dumps(dict(findings_sha256=H(path),owned_source=result['owned_source_sha256'])))
