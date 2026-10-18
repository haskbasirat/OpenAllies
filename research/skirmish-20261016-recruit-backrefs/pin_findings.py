from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(p):
    b=p.read_bytes()
    return {'path':str(p.relative_to(ROOT)).replace('\\','/'),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
base=json.loads((HERE/'native-recruit-backrefs.json').read_text())
real=json.loads((HERE/'native-real-recruit-backrefs.json').read_text())
source=json.loads((HERE/'actual-source-inputs.json').read_text())
paths=[ROOT/p for p in ('crates/aa-rules/src/native_recruit_backrefs.rs','crates/aa-rules/tests/native_recruit_backrefs.rs')]
paths.extend(HERE/p for p in ('native_recruit_backrefs.py','native-recruit-backrefs.json','actual-source-inputs.json','native-real-recruit-backrefs.json','instructions.txt','all-tests.log','clippy.log','clippy-scoped.log','README.txt'))
doc={'schema':1,'date':'2026-10-16','consumer':'0x501730','producer_addresses':['0x4624B0','0x464BD0','0x473AD0','0x475230','0x461750','0x461810','0x460000','0x460310','0x4C2310','0x45EEF0'],
     'original_sha256':base['original_sha256'],'v14_sha256':base['v14_sha256'],
     'canonical_actor_count':len(source['input']['actors']),'nonempty_17c_count':sum(bool(v) for v in source['lists'].values()),
     'source_callbacks_per_graph':len(source['order']),'source_native_graphs':len(real['cases']),
     'source_native_callback_executions':sum(len(c['output']['order']) for c in real['cases']),
     'synthetic_native_graphs':len(base['cases']),'tests':{'passed':3,'failed':0,'source_facts_equal_at':['Single24','Double53','Extended64'],'source_native_outcome_required':'ok'},
     'clippy':'passes with only two existing named lint allowances: clippy::manual_map and clippy::doc_lazy_continuation',
     'integration':'root and organization_morale_source own lazy Economy getter/Catalog cache; no live or saved Economy source edited here',
     'limits':['Completed+17C source projection only, not full native definition loading end to end','Explicit typed final4BDBA0 identity lookup, heap, CString retain/cleanup and diagnostic seams','Resource count0 omits warning diagnostics which do not change+17C','Invalid source/global producer failures remain errors; partial fault/load side effects unclaimed','No original/release/save/profile/network service files changed'],
     'full_compatibility_checks':'root coordinates aa-rules and aa-metagame include-ignored checks after combined freeze',
     'files':[pin(p) for p in paths]}
(HERE/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'files':len(paths),'findings_sha256':hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest()}))
