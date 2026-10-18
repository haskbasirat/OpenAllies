from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
binary=ROOT/'qa-output/development10/runtime-tests55.exe'
assert sha(binary)=='12817c597f438558350e5aee646cde55fd0cb7b393d25490ab6e1dbc725a4f41'
assert '1 passed; 0 failed' in(HERE/'independent-actual-runtime55.log').read_text()
capture=json.loads((HERE/'actual-source-input.json').read_text());native=json.loads((HERE/'native-actual-stored-creation-fx.json').read_text())
assert len(capture)==len(native['rows'])==3 and len(native['instructions'])==300
assert native['capture_sha256']==sha(HERE/'actual-source-input.json')
old=json.loads((HERE/'independent-source-review.json').read_text())
source_match={name:sha(ROOT/name)==expected for name,expected in old['pins'].items() if name in ['crates/aa-runtime/src/stored_creation_fx.rs','crates/aa-runtime/src/stored_creation_fx_tests.rs']}
paths=['actual-source-input.json','native-actual-stored-creation-fx.json','native_actual_stored_creation_fx.py','native_existing_fx_null_view.py','basis-native-existing-fx-command.py','native-actual-write.log','native-actual-replay.log','independent-actual-runtime55.log','independent-source-review.json','seal_actual_review.py']
doc={
 'schema':1,'status':'accepted within actual source/ActorStore/Input/event-constructor/pending-manager laboratory scope',
 'predecessor':'independent-source-review.json retained unchanged; source/common native acceptance remains qualified there',
 'new_native':{'actual_captured_contexts':3,'images':['original','v14'],'precisions':['Single24','Double53','Extended64'],'executions_each_write_and_immutable_pass':6,'executed_address_byte_pins':300,'method':'Native42A590/429080/428480 computes expected command from captured inputs; both Rust commands are compared against it before fixture write, rather than copied into expected'},
 'new_reviewer_runtime':{'test':'stored_creation_fx::tests::actual_fresh_actor_inputs_keep_same_graph_clock_and_pending_commands','result':'1/1 pass4.72sec','capture_environment':'AA_STORED_CREATION_FX_CAPTURE unset','binary':str(binary.relative_to(ROOT)).replace('\\','/'),'sha256':sha(binary),'new_compile_or_link':False},
 'same_owner_checks':['real ger_corps canonical source and completed moved Bank','actual SourceNew ActorStore allocation/constructor/identity init, no LoadShell copy','full20-node View Graph and17 Constructed roots, empty Created lazy return','two actual same-Graph pending commands in SAME manager with byte58=0','actual staged pose/NativeClock begin and request read order','nonnull actual Thing+C and stale E8 refuse with existing pending prefix retained'],
 'scope_limits':['counting ExecutionIds41/42 are explicitly controlled fixtures, not actual OS worker acceptance','no initialized Staged World or natural mission acceptance','positive actual Kingdom creator branch not tested (null and stale E8 only)','no positive View attachment child or effect update/render/child execution claim','current shared Graph successors remain outside predecessor source review'],
 'predecessor_source_pins_still_match':source_match,
 'artifacts':[dict(path=n,sha256=sha(HERE/n))for n in paths],
}
path=HERE/'independent-actual-execution-review.json'
assert not path.exists();path.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(review_sha256=sha(path),source_pin_matches=source_match)))
