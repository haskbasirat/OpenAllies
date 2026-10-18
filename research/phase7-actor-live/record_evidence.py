from pathlib import Path
import json,hashlib,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
commands=[]
for name,args in [('tests.log',['cargo','test','-p','aa-actors','--test','native_ability_flags','--test','native_state_effects','--test','native_property_applicability']),('property-tests.log',['cargo','test','-p','aa-body','--test','native_property_flags','--test','native_property_effects']),('clippy.log',['cargo','clippy','-p','aa-actors','-p','aa-body','--all-targets','--','-D','warnings'])]:
 r=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/name).write_bytes(r.stdout);assert r.returncode==0,r.stdout.decode(errors='replace');commands.append({'command':args,'exit':0,'log':str((HERE/name).relative_to(ROOT)),'sha256':H(HERE/name)})
fixtures=['native-ability-flags.json','native-organization-status.json','native-state-effects.json','native-property-applicability.json','native-property-flags.json']
producers=['ability_flags.py','organization_status.py','state_effects.py','property_applicability.py','property_flags.py']
for fixture,producer in zip(fixtures,producers):
 d=json.loads((HERE/fixture).read_text());assert d['producer_sha256']==H(HERE/producer)
files=['crates/aa-actors/src/ability_flags.rs','crates/aa-actors/src/state_effects.rs','crates/aa-actors/tests/native_ability_flags.rs','crates/aa-actors/tests/native_state_effects.rs','crates/aa-actors/tests/native_property_applicability.rs','crates/aa-body/src/property_flags.rs','crates/aa-body/src/property_effects.rs','crates/aa-body/tests/native_property_flags.rs','crates/aa-body/tests/native_property_effects.rs']
files +=[str((HERE/p).relative_to(ROOT))for p in fixtures+producers+['README.txt','record_evidence.py']]
deps=['crates/aa-actors/src/state.rs','crates/aa-actors/src/ground_states.rs','crates/aa-body/src/properties.rs','crates/aa-numeric/src/lib.rs','research/phase7-actors/actor_states.py','research/phase7-properties/original_effects.py','research/phase7-properties/original_modifier_load.py','research/phase7-properties/original_properties.py']
report={'schema':1,'passed':True,'scope':'Live actor actions/state/property effects; bounded required world callbacks, not full publication or playable game','native_cases':{'ability':290,'organization_status':276,'state_effects':192,'recursive_applicability':128,'mixed_property_sequences':111,'mixed_property_transitions':246,'flag_loaders':17,'flag_registrations':12,'numeric_regression_sequences':111,'numeric_regression_transitions':390},'focused_tests':12,'commands':commands,'source_sha256':{p.replace('\\','/'):H(ROOT/p)for p in files},'dependency_sha256':{p:H(ROOT/p)for p in deps},'scope_note':'Generic property lifetime extraction preserves previous numeric API/algorithm. Reviewed or historical evidence is not silently updated by this author ledger. External callback/seam limitations are explicit in README and each fixture.','independent_review':'Required separately.'}
p=HERE/'actor-live-evidence.json';p.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(H(p))
