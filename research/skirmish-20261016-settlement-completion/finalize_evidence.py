"""Record this bounded port's reproducible evidence pins; no source changes."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(path):
    p=ROOT/path
    return {'path':path,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size}
native=json.loads((HERE/'native-prefix.json').read_text())
branch=json.loads((HERE/'native-presence-branch.json').read_text())
hardware=json.loads((HERE/'hardware-trig.json').read_text())
census_doc=json.loads((HERE/'real-source-census-v2.json').read_text());census=census_doc['census']
assert len(native['prefix_cases'])==384 and len(native['wall_set_cases'])==192 and len(native['initial_radius_cases'])==54
assert len(branch['cases'])==12 and len(hardware['cases'])==96
assert census['actor_registrations']==1028 and len(census['registered_actors'])==1028
assert len(census['sources'])==1647 and census['wall_set_registrations']==0 and census['settlement_actors']==[]
assert len(census['source_diagnostics'])==11
assert all(d['message'].startswith('native syntax notice ') for d in census['source_diagnostics'])
assert '4 passed; 0 failed' in (HERE/'tests-accepted.log').read_text()
assert '18 passed; 0 failed' in (HERE/'rules-compatibility.log').read_text()
assert 'Finished `release`' in (HERE/'clippy.log').read_text()
paths=['crates/aa-rules/src/settlement_completion.rs','crates/aa-rules/tests/settlement_completion.rs']
paths += [str(p.relative_to(ROOT)).replace('\\','/') for p in sorted(HERE.iterdir()) if p.is_file() and p.name not in ['findings.json']]
out={'schema':1,'passed':True,'scope':'New bounded Settlement gameplay-shape prefix and source dependency census; no complete source arena or battle startup.','native_images':{k:native[k] for k in ['original_sha256','v14_sha256']},'counts':{'native_shared_prefix':384,'native_wall_set_validation':192,'native_initial_radius':54,'native_optional_child_branch':12,'hardware_trig_repeated':96,'scoped_rust_tests':4,'existing_rules_library_tests_including_ignored':18,'registered_kind2_actors':1028,'source_files_hash_checked':1647,'settlement_actor_registrations':0,'wall_set_registrations':0,'native_syntax_notices':11,'fatal_or_truncating_source_exceptions':0},'source_changes':['New aa-rules/settlement_completion module and integration test','Append one pub mod settlement_completion line after rereading shared lib.rs','No parser/definition-loading implementation, manifests, lockfile, runtime, original assets/executables/saves/profiles or releases modified'],'clippy':'ordinary library Clippy passed;5 existing warnings in unchanged native_load/native_display_name; no strict-clean or allowance claim','critical_path_correction':'No registered SettlementComponent is present in current1028 source registrations, and native optional-child branch skips4640F0 when absent.460940 fitting is not established as a reached shipped-data blocker. Existing saved adapter still explicitly rejects uncompleted canonical ActorData+1E4.','remaining':['Complete reached CV settings/profile/means/shared source graph in original order and explicit source-load PC/CRT environment','Historical load-time WallSet reference identity, if future sources actually use it; current source API is immutable current-registry inputs','Full4640F0/460940 fitting, CRT behavior and wall-piece placement, if reached','Source+1E4/+30C/+36C publication and live Performer+60','Immediate per-record Init/PostCreate, required world services and complete battle startup'],'hardware_limit':'Physical Intel Family6 Model167 x87 FSIN/FCOS raw80 reference on this machine, not software trig or cross-CPU/application-source-PC proof','independent_review':pin(str((HERE/'independent-review.json').relative_to(ROOT)).replace('\\','/')),'files':[pin(p) for p in paths]}
(HERE/'findings.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
print('frozen findings SHA256',hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest())
print('source SHA256',out['files'][0]['sha256'],'tests SHA256',out['files'][1]['sha256'])
