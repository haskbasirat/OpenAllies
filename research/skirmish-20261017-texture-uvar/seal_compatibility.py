"""Seal the approved same-owner indexed61 successor separately from v1."""
from pathlib import Path
import hashlib,json,shutil
here=Path(__file__).resolve().parent;root=here.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
snapshot=here/'compatibility-source-snapshot';snapshot.mkdir(exist_ok=True);pins={}
for name in ['crates/aa-ui/src/view_variables.rs','crates/aa-ui/src/view_variables_tests.rs','crates/aa-ui/src/uvars.rs']:
 p=root/name;dest=snapshot/p.name
 if dest.exists():assert dest.read_bytes()==p.read_bytes()
 else:shutil.copyfile(p,dest)
 pins[name]=sha(p)
exe=root/'target/s-borrowed-leader/release/deps/aa_ui-fb01eff3042d2284.exe';frozen=here/'frozen-aa-ui-tests-compatibility.exe'
if frozen.exists():assert frozen.read_bytes()==exe.read_bytes()
else:shutil.copyfile(exe,frozen)
pins[frozen.name]=sha(frozen)
registration=root/'research/skirmish-20261017-view-init/native-uvar-registration.json'
rows=json.loads(registration.read_bytes())['cases'];assert len(rows)==6
for row in rows:
 v=row['registrations'][61]
 assert ''.join(map(chr,v['name']))=='DataCompatibilityMode'
 assert (v['index'],v['kind'],v['flags'],v['bool_current'],v['bool_default'])==(61,1,0,1,1)
report=dict(
 scope='Add actual indexed UVar61 DataCompatibilityMode to SAME Ui.uvars State; preserve all named Bool v1 behavior. Not a native texture catch/decoder completion proof.',
 api='Ui.uvars.data_compatibility_mode_current()->Result<u8,String>',
 native_registration=dict(fixture=str(registration.relative_to(root)),sha256=sha(registration),fresh_immutable_replay=True,separate_original_v14_all_pc_runs=6,registrations_per_run=213,instruction_pins=5764,actual61=dict(name='DataCompatibilityMode',kind=1,flags=0,current=1,default=1)),
 raw_setter_proof=dict(fixture='research/development10/native-view-uvar108.json',scope='Same actual flags0 Bool setter4B3190/4B29B0. Successor test projects all144 rows onto actual61 metadata and preserves its native reset-default byte.'),
 rust=dict(view_variable_tests=9,passed=9,failed=0,seconds=0.41,actual_source=True,source_declaration='Actual original UVars.tgi171 flag DataCompatibilityMode=true; strict actual3-file source test verifies current1.',map_regressions=2,map_seconds=0.02,log='compatibility-tests-01.log',same_binary='compatibility-map-regressions.log',coverage='Native61 constructor metadata, source=false mutation, arbitrary raw current bytes, reset nonmutation, private override save/apply, draft independence, absent parallel string entries and unknown-profile refusal; all named/indexed prior tests still pass.'),
 integration='State owns native5A7976 catch/read proof. Getter returns raw byte only; zero/nonzero and soft/hard request order remain with that caller. No texture load, missing-resource catch, default texture or GPU completion claim here.',
 index47='Optional TextureNoMipMaps47 lead was not applied: actual UVar47 is ResolutionX(kind3/flags16). No guessed registration or variable was added.',
 previous_checkpoint=dict(findings='findings.json',scope='The earlier named-only source/exe snapshots remain unchanged and historical. These successor source hashes include approved indexed61.'),
 pins=pins)
(here/'compatibility-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=sha(here/'compatibility-findings.json'),source=pins['crates/aa-ui/src/view_variables.rs'],uvars=pins['crates/aa-ui/src/uvars.rs'],frozen_exe=pins[frozen.name])))
