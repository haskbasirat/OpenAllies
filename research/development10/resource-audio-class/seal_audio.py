"""Seal bounded Audio-class evidence without rewriting native fixtures/logs."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
owned=[ROOT/'crates/aa-runtime/src'/n for n in ['resource_audio_class.rs','resource_audio_class_core.rs','resource_audio_class_tests.rs','resource_audio_manager_tests.rs']]
evidence=[HERE/n for n in ['native_audio_class.py','native-audio-class.json','native-generation-01.log','native-replay-01.log','audit_audio.py','pin-audit.json','pin-audit.log','exact-source-tests.rs','exact-source-tests.exe','exact-source-build-03.log','exact-source-tests-01.log','check-01.log','typecheck-tests-02.log']]
assert '2 passed; 0 failed' in (HERE/'exact-source-tests-01.log').read_text()
assert 'Finished `release`' in (HERE/'check-01.log').read_text()
assert 'paired_class_rows": 18' in (HERE/'native-replay-01.log').read_text()
assert json.loads((HERE/'pin-audit.json').read_text())['executed_instruction_count']==736
sources={str(p.relative_to(ROOT)):H(p)for p in owned}
shared={str(p.relative_to(ROOT)):H(p)for p in [ROOT/'crates/aa-runtime/src/resource_depot.rs',ROOT/'crates/aa-runtime/src/world_ready_services.rs',ROOT/'crates/aa-runtime/src/lib.rs']}
support=[ROOT/'target/native-runtime/release/deps'/n for n in ['libaa_runtime-4004dde273efdec2.rlib','libaa_rules-01028b5152c9e32e.rlib','libaa_numeric-55927aa222c0a1ba.rlib','libserde_json-3f0b3bb60c709793.rlib']]
doc=dict(schema=1,status='bounded-class/source accepted; current Manager integration test is saved, not yet executed',
 problem='Farm Audio source reached actual Sound Entry/Task publication then required59CB00 before streaming41/list append.',
 implementation='One Application-retained fresh no-profile source/PC scope is installed only after real source validation. Same Manager lazily owns one typed AudioClass after existing Sound publication. Valid source class creation precedes streaming41; absent policy or source refusal retains Entry/Task and does not mark anything loaded. No raw class ordinal constructor.',
 native=dict(paired_class_rows=18,class_calls=72,descriptor_table_runs=6,executed_instruction_pins=736,precision=['Single24','Double53','Extended64'],images=['original','v14'],
 transport='ASCII-to-counted CString, CRT name formatting, current Int lookup and atexit/heap/Win32 support are explicit. Empty valid TypeManager assigns relative ordinal0; not observed full startup registry. Positive variable lookup is called twice by native; fresh immutable source scope excludes intervening current mutations. Cache-hit second59CB00 performs zero variable lookups. Actual Sound bytes remain unchanged.'),
 rust=dict(exact_current_module_tests_passed=2,actual_source='ResourceCacheAudioRAMWeight unique Int10 in UVars.tgi; RAM10/VRAM10; same completed Bank moved from Catalog; empty Bank and unbound/incomplete source contexts refuse.',
 dependency_qualification='Exact current Audio source/class module and original tests are imported directly. Frozen existing runtime rlib supplies Catalog/source-CV support only; this is not a rebuild of the whole current ResourceManager.',
 production_check='cargo check -p aa-runtime --lib release/locked/offline own target PASS19.90s before the later shared dispatch initializer appeared; that initializer now loans the same cache_variables.',
 current_cfg_test_check='All Audio/Manager tests typechecked; full run blocked by foreign data_actor_request_tests.rs29 E0507 move out ofArc. No Cargo link or manager runtime-test success claim.',
 pending_manager_filter='actual_sound_class_is_lazy_after_entry_and_reuses_same_manager_allocation --include-ignored'),
 deviation='User policy removes fixed RAM/VRAM eviction budgets. Growable manager retains typed class, current weights and resource lifetimes. The native zero-budget laboratory fixture is not a native startup-budget observation; no full eviction/accounting parity claim.',
 limits=['No Sound decoding, task completion, audio playback, mission activation, VWorld readiness or playable-battle certificate.','Profile/console/reloaded variable writers need a genuine current owner; they cannot enter the admitted fresh source policy.','Zero/negative RAM weights and native throw/OOM prefixes are explicitly outside the proved positive constructor domain.'],
 owned_source_sha256=sources,shared_whole_file_snapshot_only_sha256=shared,evidence_sha256={str(p.relative_to(ROOT)):H(p)for p in evidence},compiled_support_sha256={str(p.relative_to(ROOT)):H(p)for p in support})
path=HERE/'findings.json';path.write_text(json.dumps(doc,indent=1)+'\n');print(json.dumps(dict(findings_sha256=H(path),owned_source=sources)))
