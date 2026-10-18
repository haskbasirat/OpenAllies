"""Seal bounded saved-state evidence without modifying source or native assets."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


fixture = json.loads((HERE / 'native-saved-actor-state.json').read_text())
cases = fixture['cases']
log = (HERE / 'state-tests.log').read_text()
assert '2 passed; 0 failed' in log
boundaries = {}
for row in cases:
    key = row['output']['first_required_effect']
    if key:
        boundaries[key] = boundaries.get(key, 0) + 1
inputs = [
    ROOT / 'crates/aa-runtime/src/saved_actor_state.rs',
    ROOT / 'crates/aa-runtime/src/saved_actor_state_tests.rs',
    HERE / 'native_saved_actor_state.py',
    HERE / 'native-saved-actor-state.json',
    HERE / 'state-tests.log',
    HERE / 'context-not-yet-landed.log',
    HERE / 'native-replay.log',
    HERE / 'README.txt',
    Path(__file__),
]
out = {
    'schema': 1,
    'feature': 'saved Actor 004D64F0 coordinator on the actual shell/Thing flags',
    'source_authority': 'original AA machine code and the previous native-proved aa_actors::state::set',
    'original_sha256': fixture['original_sha256'],
    'v14_sha256': fixture['v14_sha256'],
    'precision_modes': ['Single24', 'Double53', 'Extended64'],
    'native_rows': len(cases),
    'native_completed_rows': sum(row['output']['completed'] for row in cases),
    'native_unchanged_state_rows_catalog_poisoned': sum(row['input']['poison_catalog'] for row in cases),
    'required_effect_boundary_rows': boundaries,
    'organization_state34_or37_rows': sum(row['input']['index'] in [34, 37] for row in cases),
    'rust_tests': {'passed': 2, 'failed': 0, 'mode': 'release/locked/offline -j4'},
    'context_race_compile': 'First attempt occurred before root shared contexts/traits were saved; retained separately; later focused build passed.',
    'callback_services_returning_fake_success': 0,
    'callbacks_reached_are_executed': False,
    'source_changes': [str(path.relative_to(ROOT)) for path in inputs[:2]],
    'shared_integration_owner': 'root; ActorStateContext, restore hook, Catalog forwarding, and actual Thing flag plumbing',
    'preserved_on_error': ['native prefix state bits', 'Thing flags', 'unrelated Actor words', 'component payloads'],
    'limitations': [
        '004D4A90 enter, 004D16B0 leave, and exclusive 004D1350 rebuild explicitly error at their required boundary.',
        'Native descriptors/masks supplied from earlier complete 00477FC0 registration oracle; registration transport not rerun.',
        'Concrete tutorial02 input comes from an existing Rust partial restore; it is not a complete native map-load proof.',
        'The later missing_primary=false request is a focused input case, not proof that preceding callbacks completed.',
        'Invalid-index refusal is a portable safety domain, not malformed native table parity.',
        'No immediate Init/PostCreate, live publication, battle startup, or multiplayer completion is claimed.',
        'No vendor SDK implementation copied or integrated.',
    ],
    'inputs': [{'path': str(path.relative_to(ROOT)), 'sha256': sha(path)} for path in inputs],
}
(HERE / 'findings.json').write_text(json.dumps(out, indent=2) + '\n')
print(json.dumps({'sha256': sha(HERE / 'findings.json'), 'cases': len(cases),
                  'completed': out['native_completed_rows'], 'boundaries': boundaries}))
