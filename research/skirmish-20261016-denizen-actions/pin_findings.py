from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(path):
    data=path.read_bytes()
    return {'path':str(path.relative_to(ROOT)).replace('\\','/'),'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}
base=json.loads((HERE/'native-denizen-actions.json').read_text())
saved=json.loads((HERE/'native-saved-denizen-actions.json').read_text())
files=[ROOT/p for p in ('crates/aa-actors/src/denizen_actions.rs','crates/aa-actors/tests/native_denizen_actions.rs','crates/aa-runtime/src/saved_denizen_actions.rs')]
files.extend(HERE/p for p in ('native_denizen_actions.py','native-denizen-actions.json','actual-saved-inputs.json','native-saved-denizen-actions.json','contributor-instructions.txt','helper-instructions.txt','pure-test.log','runtime-test.log','clippy-scoped.log','clippy-runtime.log','README.txt'))
mapfile=ROOT.parent/'reconstruction/assets/original/Data/Maps/Campaign/tutorial01.RMP'
doc={'schema':1,'date':'2026-10-16','native_contributor':'0x569C70','helpers':['0x4CED40','0x4CED60','0x58CB30','0x517FA0'],
     'original_sha256':base['original_sha256'],'v14_sha256':base['v14_sha256'],
     'base_native_executions':len(base['cases']),'actual_saved_native_executions':len(saved['cases']),
     'tests':{'pure':{'passed':2,'failed':0},'actual_saved_adapter':{'passed':1,'failed':0,'source_denizens':2},
              'actors_clippy':'pass with only two pre-existing named lint allowances: clippy::collapsible_if, clippy::doc_lazy_continuation',
              'runtime_clippy':'blocked by out-of-scope aa-rules/native_load.rs:878 manual_map lint'},
     'source_map':{'path':str(mapfile),'sha256':hashlib.sha256(mapfile.read_bytes()).hexdigest()},
     'observable_action_mask_order_proven':True,'raw_unchanged_high_word_store_order_claimed':False,
     'scope':'Action contributor and saved owned-storage adapter only; not world publication, execution, a battle or a release.',
     'gaps':['Present Wall payload remains explicitly unsupported','No Denizen refresh/tick or command execution','Malformed-storage exact native fault/read timing unclaimed','No release or original files changed'],
     'files':[pin(p) for p in files]}
(HERE/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'files':len(files),'findings_sha256':hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest()}))
