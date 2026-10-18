from pathlib import Path
import hashlib, json
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
native=json.loads((HERE/'native-actor-commands.json').read_text())
census=json.loads((HERE/'source-census.json').read_text())
log=(HERE/'tests.log').read_text()
assert '4 passed; 0 failed; 0 ignored' in log
paths=['crates/aa-rules/src/native_actor_command.rs','crates/aa-rules/tests/native_actor_command.rs']
report={
 'schema':1,'date':'2026-10-16',
 'scope':'Code-predefined ActorCommandData type/component/behavior fields; not saved command loading, execution, aliases, or full UI loader',
 'api':'aa_rules::native::actor_command::predefined_actor_command(&str) -> native::Result<ActorCommandDefinition>',
 'struct_fields':['id','mind_type','component_slot','behavior_flags'],
 'original_sha256':native['original_sha256'],'v14_sha256':native['v14_sha256'],
 'addresses':{'crt_initializer':'0x9cd7e0','enum_constructor':'0x4da9d0','enum_registration':'0x4da2f0','enum_global':'0xb3cba0',
              'manager_constructor':'0x47f7c0','registry_global':'0xb3aa58','manager_offset':'GameData+0x274',
              'definition_registration':'0x475870','definition_constructor':'0x4740f0','register':'0x4b4e60',
              'saved_lookup':'0x463fa0','saved_reader':'0x4dab60','tgi_flag_fragment':['0x48cf0a','0x48cf97']},
 'counts':{'builtins':39,'registered_records':sum(len(r['definitions']) for r in native['runs']),
           'native_lookups':sum(len(r['lookups']) for r in native['runs']),
           'authored_flag_loads':sum(len(r['authored_flag_loads']) for r in native['runs']),
           'input_files':len(native['source_files']),'authored_v14_ids':len(census['authored_ids']),
           'tests_passed':4,'tests_failed':0,'tests_ignored':0},
 'precisions':['Single24','Double53','Extended64'],
 'source_sha256':{p:sha(ROOT/p) for p in paths},
 'evidence_sha256':{p:sha(HERE/p) for p in ['native_actor_commands.py','native-actor-commands.json','source-census.json','tests.log']},
 'original_data_inputs':native['source_files'],
 'identity_policy':'Built-in ASCII case-insensitive names, no trimming or second alias translation; unknown and empty return explicit error',
 'limits_changed':[],
 'seams':['Existing allocation/CString/mutex/line-source harness; guarded null page and C++ throw observation. No command definition values supplied by seams.',
          'CRT atexit96CAB0 records destructor9ED2F0 without executing shutdown.',
          'TGI evidence is the exact UI flag fragment, excluding localization/resource/cursor/VFX loading and cleanup.'],
 'remaining':['Saved command decoding/interning and runtime binding','Command subtype execution','Source Translate resolution owned by caller','Full ActorCommand UI loader']}
(HERE/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report['counts']))
