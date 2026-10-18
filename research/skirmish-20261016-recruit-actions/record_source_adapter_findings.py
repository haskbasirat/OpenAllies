"""Pin completed bounded source and saved-restore adapter evidence."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

native = json.loads((HERE / 'native-recruit-source.json').read_text())
census = json.loads((HERE / 'source-census.json').read_text())
assert '2 passed; 0 failed' in (HERE / 'source-tests.log').read_text()
assert '1 passed; 0 failed' in (HERE / 'adapter-tests.log').read_text()
source_paths = [
    'crates/aa-rules/src/native_recruit_data.rs',
    'crates/aa-rules/tests/native_recruit_data.rs',
    'crates/aa-runtime/src/saved_recruit_actions.rs',
]
evidence_paths = [
    'native_recruit_source.py', 'native-recruit-source.json', 'source-census.json',
    'source-oracle.log', 'source-tests.log', 'adapter-tests.log',
]
prior_paths = [
    'crates/aa-actors/src/recruit_state.rs',
    'research/phase8-restoration/native-recruit.json',
]
result = {
    'schema': 1,
    'date': '2026-10-16',
    'scope': 'Bounded completed Recruit source facts and saved-restore 582C40 adapter; eligibility 5827B0 remains explicit',
    'original_sha256': native['original_sha256'],
    'v14_sha256': native['v14_sha256'],
    'apis': [
        'aa_rules::native::recruit_data::actor_recruit_targets(catalog, canonical_actor_id)',
        'aa_rules::native::recruit_data::actor_transport_class(catalog, canonical_actor_id)',
        'aa_rules::native::recruit_data::actor_state_type(catalog, compiled_state_name, precision)',
        'aa_runtime::saved_recruit_actions::contribute(&mut SavedActions, ActorId) [crate-private]',
    ],
    'native_addresses': {
        'recruit_producer': '467C89..467DB6 in464BD0',
        'deferred_registration': '473AD0',
        'consolidation': '475230 ->461810 ->460310',
        'actor_lookup_seam': '4BDBA0 category2B',
        'transport_class': '4608C0 ->ActorData+184',
        'state_enum': '9CC560 ->48A630(B3AD80,1)',
        'state_type_store': '47DCB3..47DD32 in47DC50 ->ActorState+28',
        'recruit_constructor': '581E40 ->byte10/11 zero',
        'recruit_saved_load': '583C80 leavesbyte10/11 unchanged',
    },
    'cases': len(native['cases']),
    'case_distribution_per_precision': {'targets': 7, 'transport': 4, 'state': 4},
    'precisions': ['Single24', 'Double53', 'Extended64'],
    'native_windows_count': len(native['native_windows']),
    'template': {key: native['template'][key] for key in ['path', 'sha256']},
    'real_source_coverage': {
        'catalog_scan': 'Every standard ActorData in the actual native-order V14Sources catalog',
        'nonempty_lists': len(census['lists']),
        'transport_linked_actors': census['transport_linked_actors'],
        'state_definitions_all_three_precisions': len(census['states']),
        'source_files_with_hashes': len(census['sources']),
    },
    'saved_adapter_probe': {
        'source': 'Actual tutorial02 partial restored records, real nonempty Recruit target list',
        'explicit_probe_inputs': 'Normal state bit27 and known initial action bits; second case sets Recruit byte11=1',
        'zero_byte_path': 'Actual saved byte10/11=0, target list presence checked, final action bits correct',
        'enabled_path': 'Explicit5827B0 dependency error after committed bit14 prefix',
        'not_claimed': 'Original full load completion or live recruitment execution',
    },
    'source_sha256': {p: sha(ROOT / p) for p in source_paths},
    'evidence_sha256': {p: sha(HERE / p) for p in evidence_paths},
    'reused_evidence_sha256': {p: sha(ROOT / p) for p in prior_paths},
    'backup_sha256': {p.name: sha(p) for p in sorted((HERE / 'backups').iterdir())},
    'test_results': {'source': {'passed': 2, 'failed': 0}, 'saved_adapter': {'passed': 1, 'failed': 0}},
    'semantics': [
        'Canonical definitions shadowed by translations fail rather than re-resolving identity',
        'Deferred targets validate in reverse callback order; successful list retains authored order and duplicates',
        'Failed source load returns failure; no claim to preserve partial source registry after load failure',
        'Nonempty target lists never become synthetic absent-head success',
        'Saved eligibility bytes are read from real Recruit payload, never synthesized for missing data',
    ],
    'limits_changed': [],
    'explicit_seams': [
        'Source oracle4BDBA0 target registry uses explicit typed target fixture objects',
        'Diagnostic formatting405D60 and atexit infrastructure seams',
        'Organization recursion/backreference branches excluded from simple target fixture masks',
    ],
    'remaining': [
        'Concrete5827B0 eligibility/requirements service',
        'Actual Settlement+14 payload if present',
        'Live source mutation and actor update execution',
        'Broader rules/metagame compatibility coordinated by root',
        'Full skirmish and multiplayer',
    ],
}
(HERE / 'source-adapter-findings.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({
    'cases': result['cases'],
    'tests': result['test_results'],
    'source_sha256': result['source_sha256'],
    'fixture_sha256': result['evidence_sha256']['native-recruit-source.json'],
    'census_sha256': result['evidence_sha256']['source-census.json'],
}, indent=2))
