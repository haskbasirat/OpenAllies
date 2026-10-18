"""Independent source/oracle acceptance for the parent's 570020 implementation."""
from pathlib import Path
import csv,hashlib,json,subprocess
import native_refresh as native

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
fixture=json.loads((HERE/'native-refresh-fixtures.json').read_text())
assert len(fixture['cases'])==159 and len(fixture['equality_cases'])==450
assert fixture['producer_sha256']==sha(HERE/'native_refresh.py')
for item in fixture['visited_windows']:
    address=int(item['address'],16);raw=bytes.fromhex(item['bytes'])
    assert native.base.ORIGINAL.at(address,len(raw))==raw==native.base.ACCEPTED.at(address,len(raw))
sizes={int(row['address'],16):int(row['bodyBytes']) for row in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
addresses=[0x570020,0x56f310,0x56f340,0x4cd440,0x4cd230,0x4cd250,0x4cce70,0x45f180,0x45e5a0,0x4e48c0,0x4e4930,0x4e3940,0x4e1f60]
ranges=[]
for address in addresses:
    raw=native.base.ORIGINAL.at(address,sizes[address]);assert raw==native.base.ACCEPTED.at(address,len(raw))
    ranges.append({'address':hex(address),'length':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
    (HERE/f'{address:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in native.Cs(native.CS_ARCH_X86,native.CS_MODE_32).disasm(raw,address))+'\n')
commands=[]
for log,args in [('tests.log',['cargo','test','-p','aa-runtime','--test','economy_refresh_native']),('clippy.log',['cargo','clippy','-p','aa-runtime','--test','economy_refresh_native','--','-D','warnings'])]:
    run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/log).write_bytes(run.stdout)
    assert run.returncode==0,run.stdout.decode(errors='replace')
    commands.append({'command':args,'exit_code':run.returncode,'log':log,'sha256':sha(HERE/log)})
files=['crates/aa-runtime/src/economy_refresh.rs','crates/aa-runtime/tests/economy_refresh_native.rs']+[f'research/phase7-economy-refresh/{name}' for name in ['README.txt','native_refresh.py','native-refresh-fixtures.json','record_findings.py']]
dependencies=['crates/aa-actors/src/ground_states.rs','crates/aa-economy/src/lib.rs','crates/aa-economy/src/binary.rs','research/evidence/economy-phase2-oracle.py']
report={'schema':1,'passed':True,'reviewer':'core_review; production owned by root','scope':'Original570020 outer refresh plus actual vector and coupled Kingdom arithmetic; value computation/world callbacks explicitly supplied.','original_sha256':fixture['original_sha256'],'v14_sha256':fixture['v14_sha256'],'native_refresh_cases':159,'native_equality_cases':450,'rust_tests':4,'commands':commands,'ranges':ranges,'source_sha256':{p:sha(ROOT/p) for p in files},'dependency_sha256':{p:sha(ROOT/p) for p in dependencies},'findings':[],'checked_semantics':['First owner captured for all additions, removal obtains live owner after clearing flag.','Both old arrays and callback changes are reread; signed zeros compare equal and NaNs unequal.','Providers second predicate may turn false; member cache cleared before it and write_member_cache true only on supplied getter call.','Cost computed after resource contribution changes, final parent reread and callback follow.','Real Kingdom arrays preserve exact per-PC shortage rounding.','Errors retain already completed writes; nonfinite resource arithmetic is rejected explicitly.'],'seams':fixture['seams'],'limitations':['Finite refresh arithmetic only; nonfinite values covered solely by isolated equality and explicit rejection.','Coherent immutable owner identity and resource registry during each call.','No source/Organization vector computation or parent recursive refresh implementation is claimed.','No actual game session or complete world/tutoral parity.']}
path=HERE/'independent-review.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(path),'cases':159,'equality_cases':450}))
