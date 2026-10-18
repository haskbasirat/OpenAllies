"""Pin the current bounded creation port and explicitly separate missing systems."""
from pathlib import Path
import hashlib, json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
H = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

creation_path = HERE / 'creation-native-oracle.json'
creation = json.loads(creation_path.read_text())
prerequisite_path = HERE / 'definition-prerequisites.json'
prerequisite = json.loads(prerequisite_path.read_text())
assert H(HERE / 'original_creation.py') == creation['producer_sha256']
assert H(ROOT / 'research/phase4-actors/trace_registry.py') == creation['registry_machine_sha256']
assert H(HERE / 'direction-tables.bin') == creation['direction_table_sha256']
assert H(HERE / 'definition_prerequisites.py') == prerequisite['producer_sha256']
assert len(creation['cases']) == 282
assert len(prerequisite['getter_cases']) == 441
assert all(len(t['values']) == 2048 for t in creation['tables'])

out = {
    'schema': 1,
    'scope': 'Bounded base GThing construction and valid-record identity initialization; not a complete actor factory.',
    'original_sha256': creation['original_sha256'],
    'v14_sha256': creation['v14_sha256'],
    'implemented': {
        'constructor': '4EF2A0 flags, raw position/name/time, facing branch order, record mutation, RNG draw and normalization',
        'identity': '4EE460 ordinary allocation or saved generation plus registration through existing verified registry',
        'fixtures': 282,
        'direction_vectors': 6144,
        'single_vs_double_startup_vectors_differing': sum(a != b for a, b in zip(creation['tables'][0]['values'], creation['tables'][1]['values'])),
        'double_vs_extended_startup_vectors_differing': sum(a != b for a, b in zip(creation['tables'][1]['values'], creation['tables'][2]['values'])),
        'crate_tests_observed_passed': 9,
        'focused_creation_tests_observed_passed': 3,
        'strict_clippy_observed_passed': True,
    },
    'evidence_only_not_implemented': {
        'definition_mask': '4624B0 initializes +174;464BD0 typed child-node/value branches set or clear component bits. Static instruction inventory only.',
        'derived_getters': '441 original x86 speed/health/defense getter cases with null or explicit resolved modifier context; final diagnostic f32 store.',
    },
    'missing': ['typed merged/inherited actor definition evaluation', 'full GObject/GActor constructors',
                'spatial placement/world grid', 'ownership/required and nested actors', 'component constructor factories',
                'ability and state setup', 'world publication and failure destruction', 'original save record decoding'],
    'source_sha256': {str(p.relative_to(ROOT)).replace('\\', '/'): H(p) for p in [
        ROOT / 'crates/aa-actors/src/creation.rs', ROOT / 'crates/aa-actors/tests/native_creation.rs',
        HERE / 'original_creation.py', creation_path, HERE / 'direction-tables.bin',
        HERE / 'definition_prerequisites.py', prerequisite_path, HERE / 'definition-mask-instructions.txt',
        HERE / 'client_samples.rs', HERE / 'client-samples.json', HERE / 'README.txt']},
}
p = HERE / 'findings.json'
p.write_text(json.dumps(out, indent=2)+'\n')
print(json.dumps({'findings_sha256': H(p), 'constructor_cases': 282, 'getter_evidence_cases': 441}))
