from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
native=json.loads((HERE/'native-recruit-actions.json').read_text())
assert '2 passed; 0 failed' in (HERE/'tests.log').read_text()
paths=['crates/aa-actors/src/recruit_actions.rs','crates/aa-actors/tests/native_recruit_actions.rs']
result={'schema':1,'date':'2026-10-16','scope':'Native Recruit582C40 action contributor with explicit required eligibility5827B0; not recruitment execution',
        'original_sha256':native['original_sha256'],'v14_sha256':native['v14_sha256'],
        'api':'aa_actors::recruit_actions::contribute(&mut impl World, recruit_storage_owner: ActorId)',
        'cases':len(native['cases']),'inputs':151,'images':['original','v14'],'precisions':['Single24','Double53','Extended64'],
        'native_functions':['582C40','4D14C0','4D0820','529C30','517FA0'],
        'mandatory_gameplay_seam':'5827B0(definition,default_layout,1,0)',
        'state_oracle_sha256':native['state_registration_oracle_sha256'],
        'source_sha256':{p:sha(ROOT/p) for p in paths},
        'evidence_sha256':{p:sha(HERE/p) for p in ['native_recruit_actions.py','native-recruit-actions.json','tests.log']},
        'backup_sha256':sha(HERE/'backups/aa-actors-lib-before-export.rs'),
        'new_files_have_no_prior_version':paths,
        'test_result':{'passed':2,'failed':0,'ignored':0},
        'mutation_order':['commission14','eligibility calls in live list order','recruit15','location17','rally16'],
        'limits_changed':[],
        'explicit_unsupported':['Missing group13 descriptor','Cyclic source list (native does not terminate)','Unbound eligibility/source/runtime callbacks'],
        'later_source_and_saved_adapter_report':'source-adapter-findings.json',
        'remaining':['Recruit eligibility5827B0 concrete service','Live runtime binding','Production execution','Skirmish/multiplayer']}
(HERE/'findings.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'cases':result['cases'],'tests':result['test_result'],'native_fixture_sha256':result['evidence_sha256']['native-recruit-actions.json']}))
