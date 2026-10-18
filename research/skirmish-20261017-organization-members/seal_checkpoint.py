"""Seal the bounded Organization adapter checkpoint; no oracle generation."""
from pathlib import Path
import hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
sources=[
 'crates/aa-runtime/src/stored_organization_members.rs',
 'crates/aa-runtime/src/stored_organization_member_mind.rs',
 'crates/aa-runtime/src/organization_mind_callbacks.rs',
 'crates/aa-runtime/src/organization_mind_callbacks_tests.rs',
 'crates/aa-runtime/src/saved_member_properties.rs',
 'crates/aa-runtime/src/saved_member_properties_tests.rs',
 'crates/aa-actors/src/organization_add_member.rs',
 'crates/aa-actors/src/organization_state.rs',
]
snapshot=HERE/'source-v1';snapshot.mkdir(exist_ok=True)
for name in sources:
 target=snapshot/Path(name).name
 if target.exists():assert target.read_bytes()==(ROOT/name).read_bytes(),'checkpoint already sealed with different bytes'
 else:shutil.copyfile(ROOT/name,target)
files=sources+[
 str(p.relative_to(ROOT)) for p in HERE.iterdir() if p.is_file()
 and p.name not in {'freeze.json','findings.json'} and p.suffix not in {'.pdb'}
]+[
 'research/skirmish-20261016-member-properties/native_member_properties.py',
 'research/skirmish-20261016-member-properties/native-member-properties.json',
 'research/skirmish-20261016-member-properties/actual-saved-inputs.json',
 'research/phase7-mind/native-constructors.json',
 'research/skirmish-20261017-recruit-eligibility/organization-factor-tests01.log',
]
doc={
 'schema':1,'scope':'57CBA0 same installed Organization adapter; bounded native Mind leaves; shared5784C0 kernel',
 'status':'source and bounded proofs frozen; whole adapter compiled only, no live add-member execution',
 'native_new':{'rows':360,'original_v14_executions':720,
   'per_fixture_instruction_pins':120,'executed_pins':116,'required_entry_pins':4,
   'deduplicated_across_fixtures':False},
 'native_reused_5784c0':{'executions':294,'address_byte_pins':118,
   'scope':'historical callback/lookup/actual-saved fixture, freshly immutable-replayed; no new stored owner native run'},
 'rust':[{'scope':'exact current Mind module source','passed':3,'log':'exact-mind-replay03.log'},
   {'scope':'exact unchanged5784C0/lookup source and two original tests; import/path extraction only','passed':2,'log':'member-kernel-replay01.log'},
   {'scope':'old aa-actors Organization native fixture regressions after AddMemberFields factoring','passed':3,
    'log':'../skirmish-20261017-recruit-eligibility/organization-factor-tests01.log'}],
 'typecheck':{'command':'cargo check -p aa-runtime --tests --release --locked --offline -j4',
   'log':'tests-source-properties07.log','result':'PASS','elapsed':'1m14s','whole_runtime_link':False},
 'ownership':['actual Organization Box stays installed; each short borrow ends before child callback',
   'same ActorStore/moved Bank/Arena/World variables/queues/clock/RNG and moved managed lifecycle',
   'captured Properties Box identity is checked; actual current slots/count/instances reread',
   'support4CEF10 target is Organization owner; source is member, as57CCE2..CCE9'],
 'not_proven':['whole live57CBA0 adapter or fresh full Regiment spawn',
   'native allocator/Win32/unwind/malformed-pointer equivalence',
   'new source-independent defaults or always-successful child callbacks'],
 'required_children':['active Mind move virtual24 speed callback',
   'nonzero Performer rank definition/index','522890 actual zone/command child and later arrival allocation',
   'nonempty nested Regiment member3C/40 propagation','healthy home target virtual174 relation',
   'positive relocation/interpolation/scheduler publication and detached diagnostic',
   'positive Zone4 notification; any later real Property child unavailable in shared admission/insertion'],
 'source_snapshots':{str(p.relative_to(ROOT)):sha(p) for p in snapshot.iterdir()},
 'files':{name:sha(ROOT/name) for name in sorted(set(files))},
 'shared_file_note':'organization_state, saved_member_properties and lib exports may acquire independently authorized additive successors; snapshots identify this historical interval only',
}
(HERE/'freeze.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'freeze_sha256':sha(HERE/'freeze.json'),'files':len(doc['files'])}))
