from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
prior=HERE/'findings.json';assert H(prior)=='620dc603ba3fcb049c6414f5535c9d24275b728af43a7c2583a9cf6036979bbd'
base=ROOT/'research/development10/playable-startup';inputs=base/'attempt28.inputs.json';snapshot=base/'attempt28.fresh-actor-prefix.json';worker=base/'attempt28.worker.json'
i=json.loads(inputs.read_text());s=json.loads(snapshot.read_text());w=json.loads(worker.read_text())
assert i['build_started']==i['build_completed'] and i['selected_changed_files']==[]
for source in ['crates/aa-runtime/src/map_denizen_init_create.rs','crates/aa-runtime/src/map_component_init_create.rs','crates/aa-actors/src/component_virtuals.rs']:
 rows=[row for row in i['build_completed']if row['path']==source];assert len(rows)==1 and H(ROOT/source)==rows[0]['sha256']
r=next(r for r in s['records']if r['gtid']==1688);assert r['object_key']==2362
body=r['actual_runtime_debug'];a=body.index('Denizen(');b=body.index('Some(',a+1);denizen=body[a:b]
assert '52: 2147483648,'in denizen and 'inventory: [],'in denizen
assert 'fresh memory requires actual application Session+F8 owner'in w['error']
binary=ROOT/i['binary_path'];current_binary=H(binary)if binary.exists()else None
doc=dict(schema=1,status='genuine strict map run advanced through Denizen568750; later Session loan is the next stop',
 prior_findings_sha256=H(prior),build=dict(result=i['build_result'],binary_sha256=i['binary_sha256'],current_binary_sha256=current_binary,current_binary_matches=current_binary==i['binary_sha256'],source_start_end_identical=True),
 observed=dict(gtid=1688,key=2362,definition='ger_corps',denizen_header_owner=1688,inventory_count=0,word34=2147483648,word34_hex='0x80000000',new_phase=r['new_phase'],later_error=w['error']),
 qualifier='Fresh actual28 owner is distinct from actual25 predecessor allocations. The production path mutates its same retained Denizen Box directly; snapshot confirms owner/scalar/list result and later refusal. No claim of complete fresh Actor initialization, Mission or playable battle.',
 inputs={str(p.relative_to(ROOT)):H(p)for p in[inputs,snapshot,worker,ROOT/i['build_log']]})
data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'actual28-addendum.json';p.write_bytes(data)
print(json.dumps(dict(sha256=hashlib.sha256(data).hexdigest(),current_binary_matches=doc['build']['current_binary_matches'],word34='0x80000000')))
