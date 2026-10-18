from pathlib import Path
import json,re,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
files=['uvars.rs','view_variables.rs','view_variables_tests.rs','view_variable_registration.rs']
pins={str(p.relative_to(ROOT)).replace('\\','/'):sha(p)for p in [ROOT/'crates/aa-ui/src'/f for f in files]}
native=[]
for stem in ['uvar-registration','view-uvar-reset','view-bool-assignment']:
 p=HERE/f'native-{stem}.json';script=HERE/('native_'+stem.replace('-','_')+'.py')
 native.append({'fixture':p.name,'sha256':sha(p),'producer':script.name,'producer_sha256':sha(script)})
logs=['ui-all-tests-v1.log','ui-lib-tests-final.log','registration-replay.log','reset-replay.log','bool-replay.log']
counts=[int(v)for v in re.findall(r'test result: ok\. (\d+) passed;', (HERE/logs[0]).read_text())]
assert sum(counts)==55 and counts[0]==21
assert '22 passed; 0 failed' in (HERE/logs[1]).read_text()
doc={'schema':1,'status':'author frozen; bounded current UVar reset child only',
 'source_files':pins,'native':native,'mutable_indices':[22,24,25,27,29,44,100,101],
 'overlap':'CameraInformation forwards SAME aa-ui UVars cell; no parallel current/default string entries; other seven previously unowned',
 'tests':{'full_ui_before_added_oracle':55,'final_current_library':22,'distinct_library_plus_integration':56,
 'native_metadata_runs':6,'native_reset_rows':84,'native_assignment_rows':288,
 'actual_data':'three root UVars files, same CameraInformation byte, profile roundtrip and lazy reset'},
 'logs':[{'path':v,'sha256':sha(HERE/v)}for v in logs],
 'binary':{'path':'ui-tests-final.exe','sha256':sha(HERE/'ui-tests-final.exe')},
 'remaining_required':['whole43F0A0 resource/scene/terrain/camera/final-manager graph',
 'actual Screen/User/ResourceManager/Input owners before reset','positive editor source overrides with actual developer gate',
 'actual application/source file identity validation at concrete client assembly',
 'resource task queue/barrier and Loading17 callback','actual GPU/GUI acceptance','Session status3 and unfreeze'],
 'not_claimed':['VWorld AD or view-ready capability','playable battle','whole native source/profile loading'],
 'preservation':'Original executables/assets/saves/profiles unchanged; no vendor source integrated.'}
p=HERE/'findings.json';p.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'findings_sha256':sha(p),'source_files':pins,'binary_sha256':doc['binary']['sha256']}))
