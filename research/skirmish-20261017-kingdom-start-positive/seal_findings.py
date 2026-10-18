"""Pin the completed bounded positive-Commander author cohort."""
from pathlib import Path
import json,hashlib,re
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
files=['kingdom_mission_start.rs','kingdom_mission_start_tests.rs','kingdom_start_source.rs',
 'load_mission_variables.rs','load_view_variables.rs',
 'load_view_variables_tests.rs','load_property_instances.rs','actor_property_focus.rs']
pins={str(p.relative_to(ROOT)).replace('\\','/'):sha(p)for p in [ROOT/'crates/aa-runtime/src'/f for f in files]}
binary=HERE/'commander-tests-final-v4.exe'
logs=[HERE/'tests-final-v4.log',*sorted(HERE.glob('regression-final-v4-*.log'))]
tests=[]
for p in logs:
 m=re.search(r'test result: ok\. (\d+) passed; (\d+) failed;',p.read_text())
 assert m and int(m[2])==0,p
 tests.append({'path':p.name,'passed':int(m[1]),'sha256':sha(p)})
assert sum(x['passed']for x in tests)==24
capture=HERE/'actual-source-census-final-v4.json';data=json.loads(capture.read_text())
assert len(data['factions'])==24
assert all(not f['property_tokens']for f in data['factions'])
native=[]
for stem in ['starting-experience','faction-properties','commander-start']:
 p=HERE/f'native-{stem}.json';producer=HERE/('native_'+stem.replace('-','_')+'.py')
 native.append({'fixture':p.name,'sha256':sha(p),'producer':producer.name,'producer_sha256':sha(producer)})
doc={'schema':1,'status':'author-frozen bounded positive-Commander path; no whole mission claim',
 'binary':{'path':binary.name,'sha256':sha(binary)},'source_files':pins,'tests':tests,
 'passed':24,'native':native,'actual_capture':{'path':capture.name,'sha256':sha(capture),
 'factions':24,'starts':72,'source_precision':'Double53','live_precisions':['Single24','Double53','Extended64'],
 'positive_compatible_nations':sum(f['nation']is not None for f in data['factions']),
 'all_faction_property_lists_empty':True},
 'scope':['same actual Commander experience owner','same current StartingExperience SVar State',
 'same moved Bank source identity','same Property State and cached shared-loan semantics',
 '384 original/v14 native positive/prefix rows','19 additional bounded regression tests'],
 'remaining_required':['positive Faction Property grants and actual game event service',
 'positive start profile/start technology grant adapter','invalid-Faction localization service',
 'successful fresh selected full mission','playable battle and real multiplayer acceptance'],
 'historical':['v1 constructor-null-Nation source capture','pre-pin-correction faction fixture excludes later correction'],
 'preservation':'Original executable/assets/saves/profiles were not modified. No vendor source integrated.'}
p=HERE/'findings.json';p.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'passed':doc['passed'],'findings_sha256':sha(p),'binary_sha256':sha(binary),
 'capture_sha256':sha(capture),'source_files':pins}))
