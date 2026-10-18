from pathlib import Path
import csv,hashlib,json,os,struct,subprocess
import native_names as native
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-names.json').read_text());assert len(doc['cases'])==11 and sum(len(c['output']) for c in doc['cases'])==2569
assert doc['producer_sha256']==sha(HERE/'native_names.py')
for w in doc['visited_windows']:
 a=int(w['address'],16);raw=bytes.fromhex(w['bytes']);assert native.IMAGE.at(a,len(raw))==native.ACCEPTED.at(a,len(raw))==raw
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')};ranges=[]
for a in [0x4f7d00,0x4f94d0,0x4f9510,0x4f7b70,0x5c9180,0x593570,0x4154d0,0x5043f0,0x405030,0x4f8e90,0x4f8f00,0x4bedf0]:
 raw=native.IMAGE.at(a,sizes[a]);assert raw==native.ACCEPTED.at(a,len(raw));ranges.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
class_pins=[]
for table in [0xa273f0,0xa299d8]:
 for offset in [0xa8,0xb4]:
  raw=native.IMAGE.at(table+offset,4);assert raw==native.ACCEPTED.at(table+offset,4);address=struct.unpack('<I',raw)[0];code=native.IMAGE.at(address,3);assert code==native.ACCEPTED.at(address,3)
  class_pins.append({'table':hex(table),'offset':hex(offset),'function':hex(address),'bytes':code.hex()})
commands=[];env={**os.environ,'AA_GAME_ROOT':str(ROOT.parent)}
for log,args in [('tests.log',['cargo','test','-p','aa-runtime','--test','names_native','--test','trigger_world','--','--include-ignored']),('clippy.log',['cargo','clippy','-p','aa-runtime','--test','names_native','--test','trigger_world','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/log).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace');commands.append({'command':args,'exit_code':run.returncode,'log':log,'sha256':sha(HERE/log)})
sources=['crates/aa-runtime/src/names.rs','crates/aa-runtime/src/trigger_world.rs','crates/aa-runtime/tests/names_native.rs','crates/aa-runtime/tests/trigger_world.rs']+[f'research/phase7-runtime-names/{p}'for p in ['README.txt','native_names.py','native-names.json','record_findings.py']]
deps=['crates/aa-runtime/src/actor_world.rs','crates/aa-actors/src/registry.rs','crates/aa-triggers/src/world_program.rs','research/phase4-body/original_body.py']
report={'schema':1,'passed':True,'scope':'Native World name-table mutation/lookup/rehash and concrete shared-store trigger selectors/field views. Full trigger service binding is incomplete.','native_sequences':11,'native_transitions':2569,'rust_tests':3,'original_sha256':doc['original_sha256'],'v14_sha256':doc['v14_sha256'],'ranges':ranges,'class_vtables':class_pins,'commands':commands,'source_sha256':{p:sha(ROOT/p)for p in sources},'dependency_sha256':{p:sha(ROOT/p)for p in deps},'seams':doc['seams'],'limitations':['ASCII noembedded-null name domain, CRT C-locale compare.','Absent-name pointer/GTID accidental alias is outside valid logical domain.','Cross-name hash traversal is not exposed; equal-name ordering including rehash is preserved.','Publication name branch only; full4F9510 class/spatial registration remains caller-owned.','Concrete source composition explicitly sets publication order; full tutorial is not claimed.','Command/property/SAI/membership services still need concrete WorldProgram binding.'],'independent_review':'Peer review pending.'}
p=HERE/'findings.json';p.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(p)}))
