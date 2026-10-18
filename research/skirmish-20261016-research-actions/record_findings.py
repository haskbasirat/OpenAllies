from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
native=json.loads((HERE/'native-research-actions.json').read_text())
assert '1 passed; 0 failed' in (HERE/'tests.log').read_text()
assert '1 passed; 0 failed' in (HERE/'adapter-tests.log').read_text()
files=['crates/aa-actors/src/research_actions.rs','crates/aa-actors/tests/native_research_actions.rs','crates/aa-runtime/src/saved_research_actions.rs']
reused=['crates/aa-actors/src/state.rs','crates/aa-actors/src/ability_flags.rs','crates/aa-actors/src/research_state.rs','crates/aa-rules/src/native_recruit_data.rs','crates/aa-rules/src/construction.rs','research/phase8-restoration/native-research.json','research/skirmish-20261016-recruit-actions/native-recruit-source.json']
result={
    'schema':1,'date':'2026-10-16','scope':'Native Research583FC0 action35 contributor and saved-record adapter; not research execution',
    'api':['aa_actors::research_actions::contribute','aa_runtime::saved_research_actions::contribute [crate-private]'],
    'original_sha256':native['original_sha256'],'v14_sha256':native['v14_sha256'],
    'native_functions':['583FC0','4D14C0','4D0820','517FA0'],'cases':len(native['cases']),'inputs':92,
    'images':['original','v14'],'precisions':['Single24','Double53','Extended64'],'gameplay_seams':[],
    'state_registration_oracle_sha256':native['state_registration_oracle_sha256'],
    'source_sha256':{p:sha(ROOT/p) for p in files},'reused_source_or_evidence_sha256':{p:sha(ROOT/p) for p in reused},
    'evidence_sha256':{p:sha(HERE/p) for p in ['native_research_actions.py','native-research-actions.json','oracle.log','tests.log','adapter-tests.log']},
    'backups_sha256':{p.name:sha(p) for p in sorted((HERE/'backups').iterdir())},
    'tests':{'contributor':{'passed':1,'failed':0},'actual_saved_adapter':{'passed':1,'failed':0}},
    'runtime_probe':'Actual tutorial02 saved records; native mode1, PC53, InspectPartial; explicit state/action-mask and missing/empty definition-list probe inputs',
    'mutation_order':['selected group13 type read','conditional ActorData+558 head read','action low word+F0','action high word+F4'],
    'limits_changed':[],
    'explicit_unsupported':['Missing group13 descriptor','Missing required completed source ResearchItem list','Missing concrete component/actor/source-state payload'],
    'not_claimed':['Generic ResearchItem definition loader parity beyond existing provider','Complete saved-world restoration','Live research execution','Skirmish/multiplayer'],
}
(HERE/'findings.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'cases':result['cases'],'tests':result['tests'],'source_sha256':result['source_sha256'],'fixture_sha256':result['evidence_sha256']['native-research-actions.json']},indent=2))
