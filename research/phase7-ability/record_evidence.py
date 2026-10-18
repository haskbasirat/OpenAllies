"""Pin the reviewed scope; this records evidence, it does not run Rust tests."""
from pathlib import Path
import hashlib,json,re,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified,Cs,CS_ARCH_X86,CS_MODE_32
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
SCOPES=[('performer','phase7-performer','native_performer.py','native-performer-fixtures.json'),('admission','phase7-ability','native_admission.py','native-admission-fixtures.json'),('execution','phase7-ability','native_execution.py','native-execution-fixtures.json'),('projectile','phase7-ability','native_projectile.py','native-projectile-fixtures.json'),('constructors','phase7-performer','native_constructors.py','native-constructor-fixtures.json'),('weights','phase7-performer','native_weights.py','native-weight-fixtures.json'),('dispatch','phase7-ability','native_dispatch.py','native-dispatch-fixtures.json'),('launch','phase7-ability','native_launch.py','native-launch-fixtures.json'),('surface','phase7-surface','native_surface.py','native-surface-fixtures.json'),('payment','phase7-payment','native_payment.py','native-payment-fixtures.json'),('fx','phase7-fx','native_fx.py','native-fx-fixtures.json')]
ranges={'fx_wrapper':(0x4c8500,0x4c8560),'fx_request':(0x4294e0,0x4296b0),'fx_allegiance':(0x428f40,0x428fb0),'fx_constructor':(0x428570,0x428620),'shape_projection':(0x46e150,0x46e284),'target_hit_surface':(0x4eee60,0x4eef64),'ability_payment':(0x4e2590,0x4e2630),'faction_payment':(0x4cdd90,0x4cddfa),'cost_copy':(0x4beb50,0x4bebec),'stockpile_debit':(0x4e1690,0x4e16c5),'action_constructors':(0x4c8610,0x4c8740),'target_flags':(0x4c5840,0x4c5a90),'recharge':(0x4c83e0,0x4c8500),'eligibility_availability':(0x4c8740,0x4c8cb0),'projectile_launch':(0x4c8cb0,0x4c8f20),'admission':(0x4c8f20,0x4c9550),'execution_area':(0x4c9550,0x4c9eb0),'local_spawn':(0x4be780,0x4be8e0),'utility':(0x4be6c0,0x4be720),'other_utility':(0x57e670,0x57e730),'performer_dispatch':(0x57e030,0x57e1c0),'performer_begin_start_event':(0x57ed20,0x57f4d0),'performer_ctor':(0x57f4d0,0x57f790),'projectile':(0x580660,0x580ff0)}
blocks=[];asm=[];md=Cs(CS_ARCH_X86,CS_MODE_32)
for name,(start,end)in ranges.items():
 a=original.range(start,end-start);b=modified.range(start,end-start);assert a==b,name
 blocks.append({'name':name,'start':hex(start),'end_exclusive':hex(end),'sha256':hashlib.sha256(a).hexdigest(),'original_equals_accepted':True})
 asm.append('=== '+name+' ===\n'+'\n'.join(f'{i.address:08x} {i.mnemonic} {i.op_str}'for i in md.disasm(a,start)))
(HERE/'full-assembly.txt').write_text('\n\n'.join(asm)+'\n',encoding='utf8')
scopes=[];files=[]
for name,folder,producer,fixture in SCOPES:
 p=ROOT/'research'/folder/producer;f=p.parent/fixture;d=json.loads(f.read_text());assert d['producer_sha256']==sha(p)
 for w in d['visited_windows']:
  raw=bytes.fromhex(w['bytes']);a=int(w['address'],16);assert original.range(a,len(raw))==raw and modified.range(a,len(raw))==raw
 scopes.append({'name':name,'cases':len(d['cases']),'transitions':sum(len(c.get('steps',[]))for c in d['cases']),'producer':str(p.relative_to(ROOT)).replace('\\','/'),'fixture':str(f.relative_to(ROOT)).replace('\\','/')});files.extend([p,f])
assert sum(s['cases']for s in scopes)==2326
files += list((ROOT/'crates/aa-combat/src').glob('*.rs'))+list((ROOT/'crates/aa-combat/tests').glob('*.rs'))+[ROOT/'crates/aa-combat/Cargo.toml']
for folder in ['crates/aa-body/src','crates/aa-numeric/src','crates/aa-economy/src']:
 files+=list((ROOT/folder).rglob('*.rs'))
for rel in ['research/phase4-rng/original_rng.py','research/engine-baseline/audit_baseline.py','../reconstruction/tools/map_extract.py','research/phase6-weapon/native_weapon.py','research/phase6-combat/original_damage.py','research/phase4-body/original_body.py','research/phase3-terrain/terrain_uv_oracle.py']:
 files.append(ROOT/rel)
files += [ROOT/'crates/aa-spatial/src/collision.rs']
files += [HERE/n for n in ['README.txt','final-tests.log','final-clippy.log','full-assembly.txt','record_evidence.py']]
tests=(HERE/'final-tests.log').read_text(encoding='utf8');assert sum(map(int,re.findall(r'test result: ok\. (\d+) passed',tests)))==22
assert 'error:' not in (HERE/'final-clippy.log').read_text(encoding='utf8')
report={'schema':1,'status':'component_native_validation_passed_pending_independent_review_and_runtime_composition','cases':2326,'rust_tests':22,'new_rust_tests':16,'scopes':scopes,'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'full_code_ranges':blocks,'files':{str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in sorted(set(files))},'limits_reference':'research/phase7-ability/README.txt','commands':['cargo test -p aa-combat','cargo clippy -p aa-combat --all-targets -- -D warnings'],'scope_note':'Full byte windows include adjacent unported code; only named Rust APIs and fixture observations are claimed. This ledger records tests already run; it does not rerun them. Dependency hashes bind the current tested snapshot and are not independent review claims.'}
(HERE/'findings.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps({'cases':2326,'rust_tests':22,'findings_sha256':sha(HERE/'findings.json')}))
