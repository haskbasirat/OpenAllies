from pathlib import Path
import hashlib,json,shutil
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
names=['crates/aa-runtime/src/stored_member_command.rs','crates/aa-runtime/src/stored_member_command_fields.rs','crates/aa-runtime/src/stored_member_command_fields_tests.rs']
sources=[]
for name in names:
 p=ROOT/name;snapshot=HERE/'source-v1'/p.name;snapshot.parent.mkdir(exist_ok=True)
 shutil.copyfile(p,snapshot);sources.append(dict(path=name,snapshot=str(snapshot.relative_to(HERE)),sha256=sha(snapshot)))
native=json.loads((HERE/'native-member-command-fields.json').read_text())
assert len(native['rows'])==387 and len(native['instructions'])==105
assert native['constructor_fixture_sha256']=='199f1ac85a81cbbe47ed412eccf09a07d4c7c1f8e0495eaabe0d1f7abad6ace9'
core=ROOT/'research/development10/member-command/native-member-command.json'
assert sha(core)=='4877137224825ad96f2810fff17db360c7fd9ffc9095a6997157777f6b91414b'
assert '3 passed; 0 failed' in (HERE/'exact-fields-tests.log').read_text(encoding='utf-8-sig')
assert '4 passed; 0 failed' in (HERE/'element-position-regressions.log').read_text(encoding='utf-8-sig')
check=HERE/'check-runtime-final06.log';text=check.read_text(encoding='utf-8-sig')
assert 'Finished `release`' in text and 'error[' not in text
files=['native_member_command_fields.py','native-member-command-fields.json','native-fields-write.log','native-fields-immutable.log','exact-fields-tests.rs','exact-fields-tests.exe','exact-fields-tests.log','exact-fields-build.log','run_exact_fields.ps1','element-position-regressions.log','check-runtime-final06.log','README.md','seal_checkpoint.py']
report=dict(schema=1,status='author bounded native fields/source checkpoint; whole-owner integration pending',
 scope='522890 actual owner adapter and shared direct Stop; original coordinator and order backend reused',
 native_new=dict(rows=387,original_v14_executions=774,executed_address_byte_pins=105,static_vtable_data_pins=3,immutable_replay='PASS',aircraft='finite native fixture; logical hardware x87_copy tested separately'),
 native_core_reused=dict(path=str(core.relative_to(ROOT)),sha256=sha(core),pairs=288,executions=576,pins=298,reviewer_replay=False),
 rust=dict(exact_production_fields='3/3 PASS',existing_element_position_regressions='4/4 PASS',production_typecheck='PASS final06',whole_runtime_or_client_link=False,whole_actoraccess_execution='pending'),
 sources=sources,
 shared_algorithms=['aa_mind::cai_member_command::run','aa_actors::element_position::set_position','organization_mind_callbacks::position_active','player_order_decree::allocate_decoded_in_bank','player_order_world::deliver_in_bank','player_order_frame::Services','aa_mind::cai_thinks::x87_copy'],
 ownership=['same current ActorAccess/Arena/Bank/Runtime/command allocations','captured Organization identity and Zone4 owner/key; current fields reread','short field loans end before actual command delivery','constructor command reference released after both successful and failed delivery','actual goal store retained before reached relocation/interpolation error'],
 required=['positive Organization branch of4CEDE0','Base Mind vf80 purecall','unwritten/foreign current owners','full4EEB40/4D0EE0 relocation children','positive57C080 interpolation and50AC00 scheduling','any reached unbound actual command backend service'],
 not_claimed=['natural mission acceptance or initialized battle','whole57C680 positive relocation success','Unicorn signalling-NaN hardware parity','malformed native pointer/allocator/Win32/unwind equivalence','parent or peer later source successors'],
 files={name:sha(HERE/name)for name in files})
out=HERE/'findings.json';out.write_text(json.dumps(report,indent=2)+'\n');print(sha(out))
