from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
native=json.loads((HERE/'native-saved-values.json').read_text())
ctor=json.loads((HERE/'native-absent-economy-constructor.json').read_text())
actorslog=(HERE/'actors-tests.log').read_text();runlog=(HERE/'runtime-tests.log').read_text()
assert '1 passed; 0 failed' in actorslog and '2 passed; 0 failed' in actorslog
assert '1 passed; 0 failed' in runlog
assert 'compared 518 native saved value cases over 46 actors' in runlog
assert sha(HERE/'actual-saved-input.json')==native['captured_input_sha256']
files=['crates/aa-runtime/src/saved_economy_values.rs','crates/aa-runtime/src/saved_economy_values_tests.rs',
       'crates/aa-actors/src/economy_values.rs','crates/aa-actors/tests/economy_lazy_backref.rs']
reused=['research/phase7-economy-values/native-values.json','research/phase7-economy-values/native-source.json',
        'research/phase7-economy-values/native_values.py','crates/aa-rules/src/native_recruit_backrefs.rs',
        'crates/aa-rules/src/native_mobile_deployment.rs','crates/aa-actors/src/transport_state.rs',
        'crates/aa-actors/src/supply_state.rs']
evidence=['actual-saved-input.json','native-saved-values.json','native-absent-economy-constructor.json',
          'native_saved_values.py','native-saved.log','actors-tests.log','runtime-tests.log']
report={'schema':1,'date':'2026-10-16','scope':'Actual saved Economy native values/caches and lazy17C source binding; not full570020',
 'api':'View::new(&mut SavedActions,&mut EconomyState,&mut[(ActorId,&mut EconomyState)],Precision)',
 'methods':['upkeep_blocked','provides','cost_or_upkeep','production','parent_organization'],
 'original_sha256':native['original_sha256'],'v14_sha256':native['v14_sha256'],
 'actual_native_cases':len(native['cases']),'actual_source_precision':'Double53',
 'actual_source_actors':len(native['input']['actors']),'actual_source_definitions':len(native['input']['definitions']),
 'constructor_native_cases':len(ctor['cases']),'constructor_precisions':['Single24','Double53','Extended64'],
 'tests':{'new_lazy':1,'existing_native':2,'actual_saved_adapter':1,'failed':0},
 'source_sha256':{p:sha(ROOT/p) for p in files},'reused_sha256':{p:sha(ROOT/p) for p in reused},
 'evidence_sha256':{p:sha(HERE/p) for p in evidence},
 'backups_sha256':{p.name:sha(p) for p in (HERE/'backups').iterdir()},
 'ownership':'Actual focused/held Economy boxes; explicit presence and same-cache mutation ahead of registry; integrator restores same Box on Result',
 'source_correction':'501730 reads recruitment backrefs+17C lazily, not transport_actor+184',
 'native_seams':['allocator/free','pure4CF110 Properties-context identity','nonnull owner fixture and stop at end of570020 inline upkeep gate'],
 'limits_changed':[],
 'remaining':['Canonical Kingdom flow initialization/full570020','Full world publication/simulation','Skirmish/multiplayer','Malformed-storage fault timing/allocation exhaustion'],
 'preserved_failures':['capture-no-org-economy.log','native-v14-overlay-failure.log'],
 'broad_compatibility':'Root coordinates; not claimed by this scoped run'}
(HERE/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'tests':report['tests'],'actual_cases':report['actual_native_cases'],'source_sha256':report['source_sha256'],'fixture_sha256':report['evidence_sha256']['native-saved-values.json']},indent=2))
