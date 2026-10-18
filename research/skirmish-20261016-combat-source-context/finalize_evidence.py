"""Pin final source-context input binding evidence without editing source."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(path):
 p=ROOT/path
 return {'path':path,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size}
native=json.loads((HERE/'native-context.json').read_text())
assert len(native['numeric_cases'])==180 and len(native['csv_cases'])==36 and len(native['instructions'])==2489
assert '4 passed; 0 failed' in (HERE/'tests-final.log').read_text()
actual_path=HERE/'actual-source-inputs-final.json'
actual=json.loads(actual_path.read_text())
review=json.loads((HERE/'independent-review.json').read_text())
assert review['status']=='accepted' and review['blockers']==[]
assert len(actual['numeric'])==3
assert all(len(row['numeric'])==10 and not row['context_observed_native_startup'] for row in actual['numeric'])
assert len(actual['csv']['assignments'])==2 and len(actual['csv']['inputs'])==3 and actual['csv']['value_utf16']==[]
assert actual['csv']['input_load_flags']==[0,0,0] and actual['csv']['ignored_restricted_assignments']==[]
assert actual['source_load_exceptions']==0 and len(actual['world_rule_registrations'])==9
paths=['crates/aa-runtime/src/combat_value_source_context.rs','crates/aa-runtime/tests/combat_value_source_context.rs']
paths += [str(p.relative_to(ROOT)).replace('\\','/') for p in sorted(HERE.iterdir()) if p.is_file() and p.name!='findings.json']
out={'schema':1,'passed':True,'scope':'Source-phase ten typed CV inputs, explicitly supplied diagnostic FP/CRT context, and bounded normal/restricted CSV source owner. No completed combat values or startup receipt.','independent_review':pin(str((HERE/'independent-review.json').relative_to(ROOT)).replace('\\','/')),'images':{k:native[k] for k in ['original_sha256','v14_sha256']},'counts':{'native_numeric_cases':180,'native_csv_cases':36,'native_restricted_csv_cases':12,'executed_instruction_pins_both_images':2489,'focused_rust_tests':4,'actual_numeric_fields_per_context':10,'explicit_source_pcs':3,'actual_root_uvars_inputs':3,'actual_csv_assignments':2,'actual_matching_cv_worldrules':0},'native_profile_correction':'419F72/419F82 supplies4B43B0flags3; Settings+24restricts existing variables toflag16. Rootsource-created CombatValueCSVflags0 ignores this stage. Rootvariable must exist before override binding; otherwise binding explicitly refuses unsupported creation.','context_limit':'SourceFpContext has no Default and only supplied_diagnostic; every context remains NOT an observed native startup receipt. No restore-PC substitution, active profile import, credential/file discovery or fabricated completedCV.','preservation':'New aa-runtime module/test +append export only; root exposed existing Catalog source-owner guard. No parser/loader, manifests, lockfile, original images/assets/saves/profiles or releases edited.','validation_corrections':['Global lowercase filename set fixes nonadjacent case collision, with regression.','Settings.raw/source span crosschecked with actual resolved Catalog SVars source; unknown loader diagnostics refused.','Native hook entry bytes omitted from executed-instruction pins.','Nativeflags3 profile skip distinguished from normalmode0 overrides.','Unproved unquoted empty CSV assignment explicitly refused.'],'not_completed':['Actual source-load FP/CRT/application receipt','Global means and all reached shared ActorData/Ability source graph completion','CompletedActorData1E4/30C/36C and livePerformer60','Nonempty CSV stream output and unsupported text/profile/native variable policies','Original immediateInit/PostCreate and playablebattlestartup'],'files':[pin(path) for path in paths]}
(HERE/'findings.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
print('findings SHA256',hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest())
