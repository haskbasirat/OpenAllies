"""Retain the tested named-UVar owner and its native prefix proof."""
from pathlib import Path
import hashlib,json,shutil
here=Path(__file__).resolve().parent;root=here.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=['crates/aa-ui/src/view_variables.rs','crates/aa-ui/src/view_variables_tests.rs','crates/aa-ui/src/uvars.rs']
snapshot=here/'source-snapshot';snapshot.mkdir(exist_ok=True);pins={}
for name in paths:
 path=root/name;dest=snapshot/path.name
 if dest.exists():assert dest.read_bytes()==path.read_bytes()
 else:shutil.copyfile(path,dest)
 pins[name]=sha(path)
exe=root/'target/s-borrowed-leader/release/deps/aa_ui-fb01eff3042d2284.exe';frozen=here/'frozen-aa-ui-tests.exe'
if frozen.exists():assert frozen.read_bytes()==exe.read_bytes()
else:shutil.copyfile(exe,frozen)
pins[frozen.name]=sha(frozen)
for name in ['native_texture_uvar.py','native-texture-uvar.json','native-immutable-replay.log','rust-tests-final.log','ui-map-regressions.log']:
 pins[name]=sha(here/name)
report=dict(
 scope='One source-created named Bool in SAME actual Ui.uvars/view_variables owner. No invented index, constructor allocation/default, duplicate current string map, texture decode or GPU readiness.',
 api='Ui.uvars.resource_no_ni_textures_current()->Result<u8,String>',
 source_semantics='Native4B3C10 creates kind1 flags0 on the actual typed source declaration. Later admitted typed/untyped literal assignments change the SAME byte; native reset-default byte remains its creation value. The source/default-copy snapshot retains the final completed source value. The actual corpus declares flag ResourceNoNiTextures=false in UVars.tgi112.',
 native=dict(paired_rows=15,separate_original_v14_executions=30,precision=['Single24','Double53','Extended64'],instruction_pins=3878,fixture_sha256=sha(here/'native-texture-uvar.json'),scope='Whole parser/4B3C10 and real4B2E60 named kind1 lookup. Real5A8D3F..80 cache/read/Resource-bit6 prefix stops BEFORE decoder. Raw0,1,2,128,255 prove exact1 adds bit0x40; other values preserve flags. Cached variable pointer remains SAME while current byte changes.',transports='Inherited whole parser line/EOF/heap/CString/mutex/log transports; no Bool creation, lookup or prefix success seam.',immutable_replay='Passed with fixture bytes unchanged.'),
 rust=dict(view_variable_tests=8,view_variable_failures=0,seconds=0.04,ui_map_regressions=2,actual_source_included=True,command='AA_GAME_ROOT set; CARGO_TARGET_DIR=target/s-borrowed-leader CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 cargo test -p aa-ui --lib view_variables::tests --release --locked --offline -j2 -- --include-ignored',coverage='Named allocation absence; source override; raw current setter; indexed-reset nonmutation; partial source failure; unbound profile rejection; same UI private override writes; original indexed Bool/reset regressions; actual original/v14 three-file source. Added map regression verifies the named byte is absent from both generic string maps.'),
 ownership='Existing213 indexed metadata and reset loop are unchanged. This flags0 named cell has no fabricated indexed slot. Existing UI option clones remain independent drafts; the texture backend must borrow its actual Ui owner and read the current byte at the reached native boundary.',
 integration='State owns actual Texture5A8CD0 backend and Resource flags. This report proves the getter/current storage and native prefix relation; it does not claim a completed actual Texture load or resource-ready transition.',
 bounded_unsupported=['Duplicate same-name fields in one source file require native hash ownership and refuse explicitly.','The first declaration must be an admitted typed flag/bool literal; a missing or unsupported producer is an error.','Unknown profile/console scope refuses; no source false fallback.'],
 historical_failure='native-create.log retained: first fragment harness accidentally used invoke(), which resets ESI/EBP. Fixed harness executes the exact fragment with its explicit caller register state. No production fix or fabricated fixture was used.',
 pins=pins)
(here/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=sha(here/'findings.json'),source=pins[paths[0]],tests=pins[paths[1]],uvars=pins[paths[2]],frozen_exe=pins[frozen.name])))
