from pathlib import Path
import json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import ORIGINAL,ACCEPTED,sha

review=json.loads((HERE/'independent-review-native.json').read_text())
author=json.loads((HERE/'findings.json').read_text())
for item in author['files']:
    assert sha((ROOT/item['path']).read_bytes())==item['sha256'],item['path']
for name,pin in review['author_fixture_sha256'].items():
    assert sha((HERE/name).read_bytes())==pin,name
assert '2 passed; 0 failed' in (HERE/'independent-actors-test.log').read_text()
assert '1 passed; 0 failed' in (HERE/'independent-runtime-test.log').read_text()
ctor=ORIGINAL.at(0x4d6dbd,6)
assert ctor==bytes.fromhex('c706f073a200')==ACCEPTED.at(0x4d6dbd,6)
virtual=ORIGINAL.at(0xa274f0,4)
assert struct.unpack('<I',virtual)[0]==0x4ced60 and virtual==ACCEPTED.at(0xa274f0,4)
files=['crates/aa-actors/src/denizen_actions.rs','crates/aa-actors/tests/native_denizen_actions.rs','crates/aa-runtime/src/saved_denizen_actions.rs']
evidence=['independent_review.py','independent-review-native.json','independent-native-replay.log','independent-instructions.txt','independent-actors-test.log','independent-runtime-test.log']
report={
    'schema':1,'date':'2026-10-16','reviewer':'organization_morale_source','independent_of_denizen_author':True,
    'decision':'No blocking findings within the documented action-contributor/saved-binding scope',
    'blocking_findings':[],
    'reviewed_source_sha256':{p:sha((ROOT/p).read_bytes()) for p in files},
    'author_findings_sha256':sha((HERE/'findings.json').read_bytes()),
    'evidence_sha256':{p:sha((HERE/p).read_bytes()) for p in evidence},
    'native_replay':{
        'author_base_rows':review['replayed_cases']['native-denizen-actions.json'],
        'author_actual_saved_rows':review['replayed_cases']['native-saved-denizen-actions.json'],
        'additional_rows':len(review['extra_cases']),
        'raw_store_probes':len(review['raw_store_probes']),
        'images':['original','v14'],'precisions':['Single24','Double53','Extended64'],
        'method':'Inspected author memory setup; no gameplay seams; independently re-executed and derived expected instruction predicate; did not regenerate author fixtures',
    },
    'static_vtable_proof':{'saved_ctor_instruction':'4D6DBD mov [esi],A273F0','bytes':ctor.hex(),'actor_virtual_100_address':'A274F0','target':'4CED60','original_v14_equal':True},
    'rust_replays':{'pure_passed':2,'saved_adapter_passed':1,'failed':0,'target':'target/s-research-actions','scope':'Only native_denizen_actions and saved_denizen_actions::tests; no broad suite'},
    'verified':[
        'State30 clears action23,24,25,26,27,28 in that observable order and bypasses parent/flag reads',
        '4CED40 returns self on Settlement presence; otherwise actual GActor+100=4CED60 prefers nonnull Structure+10, then Wall+10; nonnull Wall getter is reread',
        'Absent parent clears23/24; present parent with absent capability preserves23/24',
        'Flags38 bit7 gates27/28; missing capability preserves those bits',
        'Byte20/21 zero/nonzero truth is correct for every u8 value, including128..255',
        'Action25/26 always written using conjunction of flags38 andflags3C bit4/6',
        'Unrelated low/high action bits retained; missing later fields preserve completed action prefix',
        'Saved binding uses actual Denizen header/bytes/words and Structure object_handle_10; it does not guess Kingdom/parent ownership',
        'Present Wall stays explicit unsupported; actual tutorial01 comparison preserves Denizen allocation and asserts true header owner identity',
    ],
    'accepted_scope_qualifications':[
        'Original state30 raw high-word self-stores sometimes precede low stores. Rust uses combined64-bit action masks; all high self-stores are unchanged and no callback intervenes. Observable mask ordering is proven; exact raw redundant store order is not claimed',
        'ActorFields reads more complete stored state than the native leaf at some points; exact malformed-storage read/fault timing is not claimed',
        'Actual tutorial01 test uses explicitly unused Organization context fixture; Denizen never queries it. It proves a leaf over actual owned Denizen data, not a full attach/publication sequence',
        'No Denizen refresh/command execution, battle, multiplayer or new release is demonstrated',
    ],
    'files_changed_by_review':evidence+['record_independent_review.py','independent-review.json','independent-review.txt'],
    'production_or_author_fixture_changes':False,
}
(HERE/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'decision':report['decision'],'native_replay':report['native_replay'],'rust_replays':report['rust_replays'],'report_sha256':sha((HERE/'independent-review.json').read_bytes())},indent=2))
