from pathlib import Path
import csv,hashlib,json,subprocess
import native_admission as native
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-admission.json').read_text());assert doc['producer_sha256']==sha(HERE/'native_admission.py') and len(doc['cases'])==60
for w in doc['visited_windows']:
 raw=bytes.fromhex(w['bytes']);a=int(w['address'],16);assert native.IMAGE.at(a,len(raw))==native.ACCEPTED.at(a,len(raw))==raw
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')};ranges=[]
for a in [0x4cef10,0x541fe0,0x5817a0,0x5030f0]:
 raw=native.IMAGE.at(a,sizes[a]);assert raw==native.ACCEPTED.at(a,len(raw));ranges.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
commands=[]
for log,args in [('admission-tests.log',['cargo','test','-p','aa-body','--test','native_property_admission']),('admission-clippy.log',['cargo','clippy','-p','aa-body','--test','native_property_admission','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/log).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace');commands.append({'command':args,'exit_code':run.returncode,'log':log,'sha256':sha(HERE/log)})
files=['crates/aa-body/src/property_admission.rs','crates/aa-body/tests/native_property_admission.rs']+[f'research/phase7-properties/{p}' for p in ['native_admission.py','native-admission.json','record_admission.py','ADMISSION.txt']]
report={'schema':1,'passed':True,'scope':'Actor4CEF10 admission and541FE0 technology wrapper; actual5817A0/5030F0 initialization compared at insertion boundary.','native_cases':60,'rust_tests':2,'original_sha256':doc['original_sha256'],'v14_sha256':doc['v14_sha256'],'ranges':ranges,'commands':commands,'source_sha256':{p:sha(ROOT/p) for p in files},'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-body/src/property_effects.rs','research/phase7-properties/original_properties.py','research/phase4-body/original_body.py']},'seams':doc['seams'],'limitations':['Applicability and mixed insertion require concrete runtime binding.','Null component removal during callback is explicit error; original invalid pointer operation is outside parity.','Callback failure retains prior mutations; detached/inserted instance ownership belongs to runtime.'],'review':'Separate independent peer review pending.'}
path=HERE/'admission-findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(path)}))
