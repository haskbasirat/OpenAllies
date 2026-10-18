from pathlib import Path
import csv,hashlib,json,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'));from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')};ranges=[]
addresses=[0x546af0,0x541900,0x5422f0,0x54a5a0,0x548900,0x4c5db0,0x541de0,0x5425c0,0x54d9e0,0x54da40,0x54da50,0x54bd30,0x540a30,0x5439a0,0x54d280,0x54cce0,0x54dd40,0x541f40,0x541f60,0x54ce70,0x54ea10,0x54da20,0x54dac0,0x54dc20,0x54e880,0x54e080,0x541d40,0x541d60,0x4ed890,0x4ed8b0,0x5421a0,0x541fe0]
for a in addresses:
 raw=IMAGE.at(a,sizes[a]);assert raw==ACCEPTED.at(a,len(raw));ranges.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
 (HERE/f'{a:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,a))+'\n')
assert IMAGE.at(0x9f32dc,4)==ACCEPTED.at(0x9f32dc,4)==b'\0'*4
corpora=[]
for fixture,producer,count in [('native-saved-writer.json','native_saved_writer.py',185),('native-manager-fixtures.json','native_manager.py',67),('native-core-program.json','native_core_program.py',327),('native-world-program.json','native_world_program.py',201)]:
 doc=json.loads((HERE/fixture).read_text());assert doc['producer_sha256']==sha(HERE/producer) and len(doc['cases'])==count
 for window in doc['visited_windows']:
  raw=bytes.fromhex(window['bytes']);address=int(window['address'],16);assert IMAGE.at(address,len(raw))==ACCEPTED.at(address,len(raw))==raw
 corpora.append({'file':fixture,'sha256':sha(HERE/fixture),'cases':count,'seams':doc['seams']})
commands=[]
for log,args in [('tests.log',['cargo','test','-p','aa-triggers']),('clippy.log',['cargo','clippy','-p','aa-triggers','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/log).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace');commands.append({'command':args,'exit_code':run.returncode,'log':log,'sha256':sha(HERE/log)})
sources=list((ROOT/'crates/aa-triggers/src').glob('*.rs'))+list((ROOT/'crates/aa-triggers/tests').glob('*.rs'))+[ROOT/'crates/aa-triggers/Cargo.toml']+list(HERE.glob('*.py'))+[HERE/p for p in ['README.txt','WORLD.txt','native-registrations.json','tutorial01-decoded.json','tutorial02-decoded.json','tutorial-census.json','saved-rust-fixtures.json']]
report={'schema':1,'passed':True,'scope':'Trigger serialization, ordered manager state, eight native intrinsic opcodes and eight recovered world-action wrappers','original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'native_corpora':corpora,'native_ranges':ranges,'rust_tests':8,'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in sources},'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-numeric/src/lib.rs','crates/aa-mapworld/src/archive.rs','crates/aa-navigation/src/control.rs','crates/aa-body/src/property_admission.rs','crates/aa-body/src/property_effects.rs','research/phase4-body/original_body.py']},'review':'Independent peer review pending.','limitations':['Not a working tutorial or full trigger VM: unsupported opcodes return explicit errors.','WorldProgram requires actual canonical registry/component/name/property services; tests declare those fixtures.','Native writer gate covers185 individual records, not manager hash-table serialization.','Manager sequencing fixture replaces546AF0 only for that separate gate.','Current runtime trigger/element lists immutable during callbacks.','Finite round-to-nearest arithmetic and ASCII variable-ID domain only.','Final Windows EXE integration not tested here.'],'prior_failures':['Initial strict check encountered dependency aa-mapworld collapsible_if lint; owner fixed it.','World oracle initially observed presentation offsets without the wrapper World+4C base; Rust comparison caught it, source verified and observer/comments corrected.']}
path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(path)}))
