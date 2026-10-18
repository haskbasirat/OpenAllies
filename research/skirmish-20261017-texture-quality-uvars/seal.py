"""Seal the exact quality-current owner and native lookup/mode prefix."""
from pathlib import Path
import hashlib,json,shutil
here=Path(__file__).resolve().parent;root=here.parents[1];sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();pins={}
snapshot=here/'source-snapshot';snapshot.mkdir(exist_ok=True)
for name in ['crates/aa-ui/src/view_variables.rs','crates/aa-ui/src/view_variables_tests.rs','crates/aa-ui/src/uvars.rs']:
 p=root/name;dest=snapshot/p.name
 if dest.exists():assert dest.read_bytes()==p.read_bytes()
 else:shutil.copyfile(p,dest)
 pins[name]=sha(p)
exe=root/'target/s-borrowed-leader/release/deps/aa_ui-fb01eff3042d2284.exe';frozen=here/'frozen-aa-ui-tests.exe'
if frozen.exists():assert frozen.read_bytes()==exe.read_bytes()
else:shutil.copyfile(exe,frozen)
pins[frozen.name]=sha(frozen)
for name in ['native_quality_uvars.py','native-quality-uvars.json','native-immutable-replay.log','rust-tests-01.log','ui-map-regressions.log']:pins[name]=sha(here/name)
report=dict(scope='Named code-created RendererForceLowQualityTextures and RendererUseHighQualityTextures in SAME Ui.uvars owner; lazy5A8B40 branch and sourceTexture87BE40 mode-store relation.',
 api=['Ui.uvars.renderer_force_low_quality_textures_current()->Result<u8,String>','Ui.uvars.renderer_use_high_quality_textures_current()->Result<u8,String>','Ui.uvars.texture_quality_source_mode()->Result<bool,String>'],
 producer='Actual41F9D0 appended4B35F0 registration:424393 constructs High current/default1, flags16;4243D3 constructs Force current/default0, flags16. No invented indexed slot; the first213 indexed reset loop does not include these named appends.',
 native=dict(machine_pairs=3,separate_original_v14_all_pc_machines=6,policy_samples_per_machine=50,total_policy_invocations=300,instruction_pins=5841,fixture_sha256=sha(here/'native-quality-uvars.json'),scope='Real entire41F9D0 prefix through Force publication, real Settings4B35F0/4B3470 hash/vector/register bodies. Actual5A8BBC..C39 named lookup/byte branches and real87BE40 store. Caller supplies explicit sourceTexture+48 prior0/poison and actual leaf vtable entry; Ni construction is separate.',transports='Inherited whole-parser harness heap/CString/mutex hosts only; no registration/Bool/quality lookup/decision/mode-store success seam. Unrelated global empty CString uses real4013E0.',immutable_replay='Passed; fixture unchanged.'),
 rust=dict(view_variable_tests=11,failed=0,seconds=0.06,actual_source=True,same_binary_ui_map_regressions=2,map_seconds=0.01,coverage='Native constructor metadata; raw byte cross-product; Force short circuit; conditional High error; earlier getter failure; typed source overrides; same current UI/private writes; default snapshot; source/profile refusal; no string-map duplication; indexed/named prior regressions.',build_seconds=20.52),
 semantics='Force!=0 invokes sourceTexture3C(1) without looking up High. Otherwise High==0 invokes it. False preserves prior+48 instead of storing zero. This is a sourceTexture mode decision, not Resource ready/loaded state.',
 current_ownership='Code constructor bytes and admitted source mutations are owned once. Existing UI option clones remain drafts, and backend must borrow actual current Ui. Same raw/private UI setters update these fields. Native70F4F0/71032E graphics-device Force writers and67D010/options High writers are distinct callers; this report does not claim those unported legacy hardware paths executed or bless a cached constructor snapshot.',
 required_boundaries=['Real NiSourceTexture allocation/constructor and file decode/IO are owned by State/Root.','Native sourceTexture data load, GPU publication and Resource ready transition are not certified by the Bool decision.','Unknown external profile/console and unproved source forms refuse explicitly.'],
 historical_failure='native-create-01/02.log retained: RenderStandardFont CString global was initially unconstructed; fixed harness executes actual4013E0 rather than supplying guessed pointer/default bytes.',pins=pins)
(here/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=sha(here/'findings.json'),source=pins['crates/aa-ui/src/view_variables.rs'],uvars=pins['crates/aa-ui/src/uvars.rs'],frozen_exe=pins[frozen.name])))
