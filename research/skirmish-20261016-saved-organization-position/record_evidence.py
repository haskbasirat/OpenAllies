"""Pin bounded 57C800 saved-position evidence; original game files are read-only."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


fixture = json.loads((HERE / 'native-saved-position.json').read_text())
log = (HERE / 'replay-tests.log').read_text()
assert '2 passed; 0 failed' in log
cases = fixture['cases']
outcomes = {}
for row in cases:
    name = row['output']['outcome']
    outcomes[name] = outcomes.get(name, 0) + 1
paths = [
    ROOT / 'crates/aa-runtime/src/saved_organization_position.rs',
    ROOT / 'crates/aa-runtime/src/saved_organization_position_tests.rs',
    HERE / 'native_saved_position.py', HERE / 'actual-saved-inputs.json',
    HERE / 'native-saved-position.json', HERE / 'capture-tests.log',
    HERE / 'replay-tests.log', HERE / 'capture-selection-assumption.log',
    HERE / 'capture-mind-members-assumption.log', HERE / 'README.txt',
    Path(__file__),
]
out = {
    'schema': 1,
    'feature': 'saved Organization member-position notification 0057C800',
    'source_authority': 'original AA machine code and existing native-proved aa_actors::element_position core',
    'native_addresses': ['0057C800', '00578E90', '0057C680', '004515F0'],
    'original_sha256': fixture['original_sha256'],
    'v14_sha256': fixture['v14_sha256'],
    'precision_modes': ['Single24', 'Double53', 'Extended64'],
    'actual_saved_allocation_native_rows': len(cases),
    'outcomes': outcomes,
    'isolated_native_mind_presence_probes': len(fixture['isolated_native_probes']),
    'isolated_probe_result': 'all stop at required Mind virtual+64; no predicate answered',
    'rust_replay_tests': {'passed': 2, 'failed': 0, 'mode': 'release/locked/offline -j4 --include-ignored'},
    'actual_context': {'map': '(2v2)Kursk', 'organization_owners': 3, 'owner_mind_present': False,
                       'native_mode': 1, 'managed_fresh_scope': True, 'elapsed_millis_fixture': 0,
                       'no_immediate_Init_PostCreate_claim': True},
    'per_owner_probes': ['unchanged saved input', 'nonselected missing GTID', 'explicit mode6',
                         'explicit selected member', 'explicit dual flag30'],
    'storage': ['actual Organization +40', 'actual Organization goal +BC/+C0',
                'actual owner Thing flags and XY', 'generation-checked restored member Thing fields',
                'lazy real Mind slot/core validation at dispatch'],
    'preserved': ['Organization allocation identity', 'member vector', 'all unrelated Organization words',
                  'formation offsets', 'world RNG', 'native message sink', 'owner/member Thing positions'],
    'no_fabricated_predicate_or_service_success': True,
    'owned_source_frozen': True,
    'shared_wiring_owner': 'root; restore trait, OrganizationLeaderContext forwarding, Catalog',
    'limitations': [
        'Actual saved no-Mind paths only; current restore has no available Mind Organization owner with loaded members for direct actual-context comparison.',
        'Mind+64/+7C, relocation4EEB40, interpolation57C080, queue50B220 and schedule50AC00 remain required errors.',
        'Mind+7C/interpolation/scheduler are unexecuted through this adapter because the first required Mind+64 is unbound.',
        'Isolated native Mind probes are explicit input substitutions, not observed saved or initialized Rust Mind states.',
        'Selected-null notification is outside the bound saved-caller domain; mode6/nonselected guards still run first.',
        'Invalid registry/payload cases are portable refusal tests, not native malformed-pointer parity.',
        'Two failed fixture-selection assumptions retained separately; corrected actual contexts and final replay pass.',
        'No complete Init/PostCreate, live publication, battle, spatial movement or multiplayer completion.',
        'No vendor SDK implementation copied or integrated.',
    ],
    'inputs': [{'path': str(path.relative_to(ROOT)), 'sha256': sha(path)} for path in paths],
}
(HERE / 'findings.json').write_text(json.dumps(out, indent=2) + '\n')
print(json.dumps({'sha256': sha(HERE / 'findings.json'), 'native_rows': len(cases), 'outcomes': outcomes}))
