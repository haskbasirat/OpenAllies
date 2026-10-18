"""Seal the warning-current successor without altering quality checkpoints."""
from pathlib import Path
import hashlib,json,shutil
here=Path(__file__).resolve().parent;root=here.parents[1];sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();pins={}
snapshot=here/'warning-source-snapshot';snapshot.mkdir(exist_ok=True)
for name in ['crates/aa-ui/src/view_variables.rs','crates/aa-ui/src/view_variables_tests.rs','crates/aa-ui/src/uvars.rs']:
 p=root/name;dest=snapshot/p.name
 if dest.exists():assert dest.read_bytes()==p.read_bytes()
 else:shutil.copyfile(p,dest)
 pins[name]=sha(p)
exe=root/'target/s-borrowed-leader/release/deps/aa_ui-fb01eff3042d2284.exe';frozen=here/'frozen-aa-ui-tests-warning.exe'
if frozen.exists():assert frozen.read_bytes()==exe.read_bytes()
else:shutil.copyfile(exe,frozen)
pins[frozen.name]=sha(frozen)
for name in ['native_warning_uvar.py','native-warning-uvar.json','warning-native-immutable-replay.log','warning-rust-build.log','warning-rust-tests.log','warning-ui-map-regressions.log']:pins[name]=sha(here/name)
report=dict(scope='DataMissingResourceWarning named code-created Bool in SAME Ui.uvars owner; actual source writes and native diagnostic-level prefix. No default texture/factory/decode completion claim.',
 api='Ui.uvars.data_missing_resource_warning_current()->Result<u8,String>',
 producer='423A31 within41F9D0 constructs Bool kind1 flags0 current/default1 and4B35F0 appends it. It is not a fabricated indexed entry or an absent-source default. UVars.tgi172 writes true; UVars_AA.tgi12 writes false; UVars_Restored leaves it unchanged.',
 native=dict(paired_rows=3,separate_original_v14_all_pc_machines=6,warning_samples_per_machine=5,total_warning_prefix_invocations=30,instruction_pins=11624,fixture_sha256=sha(here/'native-warning-uvar.json'),scope='Full actual41F9D0 +Settings/hash/vector constructors; all3 complete original/v14 UVars source files parsed and loaded with4B3C10. Same allocation current1->0->0, native reset-default1 retained. Raw0/1/2/128/255 run real5A6E90 prefix up to mandatory404FC0 diagnostic entry.',diagnostic='NEG AL/SBB EAX,EAX/AND FFFFFFF7/ADD10: nonzero selects1, zero10. No level3 branch exists here.',transports='Inherited whole-parser line/heap/CString/mutex/log transports; no Bool constructor/source/getter/diagnostic-level decision success seam.',immutable_replay='Passed with fixture unchanged.'),
 rust=dict(view_variable_tests=13,failed=0,seconds=0.06,same_binary_ui_map_regressions=2,map_seconds=0.01,actual_source=True,coverage='Code constructor byte; actual source current0; raw/private UI writes and export/apply; source snapshot0 versus native reset-default1; indexed reset nonmutation; private draft independence; strict unknown-profile refusal; no parallel generic string entries; all prior quality/named/indexed Bool regressions.'),
 integration='State owns actual5A6E90 manager/default-Texture factory and diagnostic service. This getter supplies only the reached SAME current byte. It does not manufacture a default Texture, Resource ready bit or GPU acknowledgement.',
 previous_checkpoints='findings.json and texture-uvar named/compatibility checkpoints remain historical exact snapshots; these successor source hashes include the approved warning cell.',pins=pins)
(here/'warning-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=sha(here/'warning-findings.json'),source=pins['crates/aa-ui/src/view_variables.rs'],uvars=pins['crates/aa-ui/src/uvars.rs'],frozen_exe=pins[frozen.name])))
