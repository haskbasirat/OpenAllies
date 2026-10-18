"""Record separate numeric property-lifetime and Body update acceptance scopes."""
from pathlib import Path
import csv,hashlib,json,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
scopes=[('effects','native-effects-fixtures.json','original_effects.py','property_effects.rs','native_property_effects.rs',111,2,[0x581630,0x581810,0x5020a0,0x502a70,0x5030f0,0x5032d0,0x5034f0,0x503680,0x4fe6c0]),('body-updates','native-body-updates.json','original_body_updates.py','updates.rs','native_body_updates.rs',87,1,[0x561d40,0x561790,0x562200,0x45d730,0x45d760]),('modifier-load','native-modifier-load.json','original_modifier_load.py','modifier_load.rs','native_modifier_load.rs',957,2,[0x4fec70,0x4fed70])]
commands=[]
for name,args in [('lifecycle-tests.log',['cargo','test','-p','aa-body']),('lifecycle-clippy.log',['cargo','clippy','-p','aa-body','--all-targets','--','-D','warnings'])]:
    run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/name).write_bytes(run.stdout)
    assert run.returncode==0,run.stdout.decode(errors='replace');commands.append({'command':args,'exit_code':0,'log':name,'sha256':sha(HERE/name)})
for name,fixture,producer,source,test,count,test_count,addresses in scopes:
    doc=json.loads((HERE/fixture).read_text());assert len(doc['cases'])==count and doc['producer_sha256']==sha(HERE/producer)
    ranges=[]
    for address in addresses:
        raw=IMAGE.at(address,sizes[address]);assert raw==ACCEPTED.at(address,len(raw))
        ranges.append({'address':hex(address),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
        (HERE/f'lifecycle-{address:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,address))+'\n')
    files=[ROOT/'crates/aa-body/src'/source,ROOT/'crates/aa-body/tests'/test]+[HERE/p for p in [fixture,producer,'LIFECYCLE.txt','record_lifecycle.py']]
    dependencies=['crates/aa-body/src/lib.rs','crates/aa-body/src/properties.rs','crates/aa-numeric/src/lib.rs','research/phase4-body/original_body.py']
    if name in ['effects','modifier-load']:dependencies+=['research/phase7-properties/original_properties.py']
    if name=='modifier-load':dependencies+=['crates/aa-body/src/property_effects.rs']
    report={'schema':1,'passed':True,'scope':name,'native_cases':count,'focused_tests':test_count,'original_sha256':doc['original_sha256'],'v14_sha256':doc['v14_sha256'],'native_ranges':ranges,'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in files},'dependency_sha256':{p:sha(ROOT/p) for p in dependencies},'seams':doc['seams'],'byte_pin_scope':'Visited code pins include replaced seam entry bytes, not execution behind those leaves. Full function ranges are static source identity checks.','independent_review':'Separate peer review required.'}
    if name=='effects':
        report['native_transitions']=sum(len(row['output']) for row in doc['cases']);assert report['native_transitions']==390
        report['limitations']=['Native loaded Add/Multiply definitions only; unsupported kinds may not be silently omitted.','Actor consequences remain required runtime callbacks.','Callback failure retains prior writes; detached release record ownership is explicit.','Complete live-world/timed/save integration not established.']
    elif name=='body-updates':report['limitations']=['Complete no-linked-node paths only; graph traversal belongs to runtime.','Capture maximum ratio before linked recursion and retain extended derived maximum across field store.','562200 callers must re-resolve linked identity after setter callbacks for every next step.']
    else:report['limitations']=['Starts from the actual binary32 amount field value and resolved numeric attribute; no TGI file parsing or attribute naming inferred.','Invalid native inputs stop at diagnostic and Rust returns a construction error; invalid partial record state is outside scope.','Nonfinite amounts and a repair flag supplied for Multiply are explicitly unsupported.']
    path=HERE/f'{name}-findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'scope':name,'passed':True,'sha256':sha(path)}))
