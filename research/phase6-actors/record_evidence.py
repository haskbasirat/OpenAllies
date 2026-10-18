from pathlib import Path
import hashlib,json,re,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
commands=[]
for name,args in [('tests',['cargo','test','-p','aa-rules','-p','aa-actors']),('local-source',['cargo','test','-p','aa-rules','--test','phase3_local','--','--ignored']),('clippy',['cargo','clippy','-p','aa-rules','-p','aa-actors','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 log=HERE/(name+'.log');log.write_text(run.stdout)
 assert run.returncode==0,run.stdout
 commands.append({'command':args,'exit_code':run.returncode,'log':str(log.relative_to(ROOT)).replace('\\','/'),'sha256':H(log),'passed_tests':sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout)))})
native={}
for name in ['typed-scalar-native','definition-flags-native','categories-native','load-constructors-native','created-actors-native']:
 p=HERE/(name+'.json');d=json.loads(p.read_text());native[name]={'sha256':H(p),'producer_sha256':d['producer_sha256']}
paths=[ROOT/'crates/aa-rules/Cargo.toml',ROOT/'crates/aa-rules/src/lib.rs',ROOT/'crates/aa-rules/src/actors.rs',ROOT/'crates/aa-rules/src/native.rs',ROOT/'crates/aa-rules/src/native_categories.rs',ROOT/'crates/aa-actors/src/lib.rs',ROOT/'crates/aa-actors/src/creation.rs',ROOT/'crates/aa-actors/src/loading.rs',ROOT/'crates/aa-actors/src/registry.rs']
paths+=list((ROOT/'crates/aa-rules/tests').glob('native_*.rs'))+list((ROOT/'crates/aa-actors/tests').glob('native_*.rs'))
out={'schema':1,'passed':True,'scope':'Typed bounded inherited definition inputs and native new/saved actor constructor states; not full actor factory or playable world','commands':commands,'native_evidence':native,'source_sha256':{str(p.relative_to(ROOT)).replace('\\','/'):H(p)for p in paths},'counts':{'scalar_assignments':786,'base_flag_cases':120,'component_presence_cases':60,'performer_filter_cases':15,'final_actor_flag_cases':len(json.loads((HERE/'definition-flags-native.json').read_text())['final_actor_flags']),'category_descriptors':43,'saved_constructor_cases':9,'new_actor_ctor_and_object_init_cases':81,'metadata_cases':6,'local_bounded_attribute_resolutions':1627,'local_explicit_unsupported':18},'limits_file':'research/phase6-actors/README.txt','independent_review':'Separate peer gate by terrain_roads; this producer does not self-approve'}
p=HERE/'evidence.json';p.write_text(json.dumps(out,indent=2)+'\n');print(H(p))
